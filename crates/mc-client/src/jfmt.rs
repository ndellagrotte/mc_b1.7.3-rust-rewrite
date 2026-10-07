//! Java's text conversions that the client's files and strings depend on:
//! `Float.toString` / `Double.toString` (string concatenation of a `float` /
//! `double`), `Float.parseFloat`, `Integer.parseInt` and `String.split` with
//! a one-character regex.
//!
//! Not vanilla classes; the JDK's behaviour, which `options.txt`
//! (GameSettings.java:322-351) and the F3 screen (GuiIngame.java:185-188)
//! inherit.

/// `Float.toString(v)`: the shortest decimal that reads back as `v`, plain
/// for `1e-3 <= |v| < 1e7` (always with a fractional part: `1.0`), else
/// computerized scientific notation (`1.0E-4`, `1.2345678E7`).
pub fn float_to_string(v: f32) -> String {
    if v.is_nan() {
        return "NaN".into();
    }
    if v.is_infinite() {
        return if v > 0.0 { "Infinity" } else { "-Infinity" }.into();
    }
    let a = v.abs();
    if a == 0.0 || (1.0e-3..1.0e7).contains(&a) {
        plain(format!("{v}"))
    } else {
        scientific(format!("{v:e}"))
    }
}

/// `Double.toString(v)`: as [`float_to_string`] for a `double`.
pub fn double_to_string(v: f64) -> String {
    if v.is_nan() {
        return "NaN".into();
    }
    if v.is_infinite() {
        return if v > 0.0 { "Infinity" } else { "-Infinity" }.into();
    }
    let a = v.abs();
    if a == 0.0 || (1.0e-3..1.0e7).contains(&a) {
        plain(format!("{v}"))
    } else {
        scientific(format!("{v:e}"))
    }
}

fn plain(s: String) -> String {
    if s.contains('.') { s } else { s + ".0" }
}

fn scientific(s: String) -> String {
    // Rust: "1e-4", "1.5e7"; Java: "1.0E-4", "1.5E7".
    let (m, e) = s.split_once('e').expect("{:e} has an exponent");
    let m = if m.contains('.') {
        m.to_string()
    } else {
        format!("{m}.0")
    };
    format!("{m}E{e}")
}

/// Why a Java number parse failed (`NumberFormatException`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NumberFormatException(pub String);

impl std::fmt::Display for NumberFormatException {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "NumberFormatException: For input string: \"{}\"", self.0)
    }
}

/// `Float.parseFloat(s)`: leading and trailing characters `<= ' '` are
/// trimmed (`String.trim`), an optional `f`/`F`/`d`/`D` suffix is allowed,
/// `NaN` and `Infinity` (signed) are accepted; the rest is a decimal number
/// rounded to the nearest `float`. Hexadecimal floats are not supported
/// (they never occur in `options.txt`).
pub fn parse_float(s: &str) -> Result<f32, NumberFormatException> {
    let err = || NumberFormatException(s.to_string());
    let t = s.trim_matches(|c: char| c <= ' ');
    let (sign, body) = match t.strip_prefix('-') {
        Some(b) => (-1.0f32, b),
        None => (1.0f32, t.strip_prefix('+').unwrap_or(t)),
    };
    match body {
        "NaN" => return Ok(f32::NAN),
        "Infinity" => return Ok(sign * f32::INFINITY),
        _ => {}
    }
    let body = body.strip_suffix(['f', 'F', 'd', 'D']).unwrap_or(body);
    // Only digits, one '.', an exponent; Rust's parser also takes "inf"/"nan",
    // which Java rejects.
    if body.is_empty()
        || !body
            .chars()
            .all(|c| c.is_ascii_digit() || matches!(c, '.' | 'e' | 'E' | '+' | '-'))
        || !body.chars().any(|c| c.is_ascii_digit())
    {
        return Err(err());
    }
    body.parse::<f32>().map(|v| sign * v).map_err(|_| err())
}

/// `Integer.parseInt(s)`: an optional sign and ASCII digits, in `int`
/// range; no whitespace.
pub fn parse_int(s: &str) -> Result<i32, NumberFormatException> {
    let digits = s.strip_prefix(['+', '-']).unwrap_or(s);
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return Err(NumberFormatException(s.to_string()));
    }
    s.parse::<i32>()
        .map_err(|_| NumberFormatException(s.to_string()))
}

/// `Long.parseLong(s)`: as [`parse_int`] for a `long`.
pub fn parse_long(s: &str) -> Result<i64, NumberFormatException> {
    let digits = s.strip_prefix(['+', '-']).unwrap_or(s);
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return Err(NumberFormatException(s.to_string()));
    }
    s.parse::<i64>()
        .map_err(|_| NumberFormatException(s.to_string()))
}

