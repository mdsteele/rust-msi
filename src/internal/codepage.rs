use encoding_rs::{EncoderResult, Encoding};

// ========================================================================= //

/// A Windows code page.
///
/// Code pages are a legacy Windows mechanism for representing character
/// encodings, but are still used within the MSI file format.
///
/// Not all Windows code pages are supported by this library yet.  See
/// [Wikipedia](https://en.wikipedia.org/wiki/Windows_code_page) for a complete
/// list of valid code pages.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum CodePage {
    /// [Windows-932 (Japanese Shift JIS)](https://en.wikipedia.org/wiki/Code_page_932_(Microsoft_Windows))
    Windows932,
    /// [Windows-936 (Chinese (simplified) GBK)](https://en.wikipedia.org/wiki/Code_page_936_(Microsoft_Windows))
    Windows936,
    /// [Windows-949 (Korean Unified Hangul Code)](https://en.wikipedia.org/wiki/Unified_Hangul_Code)
    Windows949,
    /// [Windows-950 (Chinese (traditional) Big5)](https://en.wikipedia.org/wiki/Code_page_950)
    Windows950,
    /// [Windows-951 (Chinese (traditional) Big5-HKSCS)](https://en.wikipedia.org/wiki/Hong_Kong_Supplementary_Character_Set#Microsoft_Windows)
    Windows951,
    /// [Windows-1250 (Latin 2)](https://en.wikipedia.org/wiki/Windows-1250)
    Windows1250,
    /// [Windows-1251 (Cyrillic)](https://en.wikipedia.org/wiki/Windows-1251)
    Windows1251,
    /// [Windows-1252 (Latin 1)](https://en.wikipedia.org/wiki/Windows-1252)
    Windows1252,
    /// [Windows-1253 (Greek)](https://en.wikipedia.org/wiki/Windows-1253)
    Windows1253,
    /// [Windows-1254 (Turkish)](https://en.wikipedia.org/wiki/Windows-1254)
    Windows1254,
    /// [Windows-1255 (Hebrew)](https://en.wikipedia.org/wiki/Windows-1255)
    Windows1255,
    /// [Windows-1256 (Arabic)](https://en.wikipedia.org/wiki/Windows-1256)
    Windows1256,
    /// [Windows-1257 (Baltic)](https://en.wikipedia.org/wiki/Windows-1257)
    Windows1257,
    /// [Windows-1258 (Vietnamese)](https://en.wikipedia.org/wiki/Windows-1258)
    Windows1258,
    /// [Mac OS Roman](https://en.wikipedia.org/wiki/Mac_OS_Roman)
    MacintoshRoman,
    /// [Macintosh
    /// Cyrillic](https://en.wikipedia.org/wiki/Macintosh_Cyrillic_encoding)
    MacintoshCyrillic,
    /// [US-ASCII](https://en.wikipedia.org/wiki/ASCII)
    UsAscii,
    /// [ISO-8859-1 (Latin 1)](https://en.wikipedia.org/wiki/ISO-8859-1)
    Iso88591,
    /// [ISO-8859-2 (Latin 2)](https://en.wikipedia.org/wiki/ISO-8859-2)
    Iso88592,
    /// [ISO-8859-3 (South European)](https://en.wikipedia.org/wiki/ISO-8859-3)
    Iso88593,
    /// [ISO-8859-4 (North European)](https://en.wikipedia.org/wiki/ISO-8859-4)
    Iso88594,
    /// [ISO-8859-5 (Cyrillic)](https://en.wikipedia.org/wiki/ISO-8859-5)
    Iso88595,
    /// [ISO-8859-6 (Arabic)](https://en.wikipedia.org/wiki/ISO-8859-6)
    Iso88596,
    /// [ISO-8859-7 (Greek)](https://en.wikipedia.org/wiki/ISO-8859-7)
    Iso88597,
    /// [ISO-8859-8 (Hebrew)](https://en.wikipedia.org/wiki/ISO-8859-8)
    Iso88598,
    /// [UTF-8](https://en.wikipedia.org/wiki/UTF-8)
    #[default]
    Utf8,
}

