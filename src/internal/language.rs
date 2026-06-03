use std::fmt;

const LANG_MASK: u16 = 0x3ff;
const SUBLANG_SHIFT: u8 = 10;

const LANG_NEUTRAL: u16 = 0x0;
const SUBLANG_NEUTRAL: u16 = 0x0;

/// Represents a Windows language identifier.
///
/// A language identifier is a standard international numeric abbreviation for
/// the [language] in a country or geographical region. Each language has a
/// unique language identifier (data type `LANGID`), a 16-bit value that
/// consists  of a primary language identifier and a sub-language identifier.
/// For details of language identifiers, see
/// [Language Identifier Constants and Strings].
///
/// A [`LanguageId`] can be passed to [`Value::from()`] to produce a column
/// value that can be stored in a column with the `Language` category.
///
/// [language]: https://learn.microsoft.com/windows/win32/intl/locales-and-languages
/// [Language Identifier Constants and Strings]: https://learn.microsoft.com/windows/win32/intl/language-identifier-constants-and-strings
/// [`Value::from()`]: crate::Value
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct LanguageId(u16);

impl LanguageId {
    /// Creates a new [`LanguageId`] from a primary language identifier and a
    /// sub-language identifier.
    ///
    /// See [`MAKELANGID`].
    ///
    /// [`MAKELANGID`]: https://learn.microsoft.com/windows/win32/api/winnt/nf-winnt-makelangid
    ///
    /// # Examples
    ///
    /// ```
    /// use msi::LanguageId;
    ///
    /// assert_eq!(LanguageId::new(0x0C, 0x03).tag(), "fr-CA");
    /// assert_eq!(LanguageId::new(0x09, 0x01).tag(), "en-US");
    /// ```
    #[must_use]
    pub const fn new(primary_language_id: u16, sub_language_id: u16) -> Self {
        Self(primary_language_id | (sub_language_id << SUBLANG_SHIFT))
    }

    /// Creates a [`LanguageId`] from a [Windows Language ID].
    ///
    /// [Windows Language ID]: https://learn.microsoft.com/openspecs/windows_protocols/ms-lcid/70feba9f-294e-491e-b6eb-56532684c37f
    ///
    /// # Examples
    ///
    /// ```
    /// use msi::LanguageId;
    ///
    /// assert_eq!(LanguageId::from_id(9).id(), 9);
    /// assert_eq!(LanguageId::from_id(1033).id(), 1033);
    /// assert_eq!(LanguageId::from_id(3084).id(), 3084);
    /// ```
    #[must_use]
    #[inline]
    pub const fn from_id(id: u16) -> Self {
        Self(id)
    }

    /// Creates a [`LanguageId`] from a [Windows Language Code Identifier (LCID)].
    ///
    /// [Windows Language Code Identifier (LCID)]: https://learn.microsoft.com/openspecs/windows_protocols/ms-lcid/70feba9f-294e-491e-b6eb-56532684c37f
    #[expect(
        clippy::cast_possible_truncation,
        reason = "LCID must be truncated to get the language ID"
    )]
    #[must_use]
    pub const fn from_lcid(lcid: u32) -> Self {
        Self::from_id(lcid as u16)
    }

    /// Creates a [`Language`] for a given RFC 5646 language tag.
    ///
    /// # Examples
    ///
    /// ```
    /// use msi::LanguageId;
    ///
    /// assert_eq!(LanguageId::from_tag("en").tag(), "en");
    /// assert_eq!(LanguageId::from_tag("en-US").tag(), "en-US");
    /// assert_eq!(LanguageId::from_tag("fr-CA").tag(), "fr-CA");
    /// ```
    #[must_use]
    pub fn from_tag(language_tag: &str) -> Self {
        LANGUAGES
            .iter()
            .copied()
            .find_map(|(id, tag)| {
                (tag == language_tag).then(|| Self::from_id(id))
            })
            .unwrap_or_default()
    }

    /// Returns the raw language ID for this [`LanguageId`].
    ///
    /// # Examples
    ///
    /// ```
    /// use msi::LanguageId;
    ///
    /// assert_eq!(LanguageId::from_tag("en").id(), 9);
    /// assert_eq!(LanguageId::from_tag("en-US").id(), 1033);
    /// assert_eq!(LanguageId::from_tag("fr-CA").id(), 3084);
    /// ```
    #[must_use]
    #[inline]
    pub const fn id(&self) -> u16 {
        self.0
    }

    /// Returns the primary language ID of this [`LanguageId`].
    ///
    /// # Examples
    ///
    /// ```
    /// use msi::LanguageId;
    ///
    /// assert_eq!(LanguageId::from_id(1033).primary_language_id(), 9);
    /// ```
    #[must_use]
    pub const fn primary_language_id(&self) -> u16 {
        self.0 & LANG_MASK
    }

    /// Returns the sub-language identifier.
    ///
    /// # Examples
    ///
    /// ```
    /// use msi::LanguageId;
    ///
    /// assert_eq!(LanguageId::from_id(1033).sub_language_id(), 1);
    /// ```
    #[must_use]
    pub const fn sub_language_id(&self) -> u8 {
        (self.0 >> SUBLANG_SHIFT) as u8
    }

    /// Returns the RFC 5646 language tag for this language.
    ///
    /// If the language is not recognized, "und" (the language tag for
    /// "undetermined") is returned.
    ///
    /// # Examples
    ///
    /// ```
    /// use msi::LanguageId;
    ///
    /// assert_eq!(LanguageId::from_id(9).tag(), "en");
    /// assert_eq!(LanguageId::from_id(1033).tag(), "en-US");
    /// assert_eq!(LanguageId::from_id(3084).tag(), "fr-CA");
    /// assert_eq!(LanguageId::from_id(65535).tag(), "und");
    /// ```
    #[must_use]
    pub fn tag(&self) -> &str {
        LANGUAGES
            .binary_search_by_key(&self.0, |&(id, _)| id)
            .map_or("und", |index| LANGUAGES[index].1)
    }
}

