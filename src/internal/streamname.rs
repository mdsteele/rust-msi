use std::char;

// ========================================================================= //

pub const DIGITAL_SIGNATURE_STREAM_NAME: &str = "\u{5}DigitalSignature";
pub const MSI_DIGITAL_SIGNATURE_EX_STREAM_NAME: &str =
    "\u{5}MsiDigitalSignatureEx";
pub const SUMMARY_INFO_STREAM_NAME: &str = "\u{5}SummaryInformation";
pub const DOCUMENT_SUMMARY_INFO_STREAM_NAME: &str =
    "\u{5}DocumentSummaryInformation";

const TABLE_PREFIX: char = '\u{4840}';

// ========================================================================= //

/// Decodes a stream name, and returns the decoded name and whether the stream
/// was a table.
pub fn decode(name: &str) -> (String, bool) {
    let mut output = String::new();
    let mut is_table = false;
    let mut chars = name.chars().peekable();
    if chars.peek() == Some(&TABLE_PREFIX) {
        is_table = true;
        chars.next();
    }
    for chr in chars {
        let value = chr as u32;
        if (0x3800..0x4800).contains(&value) {
            let value = value - 0x3800;
            output.push(from_b64(value & 0x3f));
            output.push(from_b64(value >> 6));
        } else if (0x4800..0x4840).contains(&value) {
            output.push(from_b64(value - 0x4800));
        } else {
            output.push(chr);
        }
    }
    (output, is_table)
}

/// Encodes a stream name.
pub fn encode(name: &str, is_table: bool) -> String {
    let mut output = String::new();
    if is_table {
        output.push(TABLE_PREFIX);
    }
    let mut chars = name.chars().peekable();
    while let Some(ch1) = chars.next() {
        if let Some(value1) = to_b64(ch1) {
            if let Some(&ch2) = chars.peek() {
                if let Some(value2) = to_b64(ch2) {
                    let encoded = 0x3800 + (value2 << 6) + value1;
                    output.push(char::from_u32(encoded).unwrap());
                    chars.next();
                    continue;
                }
            }
            let encoded = 0x4800 + value1;
            output.push(char::from_u32(encoded).unwrap());
        } else {
            output.push(ch1);
        }
    }
    output
}

/// Determines if a name will work as CFB stream name once encoded.
pub fn is_valid(name: &str, is_table: bool) -> bool {
    if name.is_empty() || (!is_table && name.starts_with(TABLE_PREFIX)) {
        false
    } else {
        encode(name, is_table).encode_utf16().count() <= 31
    }
}

// ========================================================================= //

fn from_b64(value: u32) -> char {
    debug_assert!(value < 64);
    if value < 10 {
        char::from_u32(value + '0' as u32).unwrap()
    } else if value < 36 {
        char::from_u32(value - 10 + 'A' as u32).unwrap()
    } else if value < 62 {
        char::from_u32(value - 36 + 'a' as u32).unwrap()
    } else if value == 62 {
        '.'
    } else {
        '_'
    }
}

fn to_b64(ch: char) -> Option<u32> {
    if ch.is_ascii_digit() {
        Some(ch as u32 - '0' as u32)
    } else if ch.is_ascii_uppercase() {
        Some(10 + ch as u32 - 'A' as u32)
    } else if ch.is_ascii_lowercase() {
        Some(36 + ch as u32 - 'a' as u32)
    } else if ch == '.' {
        Some(62)
    } else if ch == '_' {
        Some(63)
    } else {
        None
    }
}

// ========================================================================= //

#[cfg(test)]
mod tests {
    use super::{decode, encode, from_b64, is_valid, to_b64};

    #[test]
    fn to_from_b64() {
        for value in 0..64 {
            assert_eq!(to_b64(from_b64(value)), Some(value));
        }
    }

    #[test]
    fn decode_stream_name() {
        assert_eq!(
            decode("\u{4840}\u{3b3f}\u{43f2}\u{4438}\u{45b1}"),
            ("_Columns".to_string(), true)
        );
        assert_eq!(
            decode("\u{4840}\u{3f7f}\u{4164}\u{422f}\u{4836}"),
            ("_Tables".to_string(), true)
        );
        assert_eq!(
            decode("\u{44ca}\u{47b3}\u{46e8}\u{4828}"),
            ("App.exe".to_string(), false)
        );
    }

    #[test]
    fn encode_stream_name() {
        assert_eq!(
            encode("_Columns", true),
            "\u{4840}\u{3b3f}\u{43f2}\u{4438}\u{45b1}"
        );
        assert_eq!(
            encode("_Tables", true),
            "\u{4840}\u{3f7f}\u{4164}\u{422f}\u{4836}"
        );
        assert_eq!(
            encode("App.exe", false),
            "\u{44ca}\u{47b3}\u{46e8}\u{4828}"
        );
    }

    #[test]
    fn is_valid_stream_name() {
        assert!(is_valid("Icon.AppIcon.ico", false));
        assert!(is_valid("_Columns", true));
        assert!(is_valid("¿Qué pasa?", false));

        assert!(!is_valid("", false));
        assert!(!is_valid("", true));
        assert!(!is_valid("\u{4840}Stream", false));
        assert!(!is_valid(
            "ThisStringIsWayTooLongToBeAStreamName\
                           IMeanSeriouslyWhoWouldTryToUseAName\
                           ThatIsThisLongItWouldBePrettySilly",
            false
        ));
    }
}

// ========================================================================= //