impl CodePage {
    /// Returns the code page (if any) with the given ID number.
    pub fn from_id(id: i32) -> Option<CodePage> {
        match id {
            0 => Some(CodePage::default()),
            932 => Some(CodePage::Windows932),
            936 => Some(CodePage::Windows936),
            949 => Some(CodePage::Windows949),
            950 => Some(CodePage::Windows950),
            951 => Some(CodePage::Windows951),
            1250 => Some(CodePage::Windows1250),
            1251 => Some(CodePage::Windows1251),
            1252 => Some(CodePage::Windows1252),
            1253 => Some(CodePage::Windows1253),
            1254 => Some(CodePage::Windows1254),
            1255 => Some(CodePage::Windows1255),
            1256 => Some(CodePage::Windows1256),
            1257 => Some(CodePage::Windows1257),
            1258 => Some(CodePage::Windows1258),
            10000 => Some(CodePage::MacintoshRoman),
            10007 => Some(CodePage::MacintoshCyrillic),
            20127 => Some(CodePage::UsAscii),
            28591 => Some(CodePage::Iso88591),
            28592 => Some(CodePage::Iso88592),
            28593 => Some(CodePage::Iso88593),
            28594 => Some(CodePage::Iso88594),
            28595 => Some(CodePage::Iso88595),
            28596 => Some(CodePage::Iso88596),
            28597 => Some(CodePage::Iso88597),
            28598 => Some(CodePage::Iso88598),
            65001 => Some(CodePage::Utf8),
            _ => None,
        }
    }

    /// Returns the ID number used within Windows to represent this code page.
    pub fn id(&self) -> i32 {
        match *self {
            CodePage::Windows932 => 932,
            CodePage::Windows936 => 936,
            CodePage::Windows949 => 949,
            CodePage::Windows950 => 950,
            CodePage::Windows951 => 951,
            CodePage::Windows1250 => 1250,
            CodePage::Windows1251 => 1251,
            CodePage::Windows1252 => 1252,
            CodePage::Windows1253 => 1253,
            CodePage::Windows1254 => 1254,
            CodePage::Windows1255 => 1255,
            CodePage::Windows1256 => 1256,
            CodePage::Windows1257 => 1257,
            CodePage::Windows1258 => 1258,
            CodePage::MacintoshRoman => 10000,
            CodePage::MacintoshCyrillic => 10007,
            CodePage::UsAscii => 20127,
            CodePage::Iso88591 => 28591,
            CodePage::Iso88592 => 28592,
            CodePage::Iso88593 => 28593,
            CodePage::Iso88594 => 28594,
            CodePage::Iso88595 => 28595,
            CodePage::Iso88596 => 28596,
            CodePage::Iso88597 => 28597,
            CodePage::Iso88598 => 28598,
            CodePage::Utf8 => 65001,
        }
    }