/// `s.split(String.valueOf(c))` for a character that is not a regex
/// metacharacter: the pieces between occurrences, with trailing empty
/// pieces removed (an input without `c` is one piece, even when empty).
pub fn split(s: &str, c: char) -> Vec<&str> {
    let mut v: Vec<&str> = s.split(c).collect();
    if v.len() > 1 {
        while v.len() > 1 && v.last().is_some_and(|p| p.is_empty()) {
            v.pop();
        }
        // "::" splits into nothing at all in Java (every piece is a
        // trailing empty one).
        if v.len() == 1 && v[0].is_empty() && !s.is_empty() {
            v.clear();
        }
    }
    v
}

/// `String.hashCode()`: `s[0]*31^(n-1) + ... + s[n-1]` over the UTF-16 code
/// units, wrapping.
pub fn string_hash_code(s: &str) -> i32 {
    s.encode_utf16()
        .fold(0i32, |h, c| h.wrapping_mul(31).wrapping_add(c as i32))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn float_to_string_matches_java() {
        assert_eq!(float_to_string(1.0), "1.0");
        assert_eq!(float_to_string(0.5), "0.5");
        assert_eq!(float_to_string(0.0), "0.0");
        assert_eq!(float_to_string(-0.0), "-0.0");
        assert_eq!(float_to_string(0.33333334), "0.33333334");
        assert_eq!(float_to_string(0.1), "0.1");
        assert_eq!(float_to_string(1.0e-4), "1.0E-4");
        assert_eq!(float_to_string(0.001), "0.001");
        assert_eq!(float_to_string(1.0e7), "1.0E7");
        assert_eq!(float_to_string(1234567.0), "1234567.0");
        assert_eq!(float_to_string(f32::NAN), "NaN");
        assert_eq!(float_to_string(f32::NEG_INFINITY), "-Infinity");
    }

    #[test]
    fn double_to_string_matches_java() {
        assert_eq!(double_to_string(64.5), "64.5");
        assert_eq!(double_to_string(65.62000000476837), "65.62000000476837");
        assert_eq!(double_to_string(-12.0), "-12.0");
        assert_eq!(double_to_string(3.0e7), "3.0E7");
        assert_eq!(double_to_string(1.0e-5), "1.0E-5");
    }

    #[test]
    fn parse_float_matches_java() {
        assert_eq!(parse_float("0.5"), Ok(0.5));
        assert_eq!(parse_float(" 1.0 "), Ok(1.0));
        assert_eq!(parse_float("1f"), Ok(1.0));
        assert_eq!(parse_float("-2.5e1"), Ok(-25.0));
        assert_eq!(parse_float("1.0E-4"), Ok(1.0e-4));
        assert!(parse_float("NaN").unwrap().is_nan());
        assert_eq!(parse_float("-Infinity"), Ok(f32::NEG_INFINITY));
        assert!(parse_float("inf").is_err());
        assert!(parse_float("").is_err());
        assert!(parse_float("abc").is_err());
    }

    #[test]
    fn parse_int_matches_java() {
        assert_eq!(parse_int("17"), Ok(17));
        assert_eq!(parse_int("+3"), Ok(3));
        assert_eq!(parse_int("-3"), Ok(-3));
        assert!(parse_int(" 3").is_err());
        assert!(parse_int("2147483648").is_err());
        assert!(parse_int("").is_err());
        assert!(parse_int("-").is_err());
    }

    #[test]
    fn split_matches_java() {
        assert_eq!(split("music:1.0", ':'), vec!["music", "1.0"]);
        assert_eq!(split("lastServer:", ':'), vec!["lastServer"]);
        assert_eq!(split("a:b:c", ':'), vec!["a", "b", "c"]);
        assert_eq!(split(":a", ':'), vec!["", "a"]);
        assert_eq!(split("", ':'), vec![""]);
        assert_eq!(split("abc", ':'), vec!["abc"]);
        assert!(split("::", ':').is_empty());
    }

    #[test]
    fn hash_code_matches_java() {
        assert_eq!(string_hash_code(""), 0);
        assert_eq!(string_hash_code("a"), 97);
        // "hello".hashCode() == 99162322
        assert_eq!(string_hash_code("hello"), 99162322);
        // "Minecraft".hashCode() == -1595926131 (wraps)
        assert_eq!(string_hash_code("Minecraft"), -1595926131);
    }
}
