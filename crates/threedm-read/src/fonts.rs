//! Fonts: `ON_Font` as dimension styles keep it (Rhino 6 and later), the font table of Rhino 4
//! and 5 (`TCODE_FONT_TABLE`, `ON_TextStyle`), and which font a text, dimension or leader uses:
//! that of its first character, from its rich text's font table and bold and italic words, else
//! its style's.

use super::dimensions::{is_annotation, V5_ANNOTATIONS};
use super::drafting::{read_annotation, DimStyle, ON_TEXT};
use super::{Cur, Uuid, R, TCODE_ANONYMOUS_CHUNK};
use threedm_model::Font;

/// `TCODE_FONT_TABLE` and its records: the text styles of Rhino 4 and 5 files.
pub(super) const TCODE_FONT_TABLE: u32 = 0x1000_0019;
pub(super) const TCODE_FONT_RECORD: u32 = 0x2000_8074;
/// `ON_TextStyle`, the class of the font table's records.
pub(super) const ON_TEXT_STYLE: Uuid = Uuid::parse("4F0F51FB-35D0-4865-9998-6D2C6A99721D");
/// `TCODE_UTF8_STRING_CHUNK`: a string as UTF-8 (`WriteWideString`).
const TCODE_UTF8_STRING_CHUNK: u32 = 0x4000_8001;
/// The longest rich text looked into for its first character's font.
const MAX_RICH_TEXT: usize = 1 << 20;

/// `ON_Font::Weight` and `ON_Font::Style` to bold and italic as Rhino has them.
fn font(family: String, face: String, postscript: String, logfont: String, weight: u8, style: u8) -> Font {
    Font { bold: weight >= 6, italic: style == 2, family, face, postscript, logfont, weight, style, ..Font::default() }
}

/// `ReadWideString`: a `TCODE_UTF8_STRING_CHUNK` chunk with a format byte (0 empty, 1 UTF-8)
/// and the bytes.
fn utf8_string(c: &mut Cur<'_>) -> R<String> {
    let chunk = c.chunk()?;
    if chunk.typecode != TCODE_UTF8_STRING_CHUNK {
        return super::err("expected a UTF-8 string");
    }
    let mut b = chunk.body;
    if b.u8()? == 0 {
        return Ok(String::new());
    }
    let rest = b.take(b.left())?;
    Ok(String::from_utf8_lossy(rest).trim_end_matches('\0').to_string())
}

/// `ON_Font::Read` (Rhino 6 and later): the chunk's body, a major and minor version, then the
/// weight, style and stretch packed in one number, the LOGFONT name, the PostScript name and,
/// as the version allows, the family and the localised and English names and faces.
pub(super) fn read_font(mut c: Cur<'_>) -> R<Font> {
    let major = c.i32()?;
    let minor = c.i32()?;
    if major != 1 {
        return super::err("a font of an unknown version");
    }
    // FontCharacteristicsAsUnsigned: 1, then weight (10), style (4), stretch (10), underlined
    // and strikethrough (2 each), as digits of those bases
    let mut u = c.u32()?;
    let one = u % 2;
    u /= 2;
    let weight = (u % 10) as u8;
    u /= 10;
    let style = (u % 4) as u8;
    u /= 4;
    u /= 10; // stretch
    let underlined = u % 2 == 1;
    u /= 2;
    let strikeout = u % 2 == 1;
    let (weight, style) = if one == 1 { (if weight == 0 { 4 } else { weight }, style) } else { (4, 1) };
    let logfont = utf8_string(&mut c)?;
    let postscript = c.wstring()?;
    let mut f = font(String::new(), String::new(), postscript, logfont.clone(), weight, style);
    (f.underlined, f.strikeout) = (one == 1 && underlined, one == 1 && strikeout);
    // what the version adds, as far as it reads
    let _ = (|| -> R<()> {
        if minor < 1 {
            return Ok(());
        }
        c.wstring()?; // obsolete description
        if minor < 2 {
            return Ok(());
        }
        c.i32()?; // LOGFONT weight
        c.f64()?; // Apple weight trait
        if minor < 3 {
            return Ok(());
        }
        c.f64()?; // point size
        if c.bool()? {
            c.take(4 + 16)?; // obsolete LOGFONT values
        }
        if minor < 4 {
            return Ok(());
        }
        f.family = c.wstring()?;
        if minor < 5 {
            return Ok(());
        }
        c.wstring()?; // locale
        let loc_postscript = c.wstring()?;
        let en_postscript = c.wstring()?;
        c.wstring()?; // localised LOGFONT name
        c.wstring()?; // English LOGFONT name
        let loc_family = c.wstring()?;
        let en_family = c.wstring()?;
        let loc_face = c.wstring()?;
        let en_face = c.wstring()?;
        for (field, loc, en) in [(&mut f.postscript, loc_postscript, en_postscript), (&mut f.family, loc_family, en_family), (&mut f.face, loc_face, en_face)] {
            if !loc.is_empty() {
                *field = loc;
            } else if !en.is_empty() {
                *field = en;
            }
        }
        Ok(())
    })();
    if f.family.is_empty() {
        f.family = logfont;
    }
    Ok(f)
}

