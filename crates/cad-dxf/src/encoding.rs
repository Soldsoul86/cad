//! Text encodings: the code page of pre-2007 files, and the escapes DXF
//! uses inside strings (`\U+XXXX`, `\M+nXXXX`, `%%` codes).

use std::borrow::Cow;

use encoding_rs::Encoding;

/// A Windows code page, as named by `$DWGCODEPAGE` (`ANSI_1252` → 1252).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CodePage(pub u16);

impl CodePage {
    /// The default for files that don't say (Western European).
    pub const ANSI_1252: CodePage = CodePage(1252);

    /// Parses a `$DWGCODEPAGE` value such as `ANSI_1250` (any case).
    /// `None` for names this library cannot decode.
    pub fn from_dwgcodepage(name: &str) -> Option<Self> {
        let name = name.trim();
        let number = name
            .get(..5)
            .filter(|p| p.eq_ignore_ascii_case("ANSI_"))
            .and_then(|_| name[5..].parse::<u16>().ok())?;
        let page = CodePage(number);
        page.encoding().map(|_| page)
    }

    /// The `$DWGCODEPAGE` name, e.g. `ANSI_1252`.
    pub fn dwgcodepage(self) -> String {
        format!("ANSI_{}", self.0)
    }

    /// The decoder for this code page, if supported: 874 (Thai), 932
    /// (Japanese), 936 (Simplified Chinese), 949 (Korean), 950 (Traditional
    /// Chinese) and 1250–1258.
    pub(crate) fn encoding(self) -> Option<&'static Encoding> {
        Some(match self.0 {
            874 => encoding_rs::WINDOWS_874,
            932 => encoding_rs::SHIFT_JIS,
            936 => encoding_rs::GBK,
            949 => encoding_rs::EUC_KR,
            950 => encoding_rs::BIG5,
            1250 => encoding_rs::WINDOWS_1250,
            1251 => encoding_rs::WINDOWS_1251,
            1252 => encoding_rs::WINDOWS_1252,
            1253 => encoding_rs::WINDOWS_1253,
            1254 => encoding_rs::WINDOWS_1254,
            1255 => encoding_rs::WINDOWS_1255,
            1256 => encoding_rs::WINDOWS_1256,
            1257 => encoding_rs::WINDOWS_1257,
            1258 => encoding_rs::WINDOWS_1258,
            _ => return None,
        })
    }

    /// True for code pages where every byte is one character.
    fn is_single_byte(self) -> bool {
        !matches!(self.0, 932 | 936 | 949 | 950)
    }

    /// Decodes `bytes`, replacing invalid sequences; the flag is true if any
    /// were replaced.
    pub(crate) fn decode_lossy(self, bytes: &[u8]) -> (String, bool) {
        let encoding = self.encoding().unwrap_or(encoding_rs::WINDOWS_1252);
        let (text, had_errors) = encoding.decode_without_bom_handling(bytes);
        (text.into_owned(), had_errors)
    }

    /// Decodes `bytes`; `None` if they are not valid in this code page.
    pub(crate) fn decode(self, bytes: &[u8]) -> Option<String> {
        let (text, had_errors) = self.encoding()?.decode_without_bom_handling(bytes);
        (!had_errors).then(|| text.into_owned())
    }
}

/// How a file's bytes were turned into text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FileEncoding {
    /// UTF-8: R2007 and later, and older files whose bytes are valid UTF-8
    /// with non-ASCII text.
    #[default]
    Utf8,
    /// A Windows code page (pre-2007 files). ASCII-only pre-2007 files report
    /// their declared code page, which is what a writer should use.
    CodePage(CodePage),
}

/// The code page selected by `\M+n`: 1 = 932, 2 = 950, 3 = 949, 4 = 1361
/// (Johab, unsupported), 5 = 936.
fn mbcs_code_page(n: char) -> Option<CodePage> {
    Some(CodePage(match n {
        '1' => 932,
        '2' => 950,
        '3' => 949,
        '5' => 936,
        _ => return None,
    }))
}

fn hex4(s: &str) -> Option<u32> {
    let digits = s.get(..4)?;
    if digits.bytes().all(|b| b.is_ascii_hexdigit()) {
        u32::from_str_radix(digits, 16).ok()
    } else {
        None
    }
}

/// Which string the escapes appear in.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    /// `TEXT`, `ATTRIB`, `ATTDEF`, dimension text: `%%nnn` is allowed.
    Text,
    /// `MTEXT`: `\\` is an escaped backslash, and formatting codes stay.
    MText,
}