impl Default for LanguageId {
    fn default() -> Self {
        Self::new(LANG_NEUTRAL, SUBLANG_NEUTRAL)
    }
}

impl fmt::Display for LanguageId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.id().fmt(f)
    }
}

impl fmt::LowerHex for LanguageId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::LowerHex::fmt(&self.0, f)
    }
}

impl fmt::UpperHex for LanguageId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::UpperHex::fmt(&self.0, f)
    }
}

impl From<u16> for LanguageId {
    fn from(code: u16) -> Self {
        Self::from_id(code)
    }
}

impl From<LanguageId> for u16 {
    fn from(language_id: LanguageId) -> Self {
        language_id.id()
    }
}

/// All non-reserved language IDs defined in section 2.2 of the
/// [Windows Language Code Identifier (LCID) Reference] revision 16.0 mapped to
/// their respective RFC 5646 language tag.
///
/// [Windows Language Code Identifier (LCID) Reference]: https://learn.microsoft.com/openspecs/windows_protocols/ms-lcid/70feba9f-294e-491e-b6eb-56532684c37f
const LANGUAGES: &[(u16, &str); 454] = &[
    (0x0001, "ar"),             // Arabic
    (0x0002, "bg"),             // Bulgarian
    (0x0003, "ca"),             // Catalan
    (0x0004, "zh-Hans"),        // Chinese (Simplified)
    (0x0005, "cs"),             // Czech
    (0x0006, "da"),             // Danish
    (0x0007, "de"),             // German
    (0x0008, "el"),             // Greek
    (0x0009, "en"),             // English
    (0x000A, "es"),             // Spanish
    (0x000B, "fi"),             // Finnish
    (0x000C, "fr"),             // French
    (0x000D, "he"),             // Hebrew
    (0x000E, "hu"),             // Hungarian
    (0x000F, "is"),             // Icelandic
    (0x0010, "it"),             // Italian
    (0x0011, "ja"),             // Japanese
    (0x0012, "ko"),             // Korean
    (0x0013, "nl"),             // Dutch
    (0x0014, "no"),             // Norwegian (Bokmal)
    (0x0015, "pl"),             // Polish
    (0x0016, "pt"),             // Portuguese
    (0x0017, "rm"),             // Romansh
    (0x0018, "ro"),             // Romanian
    (0x0019, "ru"),             // Russian
    (0x001A, "hr"),             // Croatian
    (0x001B, "sk"),             // Slovak
    (0x001C, "sq"),             // Albanian
    (0x001D, "sv"),             // Swedish
    (0x001E, "th"),             // Thai
    (0x001F, "tr"),             // Turkish
    (0x0020, "ur"),             // Urdu
    (0x0021, "id"),             // Indonesian
    (0x0022, "uk"),             // Ukrainian
    (0x0023, "be"),             // Belarusian
    (0x0024, "sl"),             // Slovenian
    (0x0025, "et"),             // Estonian
    (0x0026, "lv"),             // Latvian
    (0x0027, "lt"),             // Lithuanian
    (0x0028, "tg"),             // Tajik (Cyrillic)
    (0x0029, "fa"),             // Persian
    (0x002A, "vi"),             // Vietnamese
    (0x002B, "hy"),             // Armenian
    (0x002C, "az"),             // Azerbaijani (Latin)
    (0x002D, "eu"),             // Basque
    (0x002E, "hsb"),            // Upper Sorbian
    (0x002F, "mk"),             // Macedonian
    (0x0030, "st"),             // Sotho
    (0x0031, "ts"),             // Tsonga
    (0x0032, "tn"),             // Setswana
    (0x0033, "ve"),             // Venda
    (0x0034, "xh"),             // Xhosa
    (0x0035, "zu"),             // Zulu
    (0x0036, "af"),             // Afrikaans
    (0x0037, "ka"),             // Georgian
    (0x0038, "fo"),             // Faroese
    (0x0039, "hi"),             // Hindi
    (0x003A, "mt"),             // Maltese
    (0x003B, "se"),             // Sami (Northern)
    (0x003C, "ga"),             // Irish
    (0x003D, "yi"),             // Yiddish
    (0x003E, "ms"),             // Malay
    (0x003F, "kk"),             // Kazakh
    (0x0040, "ky"),             // Kyrgyz
    (0x0041, "sw"),             // Kiswahili
    (0x0042, "tk"),             // Turkmen
    (0x0043, "uz"),             // Uzbek (Latin)
    (0x0044, "tt"),             // Tatar
    (0x0045, "bn"),             // Bangla
    (0x0046, "pa"),             // Punjabi
    (0x0047, "gu"),             // Gujarati
    (0x0048, "or"),             // Odia
    (0x0049, "ta"),             // Tamil
    (0x004A, "te"),             // Telugu
    (0x004B, "kn"),             // Kannada
    (0x004C, "ml"),             // Malayalam
    (0x004D, "as"),             // Assamese
    (0x004E, "mr"),             // Marathi
    (0x004F, "sa"),             // Sanskrit
    (0x0050, "mn"),             // Mongolian (Cyrillic)
    (0x0051, "bo"),             // Tibetan
    (0x0052, "cy"),             // Welsh
    (0x0053, "km"),             // Khmer
    (0x0054, "lo"),             // Lao
    (0x0055, "my"),             // Burmese
    (0x0056, "gl"),             // Galician
    (0x0057, "kok"),            // Konkani
    (0x0058, "mni"),            // Manipuri
    (0x0059, "sd"),             // Sindhi
    (0x005A, "syr"),            // Syriac
    (0x005B, "si"),             // Sinhala
    (0x005C, "chr"),            // Cherokee
    (0x005D, "iu"),             // Inuktitut
    (0x005E, "am"),             // Amharic
    (0x005F, "tzm"),            // Central Atlas Tamazight
    (0x0060, "ks"),             // Kashmiri
    (0x0061, "ne"),             // Nepali
    (0x0062, "fy"),             // Frisian
    (0x0063, "ps"),             // Pashto
    (0x0064, "fil"),            // Filipino
    (0x0065, "dv"),             // Divehi
    (0x0066, "bin"),            // Bini
    (0x0067, "ff"),             // Fulah
    (0x0068, "ha"),             // Hausa
    (0x0069, "ibb"),            // Ibibio
    (0x006A, "yo"),             // Yoruba
    (0x006B, "quz"),            // Quechua
    (0x006C, "nso"),            // Northern Sotho
    (0x006D, "ba"),             // Bashkir
    (0x006E, "lb"),             // Luxembourgish
    (0x006F, "kl"),             // Greenlandic
    (0x0070, "ig"),             // Igbo
    (0x0071, "kr"),             // Kanuri
    (0x0072, "om"),             // Oromo
    (0x0073, "ti"),             // Tigrinya
    (0x0074, "gn"),             // Guarani
    (0x0075, "haw"),            // Hawaiian
    (0x0076, "la"),             // Latin
    (0x0077, "so"),             // Somali
    (0x0078, "ii"),             // Sichuan Yi
    (0x0079, "pap"),            // Papiamento
    (0x007A, "arn"),            // Mapuche
    (0x007C, "moh"),            // Mohawk
    (0x007E, "br"),             // Breton
    (0x0080, "ug"),             // Uyghur
    (0x0081, "mi"),             // Maori
    (0x0082, "oc"),             // Occitan
    (0x0083, "co"),             // Corsican
    (0x0084, "gsw"),            // Alsatian
    (0x0085, "sah"),            // Sakha
    (0x0086, "qut"),            // K'iche
    (0x0087, "rw"),             // Kinyarwanda
    (0x0088, "wo"),             // Wolof
    (0x008C, "prs"),            // Dari
    (0x0091, "gd"),             // Scottish Gaelic
    (0x0092, "ku"),             // Central Kurdish
    (0x0093, "quc"),            // K'iche
    (0x0401, "ar-SA"),          // Arabic - Saudi Arabia
    (0x0402, "bg-BG"),          // Bulgarian - Bulgaria
    (0x0403, "ca-ES"),          // Catalan - Spain
    (0x0404, "zh-TW"),          // Chinese (Traditional) - Taiwan
    (0x0405, "cs-CZ"),          // Czech - Czech Republic
    (0x0406, "da-DK"),          // Danish - Denmark
    (0x0407, "de-DE"),          // German -  Germany
    (0x0408, "el-GR"),          // Greek - Greece
    (0x0409, "en-US"),          // English - United States
    (0x040A, "es-ES_tradnl"),   // Spanish - Spain
    (0x040B, "fi-FI"),          // Finnish - Finland
    (0x040C, "fr-FR"),          // French - France
    (0x040D, "he-IL"),          // Hebrew - Israel
    (0x040E, "hu-HU"),          // Hungarian - Hungary
    (0x040F, "is-IS"),          // Icelandic - Iceland
    (0x0410, "it-IT"),          // Italian - Italy
    (0x0411, "ja-JP"),          // Japanese - Japan
    (0x0412, "ko-KR"),          // Korean - Korea
    (0x0413, "nl-NL"),          // Dutch - Netherlands
    (0x0414, "nb-NO"),          // Norwegian (Bokmål) - Norway
    (0x0415, "pl-PL"),          // Polish - Poland
    (0x0416, "pt-BR"),          // Portuguese - Brazil
    (0x0417, "rm-CH"),          // Romansh - Switzerland
    (0x0418, "ro-RO"),          // Romanian - Romania
    (0x0419, "ru-RU"),          // Russian - Russia
    (0x041A, "hr-HR"),          // Croatian - Croatia
    (0x041B, "sk-SK"),          // Slovak - Slovakia
    (0x041C, "sq-AL"),          // Albanian - Albania
    (0x041D, "sv-SE"),          // Swedish - Sweden
    (0x041E, "th-TH"),          // Thai - Thailand
    (0x041F, "tr-TR"),          // Turkish - Türkiye
    (0x0420, "ur-PK"),          // Urdu - Pakistan
    (0x0421, "id-ID"),          // Indonesian - Indonesia
    (0x0422, "uk-UA"),          // Ukrainian - Ukraine
    (0x0423, "be-BY"),          // Belarusian - Belarus
    (0x0424, "sl-SI"),          // Slovenian - Slovenia
    (0x0425, "et-EE"),          // Estonian - Estonia
    (0x0426, "lv-LV"),          // Latvian - Latvia
    (0x0427, "lt-LT"),          // Lithuanian - Lithuania
    (0x0428, "tg-Cyrl-TJ"),     // Tajik (Cyrillic) - Tajikistan
    (0x0429, "fa-IR"),          // Persian - Iran
    (0x042A, "vi-VN"),          // Vietnamese - Vietnam
    (0x042B, "hy-AM"),          // Armenian - Armenia
    (0x042C, "az-Latn-AZ"),     // Azerbaijani (Latin) - Azerbaijan
    (0x042D, "eu-ES"),          // Basque - Spain
    (0x042E, "hsb-DE"),         // Upper Sorbian - Germany
    (0x042F, "mk-MK"),          // Macedonian - North Macedonia
    (0x0430, "st-ZA"),          // Sotho - South Africa
    (0x0431, "ts-ZA"),          // Tsonga - South Africa
    (0x0432, "tn-ZA"),          // Setswana - South Africa
    (0x0433, "ve-ZA"),          // Venda - South Africa
    (0x0434, "xh-ZA"),          // Xhosa - South Africa
    (0x0435, "zu-ZA"),          // Zulu - South Africa
    (0x0436, "af-ZA"),          // Afrikaans - South Africa
    (0x0437, "ka-GE"),          // Georgian - Georgia
    (0x0438, "fo-FO"),          // Faroese - Faroe Islands
    (0x0439, "hi-IN"),          // Hindi - India
    (0x043A, "mt-MT"),          // Maltese - Malta
    (0x043B, "se-NO"),          // Sami (Northern) - Norway
    (0x043D, "yi-001"),         // Yiddish - World
    (0x043E, "ms-MY"),          // Malay - Malaysia
    (0x043F, "kk-KZ"),          // Kazakh - Kazakhstan
    (0x0440, "ky-KG"),          // Kyrgyz - Kyrgyzstan
    (0x0441, "sw-KE"),          // Kiswahili - Kenya
    (0x0442, "tk-TM"),          // Turkmen - Turkmenistan
    (0x0443, "uz-Latn-UZ"),     // Uzbek (Latin) - Uzbekistan
    (0x0444, "tt-RU"),          // Tatar - Russia
    (0x0445, "bn-IN"),          // Bangla - India
    (0x0446, "pa-IN"),          // Punjabi - India
    (0x0447, "gu-IN"),          // Gujarati - India
    (0x0448, "or-IN"),          // Odia - India
    (0x0449, "ta-IN"),          // Tamil - India
    (0x044A, "te-IN"),          // Telugu - India
    (0x044B, "kn-IN"),          // Kannada - India
    (0x044C, "ml-IN"),          // Malayalam - India
    (0x044D, "as-IN"),          // Assamese - India
    (0x044E, "mr-IN"),          // Marathi - India
    (0x044F, "sa-IN"),          // Sanskrit - India
    (0x0450, "mn-MN"),          // Mongolian (Cyrillic) - Mongolia
    (0x0451, "bo-CN"),          // Tibetan - People's Republic of China
    (0x0452, "cy-GB"),          // Welsh - United Kingdom
    (0x0453, "km-KH"),          // Khmer - Cambodia
    (0x0454, "lo-LA"),          // Lao - Laos
    (0x0455, "my-MM"),          // Burmese - Myanmar
    (0x0456, "gl-ES"),          // Galician - Spain
    (0x0457, "kok-IN"),         // Konkani - India
    (0x0458, "mni-IN"),         // Manipuri - India
    (0x0459, "sd-Deva-IN"),     // Sindhi (Devanagari) - India
    (0x045A, "syr-SY"),         // Syriac - Syria
    (0x045B, "si-LK"),          // Sinhala - Sri Lanka
    (0x045C, "chr-Cher-US"),    // Cherokee - United States
    (0x045D, "iu-Cans-CA"), // Inuktitut (Canadian Aboriginal Syllabics) - Canada
    (0x045E, "am-ET"),      // Amharic - Ethiopia
    (0x045F, "tzm-Arab-MA"), // Central Atlas Tamazight (Arabic) - Morocco
    (0x0460, "ks-Arab"),    // Kashmiri (Arabic)
    (0x0461, "ne-NP"),      // Nepali - Nepal
    (0x0462, "fy-NL"),      // Frisian - Netherlands
    (0x0463, "ps-AF"),      // Pashto - Afghanistan
    (0x0464, "fil-PH"),     // Filipino - Philippines
    (0x0465, "dv-MV"),      // Divehi - Maldives
    (0x0466, "bin-NG"),     // Bini - Nigeria
    (0x0467, "ff-Latn-NG"), // Fulah (Latin) - Nigeria
    (0x0468, "ha-Latn-NG"), // Hausa (Latin) - Nigeria
    (0x0469, "ibb-NG"),     // Ibibio - Nigeria
    (0x046A, "yo-NG"),      // Yoruba - Nigeria
    (0x046B, "quz-BO"),     // Quechua - Bolivia
    (0x046C, "nso-ZA"),     // Northern Sotho - South Africa
    (0x046D, "ba-RU"),      // Bashkir - Russia
    (0x046E, "lb-LU"),      // Luxembourgish - Luxembourg
    (0x046F, "kl-GL"),      // Greenlandic - Greenland
    (0x0470, "ig-NG"),      // Igbo - Nigeria
    (0x0471, "kr-Latn-NG"), // Kanuri (Latin) - Nigeria
    (0x0472, "om-ET"),      // Oromo - Ethiopia
    (0x0473, "ti-ET"),      // Tigrinya - Ethiopia
    (0x0474, "gn-PY"),      // Guarani - Paraguay
    (0x0475, "haw-US"),     // Hawaiian - United States
    (0x0476, "la-VA"),      // Latin - Vatican City
    (0x0477, "so-SO"),      // Somali - Somalia
    (0x0478, "ii-CN"),      // Sichuan Yi - People's Republic of China
    (0x0479, "pap-029"),    // Papiamento - Caribbean
    (0x047A, "arn-CL"),     // Mapuche - Chile
    (0x047C, "moh-CA"),     // Mohawk - Canada
    (0x047E, "br-FR"),      // Breton - France
    (0x0480, "ug-CN"),      // Uyghur - People's Republic of China
    (0x0481, "mi-NZ"),      // Maori - New Zealand
    (0x0482, "oc-FR"),      // Occitan - France
    (0x0483, "co-FR"),      // Corsican - France
    (0x0484, "gsw-FR"),     // Alsatian - France
    (0x0485, "sah-RU"),     // Sakha - Russia
    (0x0486, "qut-GT"),     // K'iche - Guatemala
    (0x0487, "rw-RW"),      // Kinyarwanda - Rwanda
    (0x0488, "wo-SN"),      // Wolof - Senegal
    (0x048C, "prs-AF"),     // Dari - Afghanistan
    (0x048D, "plt-MG"),     // Malagasy - Madagascar
    (0x048E, "zh-yue-HK"), // Chinese (Traditional) Cantonese - Hong Kong S.A.R.
    (0x048F, "tdd-Tale-CN"), // Tai Nüa - People's Republic of China
    (0x0490, "khb-Talu-CN"), // Lü - People's Republic of China
    (0x0491, "gd-GB"),     // Scottish Gaelic - United Kingdom
    (0x0492, "ku-Arab-IQ"), // Central Kurdish Iraq
    (0x0493, "quc-CO"),    // K'iche - Colombia
    (0x0501, "qps-ploc"),  // Pseudo Language
    (0x05FE, "qps-ploca"), // Pseudo Language (Mirrored)
    (0x0801, "ar-IQ"),     // Arabic - Iraq
    (0x0803, "ca-ES-valencia"), // Catalan - Valencia
    (0x0804, "zh-CN"),     // Chinese (Simplified) - People's Republic of China
    (0x0807, "de-CH"),     // German - Switzerland
    (0x0809, "en-GB"),     // English - United Kingdom
    (0x080A, "es-MX"),     // Spanish - Mexico
    (0x080C, "fr-BE"),     // French - Belgium
    (0x0810, "it-CH"),     // Italian - Switzerland
    (0x0811, "ja-Ploc-JP"), // Japanese (Pseudo)
    (0x0813, "nl-BE"),     // Dutch - Belgium
    (0x0814, "nn-NO"),     // Norwegian (Nynorsk) - Norway
    (0x0816, "pt-PT"),     // Portuguese - Portugal
    (0x0818, "ro-MD"),     // Romanian - Moldova
    (0x0819, "ru-MD"),     // Russian - Moldova
    (0x081A, "sr-Latn-CS"), // Serbian (Latin) - Serbia and Montenegro
    (0x081D, "sv-FI"),     // Swedish - Finland
    (0x0820, "ur-IN"),     // Urdu - India
    (0x082C, "az-Cyrl-AZ"), // Azerbaijani (Cyrillic) - Azerbaijan
    (0x082E, "dsb-DE"),    // Lower Sorbian - Germany
    (0x0832, "tn-BW"),     // Setswana - Botswana
    (0x083B, "se-SE"),     // Sami (Northern) - Sweden
    (0x083C, "ga-IE"),     // Irish - Ireland
    (0x083E, "ms-BN"),     // Malay - Brunei Darussalam
    (0x083F, "kk-Latn-KZ"), // Kazakh (Latin) - Kazakhstan
    (0x0843, "uz-Cyrl-UZ"), // Uzbek (Cyrillic) - Uzbekistan
    (0x0845, "bn-BD"),     // Bangla - Bangladesh
    (0x0846, "pa-Arab-PK"), // Punjabi (Arabic) - Pakistan
    (0x0849, "ta-LK"),     // Tamil - Sri Lanka
    (0x0850, "mn-Mong-CN"), // Mongolian (Traditional Mongolian) - People's Republic of China
    (0x0851, "bo-BT"),      // Tibetan - Bhutan
    (0x0859, "sd-Arab-PK"), // Sindhi (Arabic) - Pakistan
    (0x085D, "iu-Latn-CA"), // Inuktitut (Latin) - Canada
    (0x085F, "tzm-Latn-DZ"), // Central Atlas Tamazight (Latin) - Algeria
    (0x0860, "ks-Deva-IN"), // Kashmiri (Devanagari) - India
    (0x0861, "ne-IN"),      // Nepali - India
    (0x0867, "ff-Latn-SN"), // Fulah (Latin) - Senegal
    (0x086B, "quz-EC"),     // Quechua - Ecuador
    (0x0873, "ti-ER"),      // Tigrinya - Eritrea
    (0x09FF, "qps-plocm"),  // Pseudo Language (Mirrored)
    (0x0C01, "ar-EG"),      // Arabic - Egypt
    (0x0C04, "zh-HK"),      // Chinese (Traditional) - Hong Kong S.A.R.
    (0x0C07, "de-AT"),      // German - Austria
    (0x0C09, "en-AU"),      // English - Australia
    (0x0C0A, "es-ES"),      // Spanish - Spain
    (0x0C0C, "fr-CA"),      // French - Canada
    (0x0C1A, "sr-Cyrl-CS"), // Serbian (Cyrillic) - Serbia and Montenegro
    (0x0C3B, "se-FI"),      // Sami (Northern) - Finland
    (0x0C50, "mn-Mong-MN"), // Mongolian (Traditional Mongolian) - Mongolia
    (0x0C51, "dz-BT"),      // Dzongkha - Bhutan
    (0x0C5F, "tmz-MA"),     // Central Atlas Tamazight - Morocco
    (0x0C6B, "quz-PE"),     // Quechua - Peru
    (0x1001, "ar-LY"),      // Arabic - Libya
    (0x1004, "zh-SG"),      // Chinese (Simplified) - Singapore
    (0x1007, "de-LU"),      // German - Luxembourg
    (0x1009, "en-CA"),      // English - Canada
    (0x100A, "es-GT"),      // Spanish - Guatemala
    (0x100C, "fr-CH"),      // French - Switzerland
    (0x101A, "hr-BA"),      // Croatian (Latin) - Bosnia and Herzegovina
    (0x103B, "smj-NO"),     // Sami (Lule) - Norway
    (0x105F, "tzm-Tfng-MA"), // Central Atlas Tamazight (Tifinagh) - Morocco
    (0x1401, "ar-DZ"),      // Arabic - Algeria
    (0x1404, "zh-MO"),      // Chinese (Traditional) - Macao S.A.R.
    (0x1407, "de-LI"),      // German - Liechtenstein
    (0x1409, "en-NZ"),      // English - New Zealand
    (0x140A, "es-CR"),      // Spanish - Costa Rica
    (0x140C, "fr-LU"),      // French - Luxembourg
    (0x141A, "bs-Latn-BA"), // Bosnian (Latin) - Bosnia and Herzegovina
    (0x143B, "smj-SE"),     // Sami (Lule) - Sweden
    (0x1801, "ar-MA"),      // Arabic - Morocco
    (0x1809, "en-IE"),      // English - Ireland
    (0x180A, "es-PA"),      // Spanish - Panama
    (0x180C, "fr-MC"),      // French - Monaco
    (0x181A, "sr-Latn-BA"), // Serbian (Latin) - Bosnia and Herzegovina
    (0x183B, "sma-NO"),     // Sami (Southern) - Norway
    (0x1C01, "ar-TN"),      // Arabic - Tunisia
    (0x1C09, "en-ZA"),      // English - South Africa
    (0x1C0A, "es-DO"),      // Spanish - Dominican Republic
    (0x1C0C, "fr-029"),     // French - Caribbean
    (0x1C1A, "sr-Cyrl-BA"), // Serbian (Cyrillic) - Bosnia and Herzegovina
    (0x1C3B, "sma-SE"),     // Sami (Southern) - Sweden
    (0x2001, "ar-OM"),      // Arabic - Oman
    (0x2009, "en-JM"),      // English - Jamaica
    (0x200A, "es-VE"),      // Spanish - Venezuela
    (0x200C, "fr-RE"),      // French - Réunion
    (0x201A, "bs-Cyrl-BA"), // Bosnian (Cyrillic) - Bosnia and Herzegovina
    (0x203B, "sms-FI"),     // Sami (Skolt) - Finland
    (0x2401, "ar-YE"),      // Arabic - Yemen
    (0x2409, "en-029"),     // English - Caribbean
    (0x240A, "es-CO"),      // Spanish - Colombia
    (0x240C, "fr-CD"),      // French - Congo (DRC)
    (0x241A, "sr-Latn-RS"), // Serbian (Latin) - Serbia
    (0x243B, "smn-FI"),     // Sami (Inari) - Finland
    (0x2801, "ar-SY"),      // Arabic - Syria
    (0x2809, "en-BZ"),      // English - Belize
    (0x280A, "es-PE"),      // Spanish - Peru
    (0x280C, "fr-SN"),      // French - Senegal
    (0x281A, "sr-Cyrl-RS"), // Serbian (Cyrillic) - Serbia
    (0x2C01, "ar-JO"),      // Arabic - Jordan
    (0x2C09, "en-TT"),      // English - Trinidad and Tobago
    (0x2C0A, "es-AR"),      // Spanish - Argentina
    (0x2C0C, "fr-CM"),      // French - Cameroon
    (0x2C1A, "sr-Latn-ME"), // Serbian (Latin) - Montenegro
    (0x3001, "ar-LB"),      // Arabic - Lebanon
    (0x3009, "en-ZW"),      // English - Zimbabwe
    (0x300A, "es-EC"),      // Spanish - Ecuador
    (0x300C, "fr-CI"),      // French - Côte d'Ivoire
    (0x301A, "sr-Cyrl-ME"), // Serbian (Cyrillic) - Montenegro
    (0x3401, "ar-KW"),      // Arabic - Kuwait
    (0x3409, "en-PH"),      // English - Philippines
    (0x340A, "es-CL"),      // Spanish - Chile
    (0x340C, "fr-ML"),      // French - Mali
    (0x3801, "ar-AE"),      // Arabic - U.A.E.
    (0x3809, "en-ID"),      // English - Indonesia
    (0x380A, "es-UY"),      // Spanish - Uruguay
    (0x380C, "fr-MA"),      // French - Morocco
    (0x3C01, "ar-BH"),      // Arabic - Bahrain
    (0x3C09, "en-HK"),      // English - Hong Kong S.A.R.
    (0x3C0A, "es-PY"),      // Spanish - Paraguay
    (0x3C0C, "fr-HT"),      // French - Haiti
    (0x4001, "ar-QA"),      // Arabic - Qatar
    (0x4009, "en-IN"),      // English - India
    (0x400A, "es-BO"),      // Spanish - Bolivia
    (0x4401, "ar-Ploc-SA"), // Arabic (Pseudo) - Saudi Arabia
    (0x4409, "en-MY"),      // English - Malaysia
    (0x440A, "es-SV"),      // Spanish - El Salvador
    (0x4801, "ar-145"),     // Arabic - Western Asia
    (0x4809, "en-SG"),      // English - Singapore
    (0x480A, "es-HN"),      // Spanish - Honduras
    (0x4C09, "en-AE"),      // English - U.A.E.
    (0x4C0A, "es-NI"),      // Spanish - Nicaragua
    (0x5009, "en-BH"),      // English - Bahrain
    (0x500A, "es-PR"),      // Spanish - Puerto Rico
    (0x5409, "en-EG"),      // English - Egypt
    (0x540A, "es-US"),      // Spanish - United States
    (0x5809, "en-JO"),      // English - Jordan
    (0x580A, "es-419"),     // Spanish - Latin America and Caribbean
    (0x5C09, "en-KW"),      // English - Kuwait
    (0x5C0A, "es-CU"),      // Spanish - Cuba
    (0x6009, "en-TR"),      // English - Türkiye
    (0x6409, "en-YE"),      // English - Yemen
    (0x641A, "bs-Cyrl"),    // Bosnian (Cyrillic)
    (0x681A, "bs-Latn"),    // Bosnian (Latin)
    (0x6C1A, "sr-Cyrl"),    // Serbian (Cyrillic)
    (0x701A, "sr-Latn"),    // Serbian (Latin)
    (0x703B, "smn"),        // Sami (Inari)
    (0x742C, "az-Cyrl"),    // Azerbaijani (Cyrillic)
    (0x743B, "sms"),        // Sami (Skolt)
    (0x7804, "zh"),         // Chinese (Simplified)
    (0x7814, "nn"),         // Norwegian (Nynorsk)
    (0x781A, "bs"),         // Bosnian (Latin)
    (0x782C, "az-Latn"),    // Azerbaijani (Latin)
    (0x783B, "sma"),        // Sami (Southern)
    (0x783F, "kk-Cyrl"),    // Kazakh (Cyrillic)
    (0x7843, "uz-Cyrl"),    // Uzbek (Cyrillic)
    (0x7850, "mn-Cyrl"),    // Mongolian (Cyrillic)
    (0x785D, "iu-Cans"),    // Inuktitut (Canadian Aboriginal Syllabics)
    (0x785F, "tzm-Tfng"),   // Central Atlas Tamazight (Tifinagh)
    (0x7C04, "zh-Hant"),    // Chinese (Traditional)
    (0x7C14, "nb"),         // Norwegian (Bokmål)
    (0x7C1A, "sr"),         // Serbian
    (0x7C28, "tg-Cyrl"),    // Tajik (Cyrillic)
    (0x7C2E, "dsb"),        // Lower Sorbian
    (0x7C3B, "smj"),        // Sami (Lule)
    (0x7C3F, "kk-Latn"),    // Kazakh (Latin)
    (0x7C43, "uz-Latn"),    // Uzbek (Latin)
    (0x7C46, "pa-Arab"),    // Punjabi (Arabic)
    (0x7C50, "mn-Mong"),    // Mongolian (Traditional Mongolian)
    (0x7C59, "sd-Arab"),    // Sindhi (Arabic)
    (0x7C5C, "chr-Cher"),   // Cherokee
    (0x7C5D, "iu-Latn"),    // Inuktitut (Latin)
    (0x7C5F, "tzm-Latn"),   // Central Atlas Tamazight (Latin)
    (0x7C67, "ff-Latn"),    // Fulah (Latin)
    (0x7C68, "ha-Latn"),    // Hausa (Latin)
    (0x7C92, "ku-Arab"),    // Central Kurdish
    (0xE40C, "fr-015"),     // French - Northern Africa
];

#[cfg(test)]
mod tests {
    use super::LANGUAGES;
    use std::collections::HashSet;

    #[test]
    fn lang_codes_are_unique() {
        let mut codes = HashSet::new();
        for &(id, _) in LANGUAGES {
            assert!(
                codes.insert(id),
                "Language table repeats lang code {id:#02x}"
            );
        }
    }

    #[test]
    fn lang_tags_are_unique() {
        let mut tags = HashSet::new();
        for &(_, tag) in LANGUAGES {
            assert!(
                tags.insert(tag),
                "Language table repeats language tag {tag:?}"
            );
        }
    }

    #[test]
    fn lang_codes_are_sorted() {
        let mut prev_code: u16 = 0;
        for &(id, _) in LANGUAGES {
            assert!(
                id >= prev_code,
                "LANGUAGES table is not sorted, {id:#02x} is out of \
                 place"
            );
            prev_code = id;
        }
    }
}
