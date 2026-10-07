//! `FontRenderer` (FontRenderer.java) and `ChatAllowedCharacters`
//! (ChatAllowedCharacters.java): the 8×8 bitmap font of `/font/default.png`
//! with per-glyph widths measured from its pixel columns, the `§` colour
//! codes, the shadow, and the word wrap of `drawSplitString`.
//!
//! The glyphs are indexed by position in `/font.txt` (the allowed
//! characters) plus 32, not by code point: a character missing from that
//! file is skipped without advancing. Beta has no unicode font and no
//! random (`§k`) style; a bad colour code letter means white (15).
//!
//! Vanilla's display lists become direct calls on [`GuiGl`]: list `n < 256`
//! draws glyph `n` as a 7.99-pixel quad and translates by its width; lists
//! `256..288` set `glColor3f` (which also makes alpha 1, so text after a
//! colour code is opaque whatever alpha the call asked for).
//!
//! Strings are handled per `char` where Java works on UTF-16 units; they
//! differ only outside the Basic Multilingual Plane, which the font cannot
//! draw anyway.

use std::collections::BTreeMap;

use bevy::prelude::*;

use super::batch::{GuiGl, TexRef};
use crate::jfmt;
use crate::textures::ArgbImage;

/// The font texture's jar path.
// src: Minecraft.java:241
pub const FONT_PNG: &str = "/font/default.png";
/// The allowed characters' jar path.
// src: ChatAllowedCharacters.java:14
pub const FONT_TXT: &str = "/font.txt";

/// `ChatAllowedCharacters.allowedCharacters`: the characters the font
/// draws, in glyph order (glyph = index + 32), and the chat filter.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ChatAllowedCharacters {
    /// The characters, in file order.
    pub chars: Vec<char>,
    /// First index of each character (`String.indexOf`).
    index: BTreeMap<char, i32>,
}

impl ChatAllowedCharacters {
    /// `getAllowedCharacters()`: every line of `/font.txt` (UTF-8) that does
    /// not start with `#`, concatenated without the line terminators
    /// (`BufferedReader.readLine`: `\n`, `\r` or `\r\n`). An unreadable file
    /// gives the empty string (the `catch` swallows it).
    // src: ChatAllowedCharacters.java:10-28
    pub fn parse(bytes: &[u8]) -> Self {
        let text = String::from_utf8_lossy(bytes);
        let mut var0 = String::new();
        for var2 in text.split('\n') {
            // readLine splits at \r too; a "\r\n" pair ends one line.
            for (i, line) in var2.split('\r').enumerate() {
                if i > 0 && line.is_empty() && var2.ends_with('\r') {
                    continue;
                }
                if !line.starts_with('#') {
                    var0.push_str(line);
                }
            }
        }
        Self::from_chars(var0.chars().collect())
    }

    /// From the characters themselves.
    pub fn from_chars(chars: Vec<char>) -> Self {
        let mut index = BTreeMap::new();
        for (i, &c) in chars.iter().enumerate() {
            index.entry(c).or_insert(i as i32);
        }
        ChatAllowedCharacters { chars, index }
    }

    /// `allowedCharacters.indexOf(c)` (-1 when absent).
    pub fn index_of(&self, c: char) -> i32 {
        self.index.get(&c).copied().unwrap_or(-1)
    }

    /// `allowedCharacters.indexOf(c) >= 0`.
    pub fn is_allowed(&self, c: char) -> bool {
        self.index_of(c) >= 0
    }
}

/// `allowedCharactersArray`: the characters refused in file names.
// src: ChatAllowedCharacters.java:8
pub const ALLOWED_CHARACTERS_ARRAY: [char; 15] = [
    '/', '\n', '\r', '\t', '\u{0}', '\u{c}', '`', '?', '*', '\\', '<', '>', '|', '"', ':',
];