/// Decodes the escapes in a single-line text value (`TEXT`, `ATTRIB`,
/// `ATTDEF`, dimension text override):
///
/// - `\U+XXXX` → the Unicode character U+XXXX
/// - `\M+nXXXX` → the double-byte character XXXX in Asian code page `n`
/// - `%%d` → `°`, `%%p` → `±`, `%%c` → `⌀`, `%%%` → `%` (any case)
/// - `%%nnn` → character number `nnn` in `code_page`
///
/// Anything else (including `%%o`/`%%u` toggles and malformed escapes) is
/// left as written. Returns the input unchanged (borrowed) when there is
/// nothing to decode.
pub fn decode_text(raw: &str, code_page: CodePage) -> Cow<'_, str> {
    decode(raw, code_page, Kind::Text)
}

/// Decodes the character escapes in `MTEXT` content: `\U+XXXX`,
/// `\M+nXXXX`, `%%d`, `%%p`, `%%c` and `%%%`. Formatting codes (`\P`,
/// `\f...;`, `\\`, ...) are kept, so the result is still MTEXT markup.
pub fn decode_mtext(raw: &str, code_page: CodePage) -> Cow<'_, str> {
    decode(raw, code_page, Kind::MText)
}

fn decode(raw: &str, code_page: CodePage, kind: Kind) -> Cow<'_, str> {
    if !raw.contains('\\') && !raw.contains("%%") {
        return Cow::Borrowed(raw);
    }
    let mut out = String::with_capacity(raw.len());
    let mut rest = raw;
    while let Some(c) = rest.chars().next() {
        let (decoded, used) = match c {
            '\\' => backslash(rest, kind),
            '%' => percent(rest, code_page, kind),
            _ => (None, 0),
        };
        match decoded {
            Some(decoded) => {
                out.push_str(&decoded);
                rest = &rest[used..];
            }
            None => {
                out.push(c);
                rest = &rest[c.len_utf8()..];
            }
        }
    }
    if out == raw {
        Cow::Borrowed(raw)
    } else {
        Cow::Owned(out)
    }
}

/// Decodes a backslash escape at the start of `s`: the replacement and the
/// number of bytes it consumed, or `None` to copy the backslash literally.
fn backslash(s: &str, kind: Kind) -> (Option<String>, usize) {
    let tail = &s[1..];
    if kind == Kind::MText && tail.starts_with('\\') {
        // An escaped backslash: keep both so `\\U+0041` stays literal.
        return (Some("\\\\".into()), 2);
    }
    if let Some(hex) = tail.strip_prefix("U+").or_else(|| tail.strip_prefix("u+"))
        && let Some(c) = hex4(hex).and_then(char::from_u32)
    {
        return (Some(c.to_string()), 7);
    }
    if let Some(rest) = tail.strip_prefix("M+").or_else(|| tail.strip_prefix("m+"))
        && let Some(page) = rest.chars().next().and_then(mbcs_code_page)
        && let Some(code) = hex4(&rest[1..])
        && let Some(text) = page.decode(&[(code >> 8) as u8, code as u8])
        && text.chars().count() == 1
    {
        return (Some(text), 8);
    }
    (None, 0)
}

