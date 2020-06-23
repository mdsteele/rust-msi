use encoding::{self, DecoderTrap, EncoderTrap, Encoding};

// ========================================================================= //

/// A Windows code page.
///
/// Code pages are a legacy Windows mechanism for representing character
/// encodings, but are still used within the MSI file format.
///
/// Not all Windows code pages are supported by this library yet.  See
/// [Wikipedia](https://en.wikipedia.org/wiki/Windows_code_page) for a complete
/// list of valid code pages.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum CodePage {
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
    Utf8,
}

impl CodePage {
    /// Returns the code page (if any) with the given ID number.
    pub fn from_id(id: i32) -> Option<CodePage> {
        match id {
            0 => Some(CodePage::default()),
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
        self.encoding().decode(bytes, DecoderTrap::Replace).unwrap()
    }

    /// Encodes a string into a byte array, using this code page.  For
    /// non-Unicode code pages, any characters that cannot be represented will
    /// be replaced with a code-page-specific replacement character (typically
    /// `'?'`).
    pub fn encode(&self, string: &str) -> Vec<u8> {
        self.encoding().encode(string, EncoderTrap::Replace).unwrap()
    }

    fn encoding(&self) -> &dyn Encoding {
        match *self {
            CodePage::Windows1250 => encoding::all::WINDOWS_1250,
            CodePage::Windows1251 => encoding::all::WINDOWS_1251,
            CodePage::Windows1252 => encoding::all::WINDOWS_1252,
            CodePage::Windows1253 => encoding::all::WINDOWS_1253,
            CodePage::Windows1254 => encoding::all::WINDOWS_1254,
            CodePage::Windows1255 => encoding::all::WINDOWS_1255,
            CodePage::Windows1256 => encoding::all::WINDOWS_1256,
            CodePage::Windows1257 => encoding::all::WINDOWS_1257,
            CodePage::Windows1258 => encoding::all::WINDOWS_1258,
            CodePage::MacintoshRoman => encoding::all::MAC_ROMAN,
            CodePage::MacintoshCyrillic => encoding::all::MAC_CYRILLIC,
            CodePage::UsAscii => encoding::all::ASCII,
            CodePage::Iso88591 => encoding::all::ISO_8859_1,
            CodePage::Iso88592 => encoding::all::ISO_8859_2,
            CodePage::Iso88593 => encoding::all::ISO_8859_3,
            CodePage::Iso88594 => encoding::all::ISO_8859_4,
            CodePage::Iso88595 => encoding::all::ISO_8859_5,
            CodePage::Iso88596 => encoding::all::ISO_8859_6,
            CodePage::Iso88597 => encoding::all::ISO_8859_7,
            CodePage::Iso88598 => encoding::all::ISO_8859_8,
            CodePage::Utf8 => encoding::all::UTF_8,
        }
    }
}

impl Default for CodePage {
    fn default() -> CodePage {
        CodePage::Utf8
    }
}

// ========================================================================= //

#[cfg(test)]
mod tests {
    use super::CodePage;

    #[test]
    fn id_round_trip() {
        let codepages = &[
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
