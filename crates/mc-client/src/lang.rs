//! `StringTranslate` (StringTranslate.java): the translation table, loaded
//! from the jar's `/lang/en_US.lang` then `/lang/stats_US.lang` (D8: the
//! user's jar, read at runtime) with `java.util.Properties.load` semantics
//! ([`parse_properties`]).
//!
//! Vanilla holds it in a static singleton (`getInstance()`); the client
//! keeps one in [`crate::assets::Lang`].

use std::collections::BTreeMap;

/// The first language file (StringTranslate.java:10).
pub const EN_US_LANG: &str = "/lang/en_US.lang";
/// The second, loaded over the first (StringTranslate.java:11).
pub const STATS_US_LANG: &str = "/lang/stats_US.lang";

/// `StringTranslate`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StringTranslate {
    /// `translateTable`.
    // src: StringTranslate.java:6
    pub translate_table: BTreeMap<String, String>,
}

impl StringTranslate {
    /// The private constructor's two `Properties.load` calls, on the bytes
    /// of `/lang/en_US.lang` and `/lang/stats_US.lang`.
    // src: StringTranslate.java:8-16
    pub fn parse(en_us: &[u8], stats_us: &[u8]) -> Self {
        let mut t = StringTranslate::default();
        parse_properties(en_us, &mut t.translate_table);
        parse_properties(stats_us, &mut t.translate_table);
        t
    }

    /// `translateKey(key)`: the value, or the key itself.
    // src: StringTranslate.java:22-24
    pub fn translate_key(&self, var1: &str) -> String {
        self.translate_table
            .get(var1)
            .cloned()
            .unwrap_or_else(|| var1.to_string())
    }

    /// `translateKeyFormat(key, args)`: `String.format` of the value (or the
    /// key) with `args`, for the specifiers the lang files use: `%s`,
    /// `%d`, `%n$s`, `%n$d` and `%%`.
    // src: StringTranslate.java:26-29
    pub fn translate_key_format(&self, var1: &str, var2: &[String]) -> String {
        java_format(&self.translate_key(var1), var2)
    }

    /// `translateNamedKey(key)`: the value of `key + ".name"`, or `""`.
    // src: StringTranslate.java:31-33
    pub fn translate_named_key(&self, var1: &str) -> String {
        self.translate_table
            .get(&format!("{var1}.name"))
            .cloned()
            .unwrap_or_default()
    }
}

/// `String.format(fmt, args)` for `%s`, `%d`, `%n$s`, `%n$d`, `%%`; any
/// other specifier is copied through.
pub fn java_format(fmt: &str, args: &[String]) -> String {
    let mut out = String::new();
    let mut next = 0usize;
    let mut it = fmt.char_indices().peekable();
    while let Some((i, c)) = it.next() {
        if c != '%' {
            out.push(c);
            continue;
        }
        let rest = &fmt[i + 1..];
        if rest.starts_with('%') {
            out.push('%');
            it.next();
            continue;
        }
        // %n$s / %n$d
        let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
        if !digits.is_empty() {
            let after = &rest[digits.len()..];
            if let Some(conv) = after.strip_prefix('$').and_then(|a| a.chars().next())
                && (conv == 's' || conv == 'd')
            {
                let n: usize = digits.parse().unwrap_or(0);
                out.push_str(args.get(n.wrapping_sub(1)).map_or("null", |s| s.as_str()));
                for _ in 0..digits.len() + 2 {
                    it.next();
                }
                continue;
            }
        }
        match rest.chars().next() {
            Some('s') | Some('d') => {
                out.push_str(args.get(next).map_or("null", |s| s.as_str()));
                next += 1;
                it.next();
            }
            _ => out.push('%'),
        }
    }
    out
}