/// The constructor's width loop: for each of the 256 glyphs of a 128-pixel
/// wide sheet, the rightmost column (7 down to 0) with a pixel whose low
/// byte (blue, `& 255` of the ARGB int) is non-zero, plus 2; the space
/// (glyph 32) is forced to 2 + 2. An empty glyph ends the loop at -1 and
/// gets width 1.
// src: FontRenderer.java:34-60
#[allow(clippy::needless_range_loop)]
pub fn char_widths(var7: &[i32], var5: usize) -> [i32; 256] {
    let mut char_width = [0i32; 256];
    for var8 in 0..256usize {
        let var9 = var8 % 16;
        let var10 = var8 / 16;
        let mut var11: i32 = 7;
        while var11 >= 0 {
            let var12 = var9 * 8 + var11 as usize;
            let mut var13 = true;
            let mut var14 = 0;
            while var14 < 8 && var13 {
                let var15 = (var10 * 8 + var14) * var5;
                let var16 = var7.get(var12 + var15).copied().unwrap_or(0) & 255;
                if var16 > 0 {
                    var13 = false;
                }
                var14 += 1;
            }
            if !var13 {
                break;
            }
            var11 -= 1;
        }
        if var8 == 32 {
            var11 = 2;
        }
        char_width[var8] = var11 + 2;
    }
    char_width
}

/// The 32 colour lists (`fontDisplayLists + 256 + i`) as bytes: `§0`-`§f`,
/// then their shadows (a quarter of each channel), with the anaglyph
/// transform when the option is on at construction.
// src: FontRenderer.java:83-111
pub fn color_code_table(anaglyph: bool) -> [[i32; 3]; 32] {
    let mut t = [[0; 3]; 32];
    for (var9, e) in t.iter_mut().enumerate() {
        let var9 = var9 as i32;
        let var10 = (var9 >> 3 & 1) * 85;
        let mut var11 = (var9 >> 2 & 1) * 170 + var10;
        let mut var12 = (var9 >> 1 & 1) * 170 + var10;
        let mut var22 = (var9 & 1) * 170 + var10;
        // vanilla bug: only §6 (gold) gets the extra red; its shadow (22)
        // is a quarter of the plain dark yellow, (42, 42, 0).
        if var9 == 6 {
            var11 += 85;
        }
        let var24 = var9 >= 16;
        if anaglyph {
            let var15 = (var11 * 30 + var12 * 59 + var22 * 11) / 100;
            let var16 = (var11 * 30 + var12 * 70) / 100;
            let var17 = (var11 * 30 + var22 * 70) / 100;
            var11 = var15;
            var12 = var16;
            var22 = var17;
        }
        if var24 {
            var11 /= 4;
            var12 /= 4;
            var22 /= 4;
        }
        *e = [var11, var12, var22];
    }
    t
}

/// The glyph quad's size (`7.99F`).
// src: FontRenderer.java:71
const GLYPH: f32 = 7.99;

/// `FontRenderer` (module docs). A resource: `Minecraft.fontRenderer`.
#[derive(Resource, Clone, Debug)]
pub struct FontRenderer {
    /// `charWidth`.
    // src: FontRenderer.java:10
    pub char_width: [i32; 256],
    /// `fontTextureName`: the texture `allocateAndSetupTexture` made of the
    /// sheet (set at start-up); `None` falls back to `getTexture`.
    // src: FontRenderer.java:11, :62
    pub font_texture_name: Option<Handle<Image>>,
    /// The colour lists' `glColor3f` values (bytes).
    color_codes: [[i32; 3]; 32],
    /// `ChatAllowedCharacters.allowedCharacters`.
    pub allowed: ChatAllowedCharacters,
}

impl FontRenderer {
    /// `new FontRenderer(settings, "/font/default.png", renderEngine)`
    /// without the upload: the widths of `sheet` and the colour table.
    // src: FontRenderer.java:15-113
    pub fn new(sheet: &ArgbImage, anaglyph: bool, allowed: ChatAllowedCharacters) -> Self {
        let (w, _h, px) = sheet.view();
        FontRenderer {
            char_width: char_widths(px, w as usize),
            font_texture_name: None,
            color_codes: color_code_table(anaglyph),
            allowed,
        }
    }

