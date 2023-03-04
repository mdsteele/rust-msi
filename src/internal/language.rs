// ========================================================================= //

const LANG_MASK: u16 = 0x3ff;
const SUBLANG_SHIFT: i32 = 10;

const LANG_NEUTRAL: u16 = 0x0;
const SUBLANG_NEUTRAL: u16 = 0x0;
const SUBLANG_CUSTOM_UNSPECIFIED: u16 = 0x4;

// ========================================================================= //

/// Represents a particular language/dialect.
///
/// `Language` values can be passed to `Value::from()` to produce a column
/// value that can be stored in a column with the `Language` category.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Language {
    code: u16,
}

impl Language {
    fn new(lang: u16, sublang: u16) -> Language {
        debug_assert_eq!((lang & !LANG_MASK), 0);
        Language { code: lang | (sublang << SUBLANG_SHIFT) }
    }

    /// Returns a `Language` value for the given Windows language identifier
    /// code.
    ///
    /// # Examples
    ///
    /// ```
    /// assert_eq!(msi::Language::from_code(9).code(), 9);
    /// assert_eq!(msi::Language::from_code(1033).code(), 1033);
    /// assert_eq!(msi::Language::from_code(3084).code(), 3084);
    /// ```
    pub fn from_code(code: u16) -> Language {
        Language { code }
    }

    /// Returns a `Language` value for the given RFC 5646 language tag.
    ///
    /// # Examples
    ///
    /// ```
    /// assert_eq!(msi::Language::from_tag("en").tag(), "en");
    /// assert_eq!(msi::Language::from_tag("en-US").tag(), "en-US");
    /// assert_eq!(msi::Language::from_tag("fr-CA").tag(), "fr-CA");
    /// ```
    pub fn from_tag(tag: &str) -> Language {
        let parts: Vec<&str> = tag.splitn(2, '-').collect();
        for &(lang_code, lang_tag, sublangs) in LANGUAGES.iter() {
            if lang_tag == parts[0] {
                if parts.len() > 1 {
                    for &(sublang_code, sublang_tag) in sublangs.iter() {
                        if sublang_tag == tag {
                            return Language::new(lang_code, sublang_code);
                        }
                    }
                    return Language::new(
                        lang_code,
                        SUBLANG_CUSTOM_UNSPECIFIED,
                    );
                } else {
                    return Language::new(lang_code, SUBLANG_NEUTRAL);
                }
            }
        }
        Language::new(LANG_NEUTRAL, SUBLANG_NEUTRAL)
    }

    /// Returns the Windows language identifier code for this language.
    ///
    /// # Examples
    ///
    /// ```
    /// assert_eq!(msi::Language::from_tag("en").code(), 9);
    /// assert_eq!(msi::Language::from_tag("en-US").code(), 1033);
    /// assert_eq!(msi::Language::from_tag("fr-CA").code(), 3084);
    /// ```
    pub fn code(&self) -> u16 {
        self.code
    }

    /// Returns the RFC 5646 language tag for this language.  Returns "und"
    /// (the language tag for "undetermined") if the `Language` value is not
    /// recognized.
    ///
    /// # Examples
    ///
    /// ```
    /// assert_eq!(msi::Language::from_code(9).tag(), "en");
    /// assert_eq!(msi::Language::from_code(1033).tag(), "en-US");
    /// assert_eq!(msi::Language::from_code(3084).tag(), "fr-CA");
    /// assert_eq!(msi::Language::from_code(65535).tag(), "und");
    /// ```
    pub fn tag(&self) -> &str {
        let lang_code = self.code & LANG_MASK;
        match LANGUAGES.binary_search_by_key(&lang_code, |t| t.0) {
            Ok(index) => {
                let (_, lang_tag, sublangs) = LANGUAGES[index];
                let sublang_code = self.code >> SUBLANG_SHIFT;
                match sublangs.binary_search_by_key(&sublang_code, |t| t.0) {
                    Ok(index) => sublangs[index].1,
                    Err(_) => lang_tag,
                }
            }
            Err(_) => "und",
        }
    }
}

// ========================================================================= //