/// `Properties.load(InputStream)` into `table`: ISO-8859-1 bytes; logical
/// lines joined across a trailing odd number of backslashes (leading
/// whitespace of the continuation dropped); blank lines and lines whose
/// first non-blank character is `#` or `!` skipped; the key ends at the first
/// unescaped `=`, `:` or whitespace, then whitespace and one `=`/`:` are
/// skipped; `\t \n \r \f \uXXXX` and `\x` → `x` escapes in key and value.
/// Later keys replace earlier ones.
pub fn parse_properties(bytes: &[u8], table: &mut BTreeMap<String, String>) {
    // ISO-8859-1: each byte is one char.
    let text: Vec<char> = bytes.iter().map(|&b| b as char).collect();
    let mut i = 0;
    let n = text.len();
    let is_ws = |c: char| c == ' ' || c == '\t' || c == '\x0c';
    while i < n {
        // One natural line, then continuation lines: collect the logical line.
        let mut line: Vec<char> = Vec::new();
        let mut first = true;
        loop {
            // skip leading whitespace
            while i < n && is_ws(text[i]) {
                i += 1;
            }
            let start = i;
            while i < n && text[i] != '\n' && text[i] != '\r' {
                i += 1;
            }
            let natural = &text[start..i];
            // consume the line end
            if i < n && text[i] == '\r' {
                i += 1;
                if i < n && text[i] == '\n' {
                    i += 1;
                }
            } else if i < n && text[i] == '\n' {
                i += 1;
            }
            if first && (natural.is_empty() || natural[0] == '#' || natural[0] == '!') {
                // blank or comment line (comments are never continued)
                if i >= n {
                    break;
                }
                continue;
            }
            first = false;
            let trailing = natural.iter().rev().take_while(|&&c| c == '\\').count();
            if trailing % 2 == 1 {
                line.extend_from_slice(&natural[..natural.len() - 1]);
                if i >= n {
                    break;
                }
                continue;
            }
            line.extend_from_slice(natural);
            break;
        }
        if first {
            // only blank/comment lines up to the end
            continue;
        }
        // Split key / value.
        let mut k = 0;
        let mut has_sep = false;
        let mut backslash = false;
        while k < line.len() {
            let c = line[k];
            if backslash {
                backslash = false;
            } else if c == '\\' {
                backslash = true;
            } else if c == '=' || c == ':' {
                has_sep = true;
                break;
            } else if is_ws(c) {
                break;
            }
            k += 1;
        }
        let key_end = k;
        let mut v = k;
        if has_sep {
            v += 1;
        } else {
            while v < line.len() && is_ws(line[v]) {
                v += 1;
            }
            if v < line.len() && (line[v] == '=' || line[v] == ':') {
                v += 1;
            }
        }
        while v < line.len() && is_ws(line[v]) {
            v += 1;
        }
        let key = unescape(&line[..key_end]);
        let value = unescape(&line[v..]);
        table.insert(key, value);
    }
}

/// `Properties.loadConvert`: the escapes of a key or value.
fn unescape(s: &[char]) -> String {
    let mut out = String::new();
    let mut i = 0;
    while i < s.len() {
        let c = s[i];
        i += 1;
        if c != '\\' {
            out.push(c);
            continue;
        }
        let Some(&e) = s.get(i) else { break };
        i += 1;
        match e {
            'u' => {
                let hex: String = s[i..s.len().min(i + 4)].iter().collect();
                // Java throws IllegalArgumentException on a malformed \u;
                // the lang files have none. Keep the text as is then.
                match u32::from_str_radix(&hex, 16).ok().and_then(char::from_u32) {
                    Some(ch) if hex.len() == 4 => {
                        out.push(ch);
                        i += 4;
                    }
                    _ => {
                        out.push('u');
                    }
                }
            }
            't' => out.push('\t'),
            'r' => out.push('\r'),
            'n' => out.push('\n'),
            'f' => out.push('\x0c'),
            other => out.push(other),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn props(s: &str) -> BTreeMap<String, String> {
        let mut t = BTreeMap::new();
        parse_properties(s.as_bytes(), &mut t);
        t
    }

    #[test]
    fn properties_basics() {
        let t = props(
            "gui.done=Done\n\n# comment\n! also\n  menu.quit = Quit Game\nkey:value\nspaced key\n\
cont=a\\\n   b\nesc=\\u0041\\t\\\\x\nempty=\ncrlf=1\r\nlast=x",
        );
        assert_eq!(t["gui.done"], "Done");
        assert_eq!(t["menu.quit"], "Quit Game");
        assert_eq!(t["key"], "value");
        assert_eq!(t["spaced"], "key");
        assert_eq!(t["cont"], "ab");
        assert_eq!(t["esc"], "A\t\\x");
        assert_eq!(t["empty"], "");
        assert_eq!(t["crlf"], "1");
        assert_eq!(t["last"], "x");
        assert!(!t.contains_key("# comment"));
    }

    #[test]
    fn later_files_override() {
        let t = StringTranslate::parse(b"a=1\nb=2\n", b"b=3\n");
        assert_eq!(t.translate_key("a"), "1");
        assert_eq!(t.translate_key("b"), "3");
        assert_eq!(t.translate_key("missing"), "missing");
        assert_eq!(t.translate_named_key("x"), "");
    }

    #[test]
    fn latin1_bytes() {
        let t = StringTranslate::parse(b"\xef\xbb\xbf\na=\xe9\n", b"");
        // The UTF-8 BOM is three ISO-8859-1 characters forming a key.
        assert_eq!(t.translate_key("\u{ef}\u{bb}\u{bf}"), "");
        assert_eq!(t.translate_key("a"), "\u{e9}");
    }

    #[test]
    fn format_specifiers() {
        let a = vec!["W".to_string(), "S".to_string()];
        assert_eq!(java_format("Use %1$s and %2$s", &a), "Use W and S");
        assert_eq!(java_format("%s Mined", &a), "W Mined");
        assert_eq!(java_format("100%% %d", &a), "100% W");
    }
}