/// `ON_Font::ReadV5`, a record of the font table of Rhino 4 and 5 (`ON_TextStyle`): a version,
/// the index, a description, the LOGFONT face name (64 UTF-16 units), from 1.1 its weight
/// (100 to 1000) and italic.
pub(super) fn read_v5_font(mut c: Cur<'_>) -> R<(i32, Font)> {
    let (major, minor) = c.version()?;
    if major != 1 {
        return super::err("a font of an unknown version");
    }
    let index = c.i32()?;
    let description = c.wstring()?;
    let units: Vec<u16> = c.take(128)?.chunks_exact(2).map(|b| u16::from_le_bytes([b[0], b[1]])).collect();
    let end = units.iter().position(|&u| u == 0).unwrap_or(units.len());
    let logfont = String::from_utf16_lossy(&units[..end]);
    let (mut weight, mut style) = (4, 1);
    if minor >= 1 {
        // ON_Font::WeightFromWindowsLogfontWeight
        weight = match c.i32()? {
            w @ 100..=1000 => ((w + 50) / 100).clamp(1, 9) as u8,
            _ => 4,
        };
        style = if c.i32()? != 0 { 2 } else { 1 };
    }
    // Apple files keep the PostScript name in the description
    let postscript = if description.contains('-') && !description.contains(' ') { description } else { String::new() };
    Ok((index, font(logfont.clone(), String::new(), postscript, logfont, weight, style)))
}

