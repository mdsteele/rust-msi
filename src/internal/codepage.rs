use std::char;

// TODO: Use the `encoding` crate to implement this module.

// ========================================================================= //

/// A Windows code page.
///
/// Code pages are a legacy Windows mechanism for representing character
/// encodings, but are still used within the MSI file format.
///
/// Not all Windows code pages are supported by this library yet.  See
/// [Wikipedia](https://en.wikipedia.org/wiki/Windows_code_page) for a complete
/// list of valid code pages.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CodePage {
    /// [Windows-1252 (Latin)](https://en.wikipedia.org/wiki/Windows-1252)
    Windows1252,
    /// [Mac OS Roman](https://en.wikipedia.org/wiki/Mac_OS_Roman)
    MacintoshRoman,
    /// [UTF-8](https://en.wikipedia.org/wiki/UTF-8)
    Utf8,
}

impl CodePage {
    /// Returns the code page (if any) with the given ID number.
    pub fn from_id(id: u32) -> Option<CodePage> {
        match id {
            0 => Some(CodePage::Utf8),
            1252 => Some(CodePage::Windows1252),
            10000 => Some(CodePage::MacintoshRoman),
            65001 => Some(CodePage::Utf8),
            _ => None,
        }
    }

    /// Returns the ID number used within Windows to represent this code page.
    pub fn id(self) -> u32 {
        match self {
            CodePage::Windows1252 => 1252,
            CodePage::MacintoshRoman => 10000,
            CodePage::Utf8 => 65001,
        }
    }

    /// Decodes a byte array into a string, using this code page.  Invalid
    /// characters will be replaced with a Unicode replacement character
    /// (U+FFFD).
    pub fn decode(self, bytes: &[u8]) -> String {
        match self {
            CodePage::Windows1252 => {
                decode_with_table(bytes, &WINDOWS_1252_TABLE)
            }
            CodePage::MacintoshRoman => {
                decode_with_table(bytes, &MACINTOSH_ROMAN_TABLE)
            }
            CodePage::Utf8 => String::from_utf8_lossy(bytes).to_string(),
        }
    }
}

// ========================================================================= //

fn decode_with_table(bytes: &[u8], table: &[char; 128]) -> String {
    let mut output = String::new();
    for &byte in bytes.iter() {
        output.push(if byte < 0x80 {
            char::from_u32(byte as u32).unwrap()
        } else {
            table[(byte - 0x80) as usize]
        });
    }
    output
}

const WINDOWS_1252_TABLE: [char; 128] =
    ['\u{20ac}', '\u{fffd}', '\u{201a}', '\u{192}', '\u{201e}', '\u{2026}',
     '\u{2020}', '\u{2021}', '\u{2c6}', '\u{2030}', '\u{160}', '\u{2039}',
     '\u{152}', '\u{fffd}', '\u{17d}', '\u{fffd}', '\u{fffd}', '\u{2018}',
     '\u{2019}', '\u{201c}', '\u{201d}', '\u{2022}', '\u{2013}', '\u{2014}',
     '\u{2dc}', '\u{2122}', '\u{161}', '\u{203a}', '\u{153}', '\u{fffd}',
     '\u{17e}', '\u{178}', '\u{a0}', '\u{a1}', '\u{a2}', '\u{a3}', '\u{a4}',
     '\u{a5}', '\u{a6}', '\u{a7}', '\u{a8}', '\u{a9}', '\u{aa}', '\u{ab}',
     '\u{ac}', '\u{ad}', '\u{ae}', '\u{af}', '\u{b0}', '\u{b1}', '\u{b2}',
     '\u{b3}', '\u{b4}', '\u{b5}', '\u{b6}', '\u{b7}', '\u{b8}', '\u{b9}',
     '\u{ba}', '\u{bb}', '\u{bc}', '\u{bd}', '\u{be}', '\u{bf}', '\u{c0}',
     '\u{c1}', '\u{c2}', '\u{c3}', '\u{c4}', '\u{c5}', '\u{c6}', '\u{c7}',
     '\u{c8}', '\u{c9}', '\u{ca}', '\u{cb}', '\u{cc}', '\u{cd}', '\u{ce}',
     '\u{cf}', '\u{d0}', '\u{d1}', '\u{d2}', '\u{d3}', '\u{d4}', '\u{d5}',
     '\u{d6}', '\u{d7}', '\u{d8}', '\u{d9}', '\u{da}', '\u{db}', '\u{dc}',
     '\u{dd}', '\u{de}', '\u{df}', '\u{e0}', '\u{e1}', '\u{e2}', '\u{e3}',
     '\u{e4}', '\u{e5}', '\u{e6}', '\u{e7}', '\u{e8}', '\u{e9}', '\u{ea}',
     '\u{eb}', '\u{ec}', '\u{ed}', '\u{ee}', '\u{ef}', '\u{f0}', '\u{f1}',
     '\u{f2}', '\u{f3}', '\u{f4}', '\u{f5}', '\u{f6}', '\u{f7}', '\u{f8}',
     '\u{f9}', '\u{fa}', '\u{fb}', '\u{fc}', '\u{fd}', '\u{fe}', '\u{ff}'];