    /// The colour list `i` (0..32) as bytes.
    pub fn color_code(&self, i: usize) -> [i32; 3] {
        self.color_codes[i]
    }

    fn texture(&self) -> TexRef {
        match &self.font_texture_name {
            Some(h) => TexRef::Handle(h.clone()),
            None => FONT_PNG.into(),
        }
    }

    /// `drawStringWithShadow(s, x, y, color)`: the shadow one pixel down
    /// and right, then the string.
    // src: FontRenderer.java:115-118
    pub fn draw_string_with_shadow(&self, gl: &mut GuiGl, s: &str, x: i32, y: i32, color: i32) {
        self.render_string(gl, Some(s), x + 1, y + 1, color, true);
        self.draw_string(gl, s, x, y, color);
    }

    /// `drawString(s, x, y, color)`.
    // src: FontRenderer.java:120-122
    pub fn draw_string(&self, gl: &mut GuiGl, s: &str, x: i32, y: i32, color: i32) {
        self.render_string(gl, Some(s), x, y, color, false);
    }

    /// `renderString(s, x, y, color, shadow)` (`None` is Java's `null`:
    /// nothing is drawn).
    // src: FontRenderer.java:124-181
    pub fn render_string(
        &self,
        gl: &mut GuiGl,
        var1: Option<&str>,
        var2: i32,
        var3: i32,
        mut var4: i32,
        var5: bool,
    ) {
        let Some(var1) = var1 else {
            return;
        };
        if var5 {
            let var6 = var4 & -16777216;
            var4 = (var4 & 16579836) >> 2;
            var4 = var4.wrapping_add(var6);
        }
        gl.bind_texture(self.texture());
        let var10 = (var4 >> 16 & 255) as f32 / 255.0;
        let var7 = (var4 >> 8 & 255) as f32 / 255.0;
        let var8 = (var4 & 255) as f32 / 255.0;
        let mut var9 = (var4 >> 24 & 255) as f32 / 255.0;
        if var9 == 0.0 {
            var9 = 1.0;
        }
        gl.color4f(var10, var7, var8, var9);
        gl.push_matrix();
        gl.translatef(var2 as f32, var3 as f32, 0.0);
        let s: Vec<char> = var1.chars().collect();
        let len = s.len();
        let mut var6 = 0;
        while var6 < len {
            while len > var6 + 1 && s[var6] == '\u{a7}' {
                let mut var11 = color_code_index(s[var6 + 1]);
                if !(0..=15).contains(&var11) {
                    var11 = 15;
                }
                self.call_color_list(gl, var11 as usize + if var5 { 16 } else { 0 });
                var6 += 2;
            }
            if var6 < len {
                let var11 = self.allowed.index_of(s[var6]);
                if var11 >= 0 {
                    self.call_glyph_list(gl, (var11 + 32) as usize);
                }
            }
            var6 += 1;
        }
        gl.pop_matrix();
    }

    /// Display list `256 + i`: `glColor3f` of colour `i`.
    // src: FontRenderer.java:108-110
    fn call_color_list(&self, gl: &mut GuiGl, i: usize) {
        let [r, g, b] = self.color_codes[i];
        gl.color3f(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0);
    }

    /// Display list `n` (< 256): glyph `n`'s quad, then the advance.
    // src: FontRenderer.java:66-81
    fn call_glyph_list(&self, gl: &mut GuiGl, var9: usize) {
        let Some(&w) = self.char_width.get(var9) else {
            return;
        };
        let var10 = (var9 % 16 * 8) as f32;
        let var11 = (var9 / 16 * 8) as f32;
        let var20 = GLYPH;
        gl.start_drawing_quads();
        gl.add_vertex_with_uv(
            0.0,
            (0.0 + var20) as f64,
            0.0,
            (var10 / 128.0) as f64,
            ((var11 + var20) / 128.0) as f64,
        );
        gl.add_vertex_with_uv(
            (0.0 + var20) as f64,
            (0.0 + var20) as f64,
            0.0,
            ((var10 + var20) / 128.0) as f64,
            ((var11 + var20) / 128.0) as f64,
        );
        gl.add_vertex_with_uv(
            (0.0 + var20) as f64,
            0.0,
            0.0,
            ((var10 + var20) / 128.0) as f64,
            (var11 / 128.0) as f64,
        );
        gl.add_vertex_with_uv(
            0.0,
            0.0,
            0.0,
            (var10 / 128.0) as f64,
            (var11 / 128.0) as f64,
        );
        gl.draw();
        gl.translatef(w as f32, 0.0, 0.0);
    }