    /// Returns a human-readable name for this code page.
    pub fn name(&self) -> &str {
        match *self {
            CodePage::Windows932 => "Windows Japanese Shift JIS",
            CodePage::Windows936 => "Windows Chinese (simplified) GBK",
            CodePage::Windows949 => "Windows Korean Unified Hangul Code",
            CodePage::Windows950 => "Windows Chinese (traditional) Big5",
            CodePage::Windows951 => "Windows Chinese (traditional) Big5-HKSCS",
            CodePage::Windows1250 => "Windows Latin 2",
            CodePage::Windows1251 => "Windows Cyrillic",
            CodePage::Windows1252 => "Windows Latin 1",
            CodePage::Windows1253 => "Windows Greek",
            CodePage::Windows1254 => "Windows Turkish",
            CodePage::Windows1255 => "Windows Hebrew",
            CodePage::Windows1256 => "Windows Arabic",
            CodePage::Windows1257 => "Windows Baltic",
            CodePage::Windows1258 => "Windows Vietnamese",
            CodePage::MacintoshRoman => "Mac OS Roman",
            CodePage::MacintoshCyrillic => "Macintosh Cyrillic",
            CodePage::UsAscii => "US-ASCII",
            CodePage::Iso88591 => "ISO Latin 1",
            CodePage::Iso88592 => "ISO Latin 2",
            CodePage::Iso88593 => "ISO Latin 3",
            CodePage::Iso88594 => "ISO Latin 4",
            CodePage::Iso88595 => "ISO Latin/Cyrillic",
            CodePage::Iso88596 => "ISO Latin/Arabic",
            CodePage::Iso88597 => "ISO Latin/Greek",
            CodePage::Iso88598 => "ISO Latin/Hebrew",
            CodePage::Utf8 => "UTF-8",
        }
    }

    /// Decodes a byte array into a string, using this code page.  Invalid
    /// characters will be replaced with a Unicode replacement character
    /// (U+FFFD).
    pub fn decode(&self, bytes: &[u8]) -> String {
        if *self == CodePage::UsAscii {
            ascii_decode(bytes)
        } else {
            self.encoding().decode(bytes).0.into_owned()
        }
    }

    /// Encodes a string into a byte array, using this code page.  For
    /// non-Unicode code pages, any characters that cannot be represented will
    /// be replaced with a code-page-specific replacement character (typically
    /// `'?'`).
    pub fn encode(&self, string: &str) -> Vec<u8> {
        if *self == CodePage::UsAscii {
            ascii_encode(string)
        } else {
            let mut encoder = self.encoding().new_encoder();
            let mut bytes = Vec::new();
            let mut buffer = [0; 1024];
            let mut total_read = 0;
            loop {
                let (result, read, written) = encoder
                    .encode_from_utf8_without_replacement(
                        &string[total_read..],
                        &mut buffer[..],
                        true,
                    );
                total_read += read;
                bytes.extend_from_slice(&buffer[..written]);
                match result {
                    EncoderResult::InputEmpty => {
                        break;
                    }
                    EncoderResult::OutputFull => {
                        continue;
                    }
                    EncoderResult::Unmappable(_) => {
                        bytes.push(b'?');
                    }
                }
            }
            bytes
        }
    }

    fn encoding(&self) -> &'static Encoding {
        match *self {
            CodePage::Windows932 => encoding_rs::EUC_JP,
            CodePage::Windows936 => encoding_rs::BIG5,
            CodePage::Windows949 => encoding_rs::EUC_KR,
            CodePage::Windows950 | CodePage::Windows951 => encoding_rs::BIG5,
            CodePage::Windows1250 => encoding_rs::WINDOWS_1250,
            CodePage::Windows1251 => encoding_rs::WINDOWS_1251,
            CodePage::Windows1252 => encoding_rs::WINDOWS_1252,
            CodePage::Windows1253 => encoding_rs::WINDOWS_1253,
            CodePage::Windows1254 => encoding_rs::WINDOWS_1254,
            CodePage::Windows1255 => encoding_rs::WINDOWS_1255,
            CodePage::Windows1256 => encoding_rs::WINDOWS_1256,
            CodePage::Windows1257 => encoding_rs::WINDOWS_1257,
            CodePage::Windows1258 => encoding_rs::WINDOWS_1258,
            CodePage::MacintoshRoman => &encoding_rs::MACINTOSH_INIT,
            CodePage::MacintoshCyrillic => &encoding_rs::X_MAC_CYRILLIC_INIT,
            CodePage::Iso88591 => encoding_rs::WINDOWS_1252,
            CodePage::Iso88592 => encoding_rs::ISO_8859_2,
            CodePage::Iso88593 => encoding_rs::ISO_8859_3,
            CodePage::Iso88594 => encoding_rs::ISO_8859_4,
            CodePage::Iso88595 => encoding_rs::ISO_8859_5,
            CodePage::Iso88596 => encoding_rs::ISO_8859_6,
            CodePage::Iso88597 => encoding_rs::ISO_8859_7,
            CodePage::Iso88598 => encoding_rs::ISO_8859_8,
            CodePage::Utf8 => encoding_rs::UTF_8,
            CodePage::UsAscii => unreachable!(), // handled in encode/decode methods
        }
    }
}