const MACINTOSH_ROMAN_TABLE: [char; 128] =
    ['\u{c4}', '\u{c5}', '\u{c7}', '\u{c9}', '\u{d1}', '\u{d6}', '\u{dc}',
     '\u{e1}', '\u{e0}', '\u{e2}', '\u{e4}', '\u{e3}', '\u{e5}', '\u{e7}',
     '\u{e9}', '\u{e8}', '\u{ea}', '\u{eb}', '\u{ed}', '\u{ec}', '\u{ee}',
     '\u{ef}', '\u{f1}', '\u{f3}', '\u{f2}', '\u{f4}', '\u{f6}', '\u{f5}',
     '\u{fa}', '\u{f9}', '\u{fb}', '\u{fc}', '\u{2020}', '\u{b0}', '\u{a2}',
     '\u{a3}', '\u{a7}', '\u{2022}', '\u{b6}', '\u{df}', '\u{ae}', '\u{a9}',
     '\u{2122}', '\u{b4}', '\u{a8}', '\u{2260}', '\u{c6}', '\u{d8}',
     '\u{221e}', '\u{b1}', '\u{2264}', '\u{2265}', '\u{a5}', '\u{b5}',
     '\u{2202}', '\u{2211}', '\u{220f}', '\u{3c0}', '\u{222b}', '\u{aa}',
     '\u{ba}', '\u{3a9}', '\u{e6}', '\u{f8}', '\u{bf}', '\u{a1}', '\u{ac}',
     '\u{221a}', '\u{192}', '\u{2248}', '\u{2206}', '\u{ab}', '\u{bb}',
     '\u{2026}', '\u{a0}', '\u{c0}', '\u{c3}', '\u{d5}', '\u{152}',
     '\u{153}', '\u{2013}', '\u{2014}', '\u{201c}', '\u{201d}', '\u{2018}',
     '\u{2019}', '\u{f7}', '\u{25ca}', '\u{ff}', '\u{178}', '\u{2044}',
     '\u{20ac}', '\u{2039}', '\u{203a}', '\u{fb01}', '\u{fb02}', '\u{2021}',
     '\u{b7}', '\u{201a}', '\u{201e}', '\u{2030}', '\u{c2}', '\u{ca}',
     '\u{c1}', '\u{cb}', '\u{c8}', '\u{cd}', '\u{ce}', '\u{cf}', '\u{cc}',
     '\u{d3}', '\u{d4}', '\u{f8ff}', '\u{d2}', '\u{da}', '\u{db}', '\u{d9}',
     '\u{131}', '\u{2c6}', '\u{2dc}', '\u{af}', '\u{2d8}', '\u{2d9}',
     '\u{2da}', '\u{b8}', '\u{2dd}', '\u{2db}', '\u{2c7}'];

// ========================================================================= //

#[cfg(test)]
mod tests {
    use super::CodePage;

    #[test]
    fn id_round_trip() {
        let codepages =
            &[CodePage::Windows1252, CodePage::MacintoshRoman, CodePage::Utf8];
        for &codepage in codepages.iter() {
            assert_eq!(CodePage::from_id(codepage.id()), Some(codepage));
        }
    }

    #[test]
    fn decode_string() {
        assert_eq!(&CodePage::Windows1252.decode(b"\xbfQu\xe9 pasa?"),
                   "¿Qué pasa?");
        assert_eq!(&CodePage::MacintoshRoman.decode(b"\xc0Qu\x8e pasa?"),
                   "¿Qué pasa?");
        assert_eq!(&CodePage::Utf8.decode(b"\xc2\xbfQu\xc3\xa9 pasa?"),
                   "¿Qué pasa?");
    }
}

// ========================================================================= //