/// The font of the first character of rich text (RTF) as Rhino writes it: its font table, then
/// the `\fN`, `\b` and `\i` in force where the first character is. `None` when it is not rich
/// text or has no text.
pub(super) fn rich_text_font(rich: &str) -> Option<Font> {
    if !rich.trim_start().starts_with("{\\rtf") || rich.len() > MAX_RICH_TEXT {
        return None;
    }
    let chars: Vec<char> = rich.chars().collect();
    let mut table: Vec<(i64, String)> = Vec::new();
    // the font, bold and italic of each open group, and whether its text is left out
    #[derive(Clone, Copy)]
    struct State {
        font: i64,
        bold: bool,
        italic: bool,
        hidden: bool,
    }
    let mut stack = vec![State { font: 0, bold: false, italic: false, hidden: false }];
    let mut default_font = 0;
    let mut i = 0;
    while i < chars.len() {
        let state = *stack.last()?;
        match chars[i] {
            '{' => {
                let rest: String = chars[i + 1..chars.len().min(i + 12)].iter().collect();
                if rest.starts_with("\\fonttbl") {
                    let (fonts, end) = font_table(&chars, i);
                    table = fonts;
                    i = end;
                    continue;
                }
                let hidden = ["\\colortbl", "\\stylesheet", "\\info", "\\*"].iter().any(|d| rest.starts_with(d));
                stack.push(State { hidden: state.hidden || hidden, ..state });
                i += 1;
            }
            '}' => {
                stack.pop();
                if stack.is_empty() {
                    return None;
                }
                i += 1;
            }
            '\\' => {
                let next = chars.get(i + 1).copied().unwrap_or(' ');
                if !next.is_ascii_alphabetic() {
                    // an escaped character is text; other control symbols are not
                    if matches!(next, '\\' | '{' | '}' | '\'') && !state.hidden {
                        return Some(first_font(&table, state.font, default_font, state.bold, state.italic));
                    }
                    i += 2;
                    continue;
                }
                let start = i + 1;
                i = start;
                while i < chars.len() && chars[i].is_ascii_alphabetic() {
                    i += 1;
                }
                let word: String = chars[start..i].iter().collect();
                let digits = i;
                if i < chars.len() && chars[i] == '-' {
                    i += 1;
                }
                while i < chars.len() && chars[i].is_ascii_digit() {
                    i += 1;
                }
                let number: Option<i64> = chars[digits..i].iter().collect::<String>().parse().ok();
                if i < chars.len() && chars[i] == ' ' {
                    i += 1;
                }
                let top = stack.last_mut()?;
                match word.as_str() {
                    "f" => top.font = number.unwrap_or(0),
                    "deff" => default_font = number.unwrap_or(0),
                    "b" => top.bold = number != Some(0),
                    "i" => top.italic = number != Some(0),
                    "plain" => (top.bold, top.italic) = (false, false),
                    "u" if !top.hidden => return Some(first_font(&table, top.font, default_font, top.bold, top.italic)),
                    _ => {}
                }
            }
            '\r' | '\n' => i += 1,
            _ if state.hidden => i += 1,
            _ => return Some(first_font(&table, state.font, default_font, state.bold, state.italic)),
        }
    }
    None
}

/// The font numbered `number` in `table` (else the default one), bold and italic as given.
fn first_font(table: &[(i64, String)], number: i64, default: i64, bold: bool, italic: bool) -> Font {
    let family = table.iter().find(|(n, _)| *n == number).or_else(|| table.iter().find(|(n, _)| *n == default)).map(|(_, f)| f.clone()).unwrap_or_default();
    font(family.clone(), String::new(), String::new(), family, if bold { 7 } else { 4 }, if italic { 2 } else { 1 })
}

/// `{\fonttbl{\f0 Arial;}{\f1\fnil Times New Roman;}}` from `start` (its `{`): the fonts by
/// number, and where the table ends.
fn font_table(chars: &[char], start: usize) -> (Vec<(i64, String)>, usize) {
    let mut fonts = Vec::new();
    let (mut depth, mut i) = (0usize, start);
    let (mut number, mut name) = (None, String::new());
    while i < chars.len() {
        match chars[i] {
            '{' => depth += 1,
            '}' => {
                if let Some(n) = number.take() {
                    fonts.push((n, name.trim().trim_end_matches(';').trim().to_string()));
                }
                name.clear();
                depth -= 1;
                if depth == 0 {
                    return (fonts, i + 1);
                }
            }
            '\\' => {
                let s = i + 1;
                let mut j = s;
                while j < chars.len() && chars[j].is_ascii_alphabetic() {
                    j += 1;
                }
                let word: String = chars[s..j].iter().collect();
                let d = j;
                while j < chars.len() && (chars[j].is_ascii_digit() || chars[j] == '-') {
                    j += 1;
                }
                if word == "f" {
                    number = chars[d..j].iter().collect::<String>().parse().ok();
                    name.clear();
                }
                if j < chars.len() && chars[j] == ' ' {
                    j += 1;
                }
                i = j.max(s + 1);
                continue;
            }
            c => {
                if depth == 2 && !name.ends_with(';') {
                    name.push(c);
                }
            }
        }
        i += 1;
    }
    (fonts, chars.len())
}