    /// `getStringWidth(s)`: the advances of the drawable characters; a `§`
    /// skips itself and the next character.
    // src: FontRenderer.java:183-202
    pub fn get_string_width(&self, var1: &str) -> i32 {
        let s: Vec<char> = var1.chars().collect();
        let mut var2 = 0i32;
        let mut var3 = 0;
        while var3 < s.len() {
            if s[var3] == '\u{a7}' {
                var3 += 1;
            } else {
                let var4 = self.allowed.index_of(s[var3]);
                if var4 >= 0 {
                    var2 += self.char_width[(var4 + 32) as usize];
                }
            }
            var3 += 1;
        }
        var2
    }

    /// `getStringWidth(null)` is 0.
    pub fn get_string_width_opt(&self, s: Option<&str>) -> i32 {
        s.map_or(0, |s| self.get_string_width(s))
    }

    /// `drawSplitString(s, x, y, width, color)`: each `\n` line word-wrapped
    /// to `width`, 8 pixels a line, without shadow.
    // src: FontRenderer.java:204-239
    pub fn draw_split_string(
        &self,
        gl: &mut GuiGl,
        var1: &str,
        var2: i32,
        mut var3: i32,
        var4: i32,
        var5: i32,
    ) {
        let var6 = jfmt::split(var1, '\n');
        if var6.len() > 1 {
            for line in &var6 {
                self.draw_split_string(gl, line, var2, var3, var4, var5);
                var3 += self.split_string_width(line, var4);
            }
        } else {
            for line in self.wrap(var1, var4) {
                self.draw_string(gl, &line, var2, var3, var5);
                var3 += 8;
            }
        }
    }

    /// `splitStringWidth(s, width)`: the height `drawSplitString` takes (at
    /// least 8 per `\n` line).
    // src: FontRenderer.java:241-283
    pub fn split_string_width(&self, var1: &str, var2: i32) -> i32 {
        let var3 = jfmt::split(var1, '\n');
        if var3.len() > 1 {
            let mut var9 = 0;
            for line in &var3 {
                var9 += self.split_string_width(line, var2);
            }
            var9
        } else {
            let mut var6 = self.wrap(var1, var2).len() as i32 * 8;
            if var6 < 8 {
                var6 += 8;
            }
            var6
        }
    }

    /// The lines one `\n`-free string wraps into (the shared loop of
    /// `drawSplitString` and `splitStringWidth`): words are joined while
    /// the line plus the next word stays under `width`; a line still too
    /// wide is cut at the last character that fits; blank pieces are not
    /// lines.
    // src: FontRenderer.java:213-236, :253-275
    fn wrap(&self, var1: &str, var4: i32) -> Vec<String> {
        let mut out = Vec::new();
        let var7 = jfmt::split(var1, ' ');
        let mut var8 = 0;
        while var8 < var7.len() {
            let mut var9 = format!("{} ", var7[var8]);
            var8 += 1;
            while var8 < var7.len()
                && self.get_string_width(&format!("{var9}{}", var7[var8])) < var4
            {
                var9 = format!("{var9}{} ", var7[var8]);
                var8 += 1;
            }
            while self.get_string_width(&var9) > var4 {
                let chars: Vec<char> = var9.chars().collect();
                let mut var10 = 0;
                // vanilla bug: when the first character alone is wider
                // than `width`, var10 stays 0 and this loop never ends.
                while var10 < chars.len()
                    && self.get_string_width(&chars[..var10 + 1].iter().collect::<String>()) <= var4
                {
                    var10 += 1;
                }
                let head: String = chars[..var10].iter().collect();
                if !java_trim(&head).is_empty() {
                    out.push(head);
                }
                var9 = chars[var10..].iter().collect();
            }
            if !java_trim(&var9).is_empty() {
                out.push(var9);
            }
        }
        out
    }
}