fn ascii_encode(string: &str) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(string.len());
    for ch in string.chars() {
        if ch.is_ascii() {
            bytes.push(ch as u8);
        } else {
            bytes.push(b'?');
        }
    }
    bytes
}

fn ascii_decode(bytes: &[u8]) -> String {
    let mut chars = Vec::new();
    for ch in bytes {
        if ch.is_ascii() {
            chars.push(*ch as char);
        } else {
            chars.push('\u{FFFD}');
        }
    }
    chars.into_iter().collect()
}

// ========================================================================= //

#[cfg(test)]
mod tests {
    use super::CodePage;

    #[test]
    fn id_round_trip() {
        let codepages = &[
            CodePage::Windows932,
            CodePage::Windows936,
            CodePage::Windows949,
            CodePage::Windows950,
            CodePage::Windows951,
            CodePage::Windows1250,
            CodePage::Windows1251,
            CodePage::Windows1252,
            CodePage::Windows1253,
            CodePage::Windows1254,
            CodePage::Windows1255,
            CodePage::Windows1256,
            CodePage::Windows1257,
            CodePage::Windows1258,
            CodePage::MacintoshRoman,
            CodePage::MacintoshCyrillic,
            CodePage::Iso88591,
            CodePage::Iso88592,
            CodePage::Iso88593,
            CodePage::Iso88594,
            CodePage::Iso88595,
            CodePage::Iso88596,
            CodePage::Iso88597,
            CodePage::Iso88598,
            CodePage::Utf8,
        ];
        for &codepage in codepages.iter() {
            assert_eq!(CodePage::from_id(codepage.id()), Some(codepage));
        }
    }

    #[test]
    fn decode_string() {
        assert_eq!(
            &CodePage::Windows1252.decode(b"\xbfQu\xe9 pasa?"),
            "¿Qué pasa?"
        );
        assert_eq!(
            &CodePage::MacintoshRoman.decode(b"\xc0Qu\x8e pasa?"),
            "¿Qué pasa?"
        );
        assert_eq!(
            &CodePage::Utf8.decode(b"\xc2\xbfQu\xc3\xa9 pasa?"),
            "¿Qué pasa?"
        );
    }

    #[test]
    fn decoding_error() {
        assert_eq!(
            &CodePage::Utf8.decode(b"Qu\xee pasa?"),
            "Qu\u{fffd} pasa?"
        );
    }

    #[test]
    fn encode_string() {
        assert_eq!(
            &CodePage::Windows1252.encode("¿Qué pasa?") as &[u8],
            b"\xbfQu\xe9 pasa?"
        );
        assert_eq!(
            &CodePage::MacintoshRoman.encode("¿Qué pasa?") as &[u8],
            b"\xc0Qu\x8e pasa?"
        );
        assert_eq!(
            &CodePage::Utf8.encode("¿Qué pasa?") as &[u8],
            b"\xc2\xbfQu\xc3\xa9 pasa?"
        );
    }

    #[test]
    fn encoding_error() {
        assert_eq!(
            &CodePage::Windows1252.encode("Snowman=\u{2603}") as &[u8],
            b"Snowman=?"
        );
        assert_eq!(
            &CodePage::UsAscii.encode("¿Qué pasa?") as &[u8],
            b"?Qu? pasa?"
        );
    }
}

// ========================================================================= //