/// The font of a text, dimension or leader of class `class` (its data at `c`): of its first
/// character, from its rich text, else its own style's, else its dimension style's (`styles`;
/// `fonts`, the font table of Rhino 4 and 5, for their styles). `None` for other objects.
pub(super) fn object_font(class: Uuid, mut c: Cur<'_>, styles: &[DimStyle], fonts: &[(i32, Font)]) -> Option<Font> {
    let style_font = |s: &DimStyle| s.font.clone().or_else(|| fonts.iter().find(|(i, _)| *i == s.text_style).map(|(_, f)| f.clone()));
    if class == ON_TEXT || is_annotation(class) {
        let (mut outer, _, _) = c.versioned(TCODE_ANONYMOUS_CHUNK).ok()?;
        let a = if class == ON_TEXT || class == super::dimensions::ON_LEADER {
            read_annotation(&mut outer).ok()?
        } else {
            let (mut d, _, _) = outer.versioned(TCODE_ANONYMOUS_CHUNK).ok()?;
            read_annotation(&mut d).ok()?
        };
        let style = styles.iter().find(|s| s.id == a.style).or(styles.first());
        let base = a.own_style.as_ref().and_then(|s| s.font.clone()).or_else(|| style.and_then(style_font));
        return Some(match (rich_text_font(&a.rich), base) {
            (Some(mut f), Some(b)) => {
                // the style's names when the text is in the style's family
                if f.family == b.family {
                    let (bold, italic) = (f.bold, f.italic);
                    f = Font { weight: if bold == b.bold { b.weight } else { f.weight }, style: if italic == b.italic { b.style } else { f.style }, bold, italic, ..b };
                    if (bold, italic) != (b.bold, b.italic) {
                        f.face = String::new();
                        f.postscript = String::new();
                    }
                }
                f
            }
            (Some(f), None) => f,
            (None, b) => b?,
        });
    }
    if V5_ANNOTATIONS.contains(&class) {
        let (text, style_index) = v5_text_and_style(c).ok()?;
        let style = styles.iter().find(|s| s.index == style_index).or(styles.first());
        let base = style.and_then(style_font);
        return rich_text_font(&text).or(base);
    }
    None
}

/// The text and style index of a Rhino 4 or 5 annotation (`ON_OBSOLETE_V5_Annotation`).
fn v5_text_and_style(c: Cur<'_>) -> R<(String, i32)> {
    let in_chunk = c.v.archive >= 5 && c.v.opennurbs >= 200_710_180;
    let mut b = c;
    while b.peek_typecode() == Some(TCODE_ANONYMOUS_CHUNK) {
        b = b.versioned(TCODE_ANONYMOUS_CHUNK)?.0;
    }
    if !in_chunk {
        b.version()?;
    }
    b.take(8 + 4 * 24 + 32)?; // type, display mode, plane
    let n = b.count(16)?;
    b.take(16 * n)?;
    let text = b.wstring()?;
    b.u32()?;
    Ok((text, b.i32()?))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_first_characters_font_is_read_from_rich_text() {
        let f = rich_text_font("{\\rtf1\\deff0{\\fonttbl{\\f0 Arial;}{\\f1 Times New Roman;}}\\f0 \\fs23{\\f1\\i Times italic}}").unwrap();
        assert_eq!((f.family.as_str(), f.bold, f.italic), ("Times New Roman", false, true));
        let f = rich_text_font("{\\rtf1\\deff0{\\fonttbl{\\f0 Arial;}}\\f0 \\fs23{\\f0\\b Arial bold}}").unwrap();
        assert_eq!((f.family.as_str(), f.bold, f.italic, f.weight), ("Arial", true, false, 7));
        let f = rich_text_font("{\\rtf1{\\fonttbl{\\f0\\fnil\\fcharset0 Courier New;}}{\\b\\b0 plain}}").unwrap();
        assert_eq!((f.family.as_str(), f.bold), ("Courier New", false));
        assert!(rich_text_font("plain text").is_none());
        assert!(rich_text_font("{\\rtf1{\\fonttbl{\\f0 Arial;}}}").is_none());
    }
}