/// `"0123456789abcdef".indexOf(Character.toLowerCase(c))`.
// src: FontRenderer.java:150
pub fn color_code_index(c: char) -> i32 {
    let c = c.to_lowercase().next().unwrap_or(c);
    "0123456789abcdef".find(c).map_or(-1, |i| i as i32)
}

/// `String.trim()`: strips characters `<= ' '` from both ends.
pub fn java_trim(s: &str) -> &str {
    s.trim_matches(|c: char| c <= ' ')
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui::batch::GuiGl;

    /// A 128×128 sheet whose glyph `g` has its rightmost lit column at
    /// `cols[g]` (or none), lit in the blue channel only for glyph 1.
    fn sheet(cols: &[(usize, i32)]) -> ArgbImage {
        let mut img = ArgbImage::new(128, 128);
        let (w, _, _) = img.view();
        let w = w as usize;
        for &(g, col) in cols {
            let (gx, gy) = (g % 16 * 8, g / 16 * 8);
            for x in 0..=col as usize {
                // Every column up to `col` on row 3 of the glyph.
                let p = if g == 1 { 0x0000_0001 } else { -1 };
                img.pixels[(gy + 3) * w + gx + x] = p;
            }
        }
        img
    }

    #[test]
    fn widths_from_pixel_columns() {
        let img = sheet(&[(65, 4), (1, 0), (66, 7), (32, 7)]);
        let (w, _, px) = img.view();
        let cw = char_widths(px, w as usize);
        assert_eq!(cw[65], 6); // column 4 → 4 + 2
        assert_eq!(cw[1], 2); // only the blue byte is looked at
        assert_eq!(cw[66], 9); // full width
        assert_eq!(cw[32], 4); // the space is always 2 + 2
        assert_eq!(cw[0], 1); // empty: -1 + 2
        // Only the low byte counts: an alpha-only pixel is empty.
        let mut img = ArgbImage::new(128, 128);
        img.pixels[3 * 128 + 8 * 2 + 5] = 0x7f00_0000;
        let (w, _, px) = img.view();
        assert_eq!(char_widths(px, w as usize)[2], 1);
    }

    #[test]
    fn colour_table() {
        let t = color_code_table(false);
        assert_eq!(t[0], [0, 0, 0]);
        assert_eq!(t[1], [0, 0, 170]);
        assert_eq!(t[6], [255, 170, 0]);
        assert_eq!(t[7], [170, 170, 170]);
        assert_eq!(t[8], [85, 85, 85]);
        assert_eq!(t[12], [255, 85, 85]);
        assert_eq!(t[15], [255, 255, 255]);
        assert_eq!(t[16 + 15], [63, 63, 63]);
        assert_eq!(t[16 + 6], [42, 42, 0]);
        assert_eq!(t[16 + 8], [21, 21, 21]);
        let a = color_code_table(true);
        // §c (255, 85, 85): ((255·30 + 85·59 + 85·11) / 100, (255·30 + 85·70)/100, …)
        assert_eq!(a[12], [136, 136, 136]);
    }

    #[test]
    fn colour_code_parsing() {
        assert_eq!(color_code_index('0'), 0);
        assert_eq!(color_code_index('a'), 10);
        assert_eq!(color_code_index('F'), 15);
        assert_eq!(color_code_index('g'), -1);
        assert_eq!(color_code_index('k'), -1);
    }

    fn font() -> FontRenderer {
        let mut cols: Vec<(usize, i32)> = Vec::new();
        // ' ' 'a' 'b' 'c' at glyphs 32..36: widths 4, 6, 6, 4
        cols.extend([(33, 4), (34, 4), (35, 2)]);
        let allowed = ChatAllowedCharacters::from_chars(vec![' ', 'a', 'b', 'c']);
        FontRenderer::new(&sheet(&cols), false, allowed)
    }

    #[test]
    fn string_widths_skip_codes_and_unknown_characters() {
        let f = font();
        assert_eq!(f.get_string_width("abc"), 6 + 6 + 4);
        assert_eq!(f.get_string_width("a b"), 6 + 4 + 6);
        assert_eq!(f.get_string_width("\u{a7}cab"), 12);
        assert_eq!(f.get_string_width("a\u{a7}"), 6);
        assert_eq!(f.get_string_width("aZa"), 12);
        assert_eq!(f.get_string_width_opt(None), 0);
    }

    #[test]
    fn render_string_lays_out_glyphs_and_colours() {
        let f = font();
        let mut gl = GuiGl::default();
        f.draw_string_with_shadow(&mut gl, "a\u{a7}cb", 10, 20, 0xffffff);
        let b = &gl.draws()[0].batch;
        // Shadow 'a' at (11, 21), shadow 'b' after 6; then the string.
        assert_eq!(b.len(), 16);
        assert_eq!(b.positions[3], [11.0, 21.0, 0.0]);
        assert_eq!(b.positions[7], [17.0, 21.0, 0.0]);
        assert_eq!(b.positions[11], [10.0, 20.0, 0.0]);
        // (0xffffff & 0xfcfcfc) >> 2 = 0x3f3f3f, alpha 0 → 1.
        let q = 63.0 / 255.0;
        assert_eq!(b.colors[0], [q, q, q, 1.0]);
        // The shadow of §c: (63, 21, 21).
        assert_eq!(b.colors[4], [63.0 / 255.0, 21.0 / 255.0, 21.0 / 255.0, 1.0]);
        assert_eq!(b.colors[8], [1.0, 1.0, 1.0, 1.0]);
        assert_eq!(b.colors[12], [1.0, 85.0 / 255.0, 85.0 / 255.0, 1.0]);
        // UVs of glyph 'a' (33): column 1, row 2.
        assert_eq!(b.uvs[3], [8.0 / 128.0, 16.0 / 128.0]);
        assert_eq!(b.uvs[1], [(8.0 + 7.99) / 128.0, (16.0 + 7.99) / 128.0]);
        // A bad code letter is white; a trailing § draws nothing.
        let mut gl = GuiGl::default();
        f.draw_string(&mut gl, "\u{a7}za\u{a7}", 0, 0, 0x40ff0000);
        let b = &gl.draws()[0].batch;
        assert_eq!(b.len(), 4);
        assert_eq!(b.colors[0], [1.0, 1.0, 1.0, 1.0]);
    }

    #[test]
    fn word_wrap() {
        let f = font();
        // "ab ab ab": "ab ab" (28) joins, "ab ab " (32) is then cut back to
        // "ab ab"; the leftover " " is blank.
        assert_eq!(f.wrap("ab ab ab", 30), vec!["ab ab", "ab "]);
        assert_eq!(f.split_string_width("ab ab ab", 30), 16);
        assert_eq!(f.split_string_width("", 30), 8);
        assert_eq!(f.split_string_width("ab\nab", 30), 16);
        // A word wider than the line is cut.
        assert_eq!(f.wrap("aaaaaa", 20), vec!["aaa", "aaa"]);
    }

    #[test]
    fn allowed_characters_file() {
        let a = ChatAllowedCharacters::parse(b"# comment\r\n !\"\r\nab\nc\rd\n#x\n");
        assert_eq!(a.chars, vec![' ', '!', '"', 'a', 'b', 'c', 'd']);
        assert_eq!(a.index_of('a'), 3);
        assert_eq!(a.index_of('z'), -1);
        assert!(a.is_allowed('d'));
    }
}
