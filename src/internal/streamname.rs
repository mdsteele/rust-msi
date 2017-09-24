use std::char;

// ========================================================================= //

pub fn decode(name: &str) -> String {
    let mut output = String::new();
    for chr in name.chars() {
        let value = chr as u32;
        if value >= 0x3800 && value < 0x4800 {
            let value = value - 0x3800;
            output.push(from_b64(value & 0x3f));
            output.push(from_b64(value >> 6));
        } else if value >= 0x4800 && value < 0x4840 {
            output.push(from_b64(value - 0x4800));
        } else {
            output.push(chr);
        }
    }
    output
}

#[allow(dead_code)]
pub fn encode(name: &str) -> String {
    let mut output = String::new();
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
    if ch >= '0' && ch <= '9' {
        Some(ch as u32 - '0' as u32)
    } else if ch >= 'A' && ch <= 'Z' {
        Some(10 + ch as u32 - 'A' as u32)
    } else if ch >= 'a' && ch <= 'z' {
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
    use super::{decode, encode, from_b64, to_b64};

    #[test]
    fn to_from_b64() {
        for value in 0..64 {
            assert_eq!(to_b64(from_b64(value)), Some(value));
        }
    }

    #[test]
    fn decode_stream_name() {
        assert_eq!(decode("\u{3b3f}\u{43f2}\u{4438}\u{45b1}"), "_Columns");
        assert_eq!(decode("\u{3f7f}\u{4164}\u{422f}\u{4836}"), "_Tables");
        assert_eq!(decode("\u{3f3f}\u{4577}\u{446c}\u{3e6a}\u{44b2}\u{482f}"),
                   "_StringPool");
        assert_eq!(decode("\u{3fff}\u{43e4}\u{41ec}\u{45e4}\u{44ac}\u{4831}"),
                   "_Validation");
    }

    #[test]
    fn encode_stream_name() {
        assert_eq!(encode("_Columns"), "\u{3b3f}\u{43f2}\u{4438}\u{45b1}");
        assert_eq!(encode("_Tables"), "\u{3f7f}\u{4164}\u{422f}\u{4836}");
        assert_eq!(encode("_StringPool"),
                   "\u{3f3f}\u{4577}\u{446c}\u{3e6a}\u{44b2}\u{482f}");
        assert_eq!(encode("_Validation"),
                   "\u{3fff}\u{43e4}\u{41ec}\u{45e4}\u{44ac}\u{4831}");
    }
}

// ========================================================================= //