type SubLanguage = (u16, &'static str);
// TODO: This table is incomplete.
const LANGUAGES: &[(u16, &str, &[SubLanguage])] = &[
    (
        0x01,
        "ar",
        &[
            (0x01, "ar-SA"),
            (0x02, "ar-IQ"),
            (0x03, "ar-EG"),
            (0x04, "ar-LY"),
            (0x05, "ar-DZ"),
            (0x06, "ar-MA"),
            (0x07, "ar-TN"),
            (0x08, "ar-OM"),
            (0x09, "ar-YE"),
            (0x0a, "ar-SY"),
            (0x0b, "ar-JO"),
            (0x0c, "ar-LB"),
            (0x0d, "ar-KW"),
            (0x0e, "ar-AE"),
            (0x0f, "ar-BH"),
            (0x10, "ar-QA"),
        ],
    ),
    (0x02, "bg", &[(0x01, "bg-BG")]),
    (0x03, "ca", &[(0x01, "ca-ES")]),
    (0x04, "zh", &[(0x03, "zh-HK"), (0x04, "zh-SG"), (0x05, "zh-MO")]),
    (0x05, "cs", &[(0x01, "cs-CZ")]),
    (0x06, "da", &[(0x01, "da-DK")]),
    (
        0x07,
        "de",
        &[
            (0x01, "de-DE"),
            (0x02, "de-CH"),
            (0x03, "de-AT"),
            (0x04, "de-LU"),
            (0x05, "de-LI"),
        ],
    ),
    (0x08, "el", &[(0x01, "el-GR")]),
    (
        0x09,
        "en",
        &[
            (0x01, "en-US"),
            (0x02, "en-GB"),
            (0x03, "en-AU"),
            (0x04, "en-CA"),
            (0x05, "en-NZ"),
            (0x06, "en-IE"),
            (0x07, "en-ZA"),
            (0x08, "en-JM"),
            (0x09, "en-CB"),
            (0x0a, "en-BZ"),
            (0x0b, "en-TT"),
            (0x0c, "en-ZW"),
            (0x0d, "en-PH"),
            (0x10, "en-IN"),
            (0x11, "en-MY"),
            (0x12, "en-SG"),
        ],
    ),
    (
        0x0a,
        "es",
        &[
            (0x01, "es-ES"),
            (0x02, "es-MX"),
            (0x04, "es-GT"),
            (0x05, "es-CR"),
            (0x06, "es-PA"),
            (0x07, "es-DO"),
            (0x08, "es-VE"),
            (0x09, "es-CO"),
            (0x0a, "es-PE"),
            (0x0b, "es-AR"),
            (0x0c, "es-EC"),
            (0x0d, "es-CL"),
            (0x0e, "es-UY"),
            (0x0f, "es-PY"),
            (0x10, "es-BO"),
            (0x11, "es-SV"),
            (0x12, "es-HN"),
            (0x13, "es-NI"),
            (0x14, "es-PR"),
            (0x15, "es-US"),
        ],
    ),
    (0x0b, "fi", &[(0x01, "fi-FI")]),
    (
        0x0c,
        "fr",
        &[
            (0x01, "fr-FR"),
            (0x02, "fr-BE"),
            (0x03, "fr-CA"),
            (0x04, "fr-CH"),
            (0x05, "fr-LU"),
            (0x06, "fr-MC"),
        ],
    ),
    (0x0d, "he", &[(0x01, "he-IL")]),
    (0x0e, "hu", &[(0x01, "hu-HU")]),
    (0x0f, "is", &[(0x01, "is-IS")]),
    (0x10, "it", &[(0x01, "it-IT"), (0x02, "it-CH")]),
    (0x11, "ja", &[(0x01, "ja-JP")]),
    (0x12, "ko", &[(0x01, "ko-KR")]),
    (0x13, "nl", &[(0x01, "nl-NL"), (0x02, "nl-BE")]),
    (0x14, "nb", &[(0x01, "nb-NO")]),
    (0x15, "pl", &[(0x01, "pl-PL")]),
    (0x16, "pt", &[(0x01, "pt-BR"), (0x02, "pt-PT")]),
    (0x17, "rm", &[(0x01, "rm-CH")]),
    (0x18, "ro", &[(0x01, "ro-RO")]),
    (0x19, "ru", &[(0x01, "ru-RU")]),
    (0x1a, "bs", &[(0x05, "bs-BA")]),
    (0x1b, "sk", &[(0x01, "sk-SK")]),
    (0x1c, "sq", &[(0x01, "sq-AL")]),
    (0x1d, "sv", &[(0x01, "sv-SE"), (0x02, "sv-FI")]),
    (0x1e, "th", &[(0x01, "th-TH")]),
    (0x1f, "tr", &[(0x01, "tr-TR")]),
    (0x20, "ur", &[(0x01, "ur-PK")]),
    (0x21, "id", &[(0x01, "id-ID")]),
    (0x22, "uk", &[(0x01, "uk-UA")]),
    (0x23, "be", &[(0x01, "be-BY")]),
    (0x24, "sl", &[(0x01, "sl-SI")]),
    (0x25, "et", &[(0x01, "et-EE")]),
    (0x26, "lv", &[(0x01, "lv-LV")]),
    (0x27, "lt", &[(0x01, "lt-LT")]),
    (0x28, "tg", &[(0x01, "tg-TJ")]),
    (0x29, "fa", &[(0x01, "fa-IR")]),
    (0x2a, "vi", &[(0x01, "vi-VN")]),
    (0x2b, "hy", &[(0x01, "hy-AM")]),
    (0x2c, "az", &[(0x01, "az-AZ")]),
    (0x2d, "eu", &[(0x01, "eu-ES")]),
    (0x2f, "mk", &[(0x01, "mk-MK")]),
    (0x32, "tn", &[(0x01, "tn-ZA"), (0x02, "tn-BW")]),
    (0x34, "xh", &[(0x01, "xh-ZA")]),
    (0x35, "zu", &[(0x01, "zu-ZA")]),
    (0x36, "af", &[(0x01, "af-ZA")]),
    (0x37, "ka", &[(0x01, "ka-GE")]),
    (0x38, "fo", &[(0x01, "fo-FO")]),
    (0x39, "hi", &[(0x01, "hi-IN")]),
    (0x3a, "mt", &[(0x01, "mt-MT")]),
    (0x3b, "smn", &[(0x09, "smn-FI")]),
    (0x3c, "ga", &[(0x02, "ga-IE")]),
    (0x3e, "ms", &[(0x01, "ms-MY"), (0x02, "ms-BN")]),
    (0x3f, "kk", &[(0x01, "kk-KZ")]),
    (0x40, "ky", &[(0x01, "ky-KG")]),
    (0x41, "sw", &[(0x01, "sw-KE")]),
    (0x42, "tk", &[(0x01, "tk-TM")]),
    (0x43, "uz", &[(0x01, "uz-UZ")]),
    (0x44, "tt", &[(0x01, "tt-RU")]),
    (0x45, "bn", &[(0x01, "bn-IN"), (0x02, "bn-BD")]),
    (0x46, "pa", &[(0x01, "pa-IN"), (0x02, "pa-PK")]),
    (0x47, "gu", &[(0x01, "gu-IN")]),
    (0x48, "or", &[(0x01, "or-IN")]),
    (0x49, "ta", &[(0x01, "ta-IN"), (0x02, "ta-LK")]),
    (0x4a, "te", &[(0x01, "te-IN")]),
    (0x4b, "kn", &[(0x01, "kn-IN")]),
    (0x4c, "ml", &[(0x01, "ml-IN")]),
    (0x4d, "as", &[(0x01, "as-IN")]),
    (0x4e, "mr", &[(0x01, "mr-IN")]),
    (0x4f, "sa", &[(0x01, "sa-IN")]),
    (0x50, "mn", &[(0x01, "mn-MN")]),
    (0x51, "bo", &[(0x01, "bo-CN")]),
    (0x52, "cy", &[(0x01, "cy-GB")]),
    (0x53, "kh", &[(0x01, "kh-KH")]),
    (0x54, "lo", &[(0x01, "lo-LA")]),
    (0x56, "gl", &[(0x01, "gl-ES")]),
    (0x57, "kok", &[(0x01, "kok-IN")]),
    (0x5a, "syr", &[(0x01, "syr-SY")]),
    (0x5b, "si", &[(0x01, "si-LK")]),
    (0x5c, "chr", &[(0x01, "chr-CHR")]),
    (0x5d, "iu", &[(0x01, "iu-CA")]),
    (0x5e, "am", &[(0x01, "am-ET")]),
    (0x5f, "tzm", &[(0x02, "tzm-DZ")]),
    (0x61, "ne", &[(0x01, "ne-NP"), (0x02, "ne-IN")]),
    (0x62, "fy", &[(0x01, "fy-NL")]),
    (0x63, "ps", &[(0x01, "ps-AF")]),
    (0x64, "tl", &[(0x01, "tl-PH")]),
    (0x65, "dv", &[(0x01, "dv-MV")]),
    (0x67, "ff", &[(0x02, "ff-SN")]),
    (0x68, "ha", &[(0x01, "ha-NG")]),
    (0x6b, "qu", &[(0x01, "qu-BO"), (0x02, "qu-EC"), (0x03, "qu-PE")]),
    (0x6c, "nso", &[(0x01, "nso-ZA")]),
    (0x6d, "ba", &[(0x01, "ba-RU")]),
    (0x6e, "lb", &[(0x01, "lb-LU")]),
    (0x6f, "kl", &[(0x01, "kl-GL")]),
    (0x70, "ig", &[(0x01, "ig-NG")]),
    (0x73, "ti", &[(0x01, "ti-ET"), (0x02, "ti-ER")]),
    (0x75, "haw", &[(0x01, "haw-US")]),
    (0x78, "ii", &[(0x01, "ii-CN")]),
    (0x7a, "arn", &[(0x01, "arn-CL")]),
    (0x7c, "moh", &[(0x01, "moh-CA")]),
    (0x7e, "br", &[(0x01, "br-FR")]),
    (0x80, "ug", &[(0x01, "ug-CN")]),
    (0x81, "mi", &[(0x01, "mi-NZ")]),
    (0x82, "oc", &[(0x01, "oc-FR")]),
    (0x83, "co", &[(0x01, "co-FR")]),
    (0x84, "gsw", &[(0x01, "gsw-FR")]),
    (0x85, "sah", &[(0x01, "sah-RU")]),
    (0x86, "qut", &[(0x01, "qut-GT")]),
    (0x87, "rw", &[(0x01, "rw-RW")]),
    (0x88, "wo", &[(0x01, "wo-SN")]),
    (0x8c, "prs", &[(0x01, "prs-AF")]),
    (0x92, "ku", &[(0x01, "ku-IQ")]),
];

// ========================================================================= //

#[cfg(test)]
mod tests {
    use super::LANGUAGES;
    use std::collections::HashSet;

    #[test]
    fn lang_codes_are_unique() {
        let mut codes = HashSet::<u16>::new();
        for &(lang_code, _, _) in LANGUAGES.iter() {
            assert!(
                !codes.contains(&lang_code),
                "Language table repeats lang code 0x{lang_code:02x}"
            );
            codes.insert(lang_code);
        }
    }

    #[test]
    fn lang_tags_are_unique() {
        let mut tags = HashSet::<&str>::new();
        for &(_, lang_tag, _) in LANGUAGES.iter() {
            assert!(
                !tags.contains(&lang_tag),
                "Language table repeats lang tag {lang_tag:?}"
            );
            tags.insert(lang_tag);
        }
    }

    #[test]
    fn lang_codes_are_sorted() {
        let mut prev_code: u16 = 0;
        for &(lang_code, _, _) in LANGUAGES.iter() {
            assert!(
                lang_code >= prev_code,
                "LANGUAGES table is not sorted, 0x{lang_code:02x} is out of \
                 place"
            );
            prev_code = lang_code;
        }
    }

    #[test]
    fn sublang_codes_are_unique() {
        for &(lang_code, lang_tag, sublangs) in LANGUAGES.iter() {
            let mut codes = HashSet::<u16>::new();
            for &(sublang_code, _) in sublangs.iter() {
                assert!(
                    !codes.contains(&sublang_code),
                    "Sublanguage table for 0x{lang_code:02x} ({lang_tag}) \
                     repeats sublang code 0x{sublang_code:02x}"
                );
                codes.insert(sublang_code);
            }
        }
    }

    #[test]
    fn sublang_tags_are_unique() {
        for &(lang_code, lang_tag, sublangs) in LANGUAGES.iter() {
            let mut tags = HashSet::<&str>::new();
            for &(_, sublang_tag) in sublangs.iter() {
                assert!(
                    !tags.contains(&sublang_tag),
                    "Sublanguage table for 0x{lang_code:02x} ({lang_tag}) \
                     repeats sublang tag {sublang_tag:?}"
                );
                tags.insert(sublang_tag);
            }
        }
    }

    #[test]
    fn sublang_codes_are_sorted() {
        for &(lang_code, lang_tag, sublangs) in LANGUAGES.iter() {
            let mut prev_code: u16 = 0;
            for &(sublang_code, sublang_tag) in sublangs.iter() {
                assert!(
                    sublang_code >= prev_code,
                    "Sublanguage table for 0x{lang_code:02x} ({lang_tag}) \
                     is not sorted; 0x{sublang_code:02x} ({sublang_tag}) is \
                     out of place"
                );
                prev_code = sublang_code;
            }
        }
    }

    #[test]
    fn sublang_tags_start_with_lang_tag() {
        for &(_, lang_tag, sublangs) in LANGUAGES.iter() {
            for &(_, sublang_tag) in sublangs.iter() {
                assert!(
                    sublang_tag.starts_with(&format!("{lang_tag}-")),
                    "{sublang_tag:?} is not a sublanguage of {lang_tag:?}"
                );
            }
        }
    }
}

// ========================================================================= //