/// Decodes a `%%` code at the start of `s`.
fn percent(s: &str, code_page: CodePage, kind: Kind) -> (Option<String>, usize) {
    let Some(code) = s.strip_prefix("%%") else {
        return (None, 0);
    };
    let symbol = match code.chars().next() {
        Some('d' | 'D') => Some('°'),
        Some('p' | 'P') => Some('±'),
        Some('c' | 'C') => Some('⌀'),
        Some('%') => Some('%'),
        _ => None,
    };
    if let Some(symbol) = symbol {
        return (Some(symbol.to_string()), 3);
    }
    if kind == Kind::Text
        && let Some(digits) = code.get(..3)
        && digits.bytes().all(|b| b.is_ascii_digit())
        && let Ok(n) = digits.parse::<u8>()
    {
        let decoded = if n < 0x80 {
            Some(char::from(n).to_string())
        } else if code_page.is_single_byte() {
            code_page.decode(&[n])
        } else {
            None
        };
        if let Some(text) = decoded {
            return (Some(text), 5);
        }
    }
    (None, 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    const CP1252: CodePage = CodePage::ANSI_1252;

    #[test]
    fn code_page_names() {
        assert_eq!(
            CodePage::from_dwgcodepage("ANSI_1252"),
            Some(CodePage(1252))
        );
        assert_eq!(
            CodePage::from_dwgcodepage(" ansi_1250 "),
            Some(CodePage(1250))
        );
        assert_eq!(CodePage::from_dwgcodepage("ANSI_932"), Some(CodePage(932)));
        assert_eq!(
            CodePage::from_dwgcodepage("ANSI_1361"),
            None,
            "Johab is unsupported"
        );
        assert_eq!(CodePage::from_dwgcodepage("DOS850"), None);
        assert_eq!(CodePage::from_dwgcodepage("ANSI_"), None);
        assert_eq!(CodePage::from_dwgcodepage(""), None);
        assert_eq!(CodePage(1251).dwgcodepage(), "ANSI_1251");
        for n in [
            874, 932, 936, 949, 950, 1250, 1251, 1252, 1253, 1254, 1255, 1256, 1257, 1258,
        ] {
            assert!(CodePage(n).encoding().is_some(), "{n}");
        }
    }

    #[test]
    fn code_pages_decode_their_characters() {
        let cases: [(u16, &[u8], &str); 9] = [
            (1250, b"\xb9\xe6", "ąć"),
            (1251, b"\xcf\xf0\xe8", "При"),
            (1252, b"W\xe4nde \xb1", "Wände ±"),
            (1253, b"\xc1\xe2", "Αβ"),
            (1254, b"\xfe\xf0", "şğ"),
            (932, b"\x94\x7a\x8a\xc7", "配管"),
            (936, b"\xc5\xe4\xb9\xdc", "配管"),
            (949, b"\xb9\xe8\xb0\xfc", "배관"),
            (950, b"\xb0\x74\xba\xde", "配管"),
        ];
        for (page, bytes, expected) in cases {
            assert_eq!(
                CodePage(page).decode(bytes).as_deref(),
                Some(expected),
                "{page}"
            );
        }
        // An incomplete double-byte character is an error, not a guess.
        assert_eq!(CodePage(932).decode(b"\x94"), None);
    }

    #[test]
    fn unicode_escapes() {
        assert_eq!(decode_text("\\U+914D\\U+7BA1", CP1252), "配管");
        assert_eq!(decode_text("x\\u+00b0y", CP1252), "x°y");
        // Malformed or surrogate escapes stay literal.
        assert_eq!(decode_text("\\U+12", CP1252), "\\U+12");
        assert_eq!(decode_text("\\U+D800", CP1252), "\\U+D800");
        assert_eq!(decode_text("C:\\Users", CP1252), "C:\\Users");
    }

    #[test]
    fn multibyte_escapes() {
        assert_eq!(decode_text("\\M+1947A", CP1252), "配");
        assert_eq!(decode_text("\\M+5C5E4", CP1252), "配");
        assert_eq!(decode_text("\\M+3B9E8", CP1252), "배");
        assert_eq!(
            decode_text("\\M+4B9E8", CP1252),
            "\\M+4B9E8",
            "Johab stays literal"
        );
        assert_eq!(decode_text("\\M+1ZZZZ", CP1252), "\\M+1ZZZZ");
    }

    #[test]
    fn percent_codes() {
        assert_eq!(decode_text("90%%d %%P0.1 %%c25", CP1252), "90° ±0.1 ⌀25");
        assert_eq!(decode_text("100%%%", CP1252), "100%");
        assert_eq!(decode_text("%%065%%176", CP1252), "A°");
        // %%nnn uses the drawing's code page.
        assert_eq!(decode_text("%%185", CodePage(1250)), "ą");
        assert_eq!(decode_text("%%185", CP1252), "¹");
        // Toggles, unknown codes and out-of-range numbers stay literal.
        assert_eq!(
            decode_text("%%uUnder%%u %%o %%x 50%", CP1252),
            "%%uUnder%%u %%o %%x 50%"
        );
        assert_eq!(decode_text("%%999", CP1252), "%%999");
        assert_eq!(decode_text("%%12", CP1252), "%%12");
        assert_eq!(decode_text("%%200", CodePage(932)), "%%200");
    }

    #[test]
    fn plain_text_is_borrowed() {
        assert!(matches!(decode_text("plain", CP1252), Cow::Borrowed(_)));
        assert!(matches!(decode_text("50%", CP1252), Cow::Borrowed(_)));
        assert!(matches!(decode_text("%%u", CP1252), Cow::Borrowed(_)));
        assert!(matches!(decode_text("%%d", CP1252), Cow::Owned(_)));
    }

    #[test]
    fn mtext_keeps_formatting_and_escaped_backslashes() {
        assert_eq!(
            decode_mtext("\\fArial|b1;Ø\\U+00D8 %%c10\\P\\\\U+0041", CP1252),
            "\\fArial|b1;ØØ ⌀10\\P\\\\U+0041"
        );
        // %%nnn is a TEXT feature.
        assert_eq!(decode_mtext("%%176", CP1252), "%%176");
    }

    #[test]
    fn escapes_at_the_end_and_multibyte_neighbours() {
        assert_eq!(decode_text("ä\\", CP1252), "ä\\");
        assert_eq!(decode_text("%%", CP1252), "%%");
        assert_eq!(decode_text("%", CP1252), "%");
        assert_eq!(decode_text("日本%%d", CP1252), "日本°");
        assert_eq!(decode_text("\\U+日本", CP1252), "\\U+日本");
        assert_eq!(decode_text("\\M+日本語語", CP1252), "\\M+日本語語");
    }
}
