use std::{fmt, i16, i32};
use std::io;
use std::str;
use uuid::Uuid;

// ========================================================================= //

/// Indicates the format of a string-typed database column.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ColumnCategory {
    /// An unrestricted text string.
    ///
    /// # Examples
    ///
    /// ```
    /// assert!(msi::ColumnCategory::Text.validate("Hello, World!"));
    /// ```
    Text,
    /// A text string containing no lowercase letters.
    ///
    /// # Examples
    ///
    /// ```
    /// assert!(msi::ColumnCategory::UpperCase.validate("HELLO, WORLD!"));
    /// assert!(!msi::ColumnCategory::UpperCase.validate("Hello, World!"));
    /// ```
    UpperCase,
    /// A text string containing no uppercase letters.
    ///
    /// # Examples
    ///
    /// ```
    /// assert!(msi::ColumnCategory::LowerCase.validate("hello, world!"));
    /// assert!(!msi::ColumnCategory::LowerCase.validate("Hello, World!"));
    /// ```
    LowerCase,
    /// A signed 16-bit integer.
    ///
    /// # Examples
    ///
    /// ```
    /// assert!(msi::ColumnCategory::Integer.validate("32767"));
    /// assert!(msi::ColumnCategory::Integer.validate("-47"));
    /// assert!(!msi::ColumnCategory::Integer.validate("40000"));
    /// ```
    Integer,
    /// A signed 32-bit integer.
    ///
    /// # Examples
    ///
    /// ```
    /// assert!(msi::ColumnCategory::DoubleInteger.validate("2147483647"));
    /// assert!(msi::ColumnCategory::DoubleInteger.validate("-99999"));
    /// assert!(!msi::ColumnCategory::DoubleInteger.validate("3000000000"));
    /// ```
    DoubleInteger,
    /// A string identifier (such as a table or column name).  May only contain
    /// alphanumerics, underscores, and periods, and must start with a letter
    /// or underscore.
    ///
    /// # Examples
    ///
    /// ```
    /// assert!(msi::ColumnCategory::Identifier.validate("HelloWorld"));
    /// assert!(msi::ColumnCategory::Identifier.validate("_99.Bottles"));
    /// assert!(!msi::ColumnCategory::Identifier.validate("3.14159"));
    /// ```
    Identifier,
    /// A string that is either an identifier (see above), or a reference to an
    /// environment variable (which consists of a `%` character followed by an
    /// identifier).
    ///
    /// # Examples
    ///
    /// ```
    /// assert!(msi::ColumnCategory::Property.validate("HelloWorld"));
    /// assert!(msi::ColumnCategory::Property.validate("%HelloWorld"));
    /// assert!(!msi::ColumnCategory::Property.validate("Hello%World"));
    /// ```
    Property,
    /// The name of a file or directory.
    Filename,
    /// A filename that can contain shell glob wildcards.
    WildCardFilename,
    /// A string containing an absolute filepath.
    Path,
    /// A string containing a semicolon-separated list of absolute filepaths.
    Paths,
    /// A string containing an absolute or relative filepath.
    AnyPath,
    /// A string containing either a filename or an identifier.
    DefaultDir,
    /// A string containing a registry path.
    RegPath,
    /// A string containing special formatting escapes, such as environment
    /// variables.
    Formatted,
    /// Unknown.
    KeyFormatted,
    /// Like `Formatted`, but allows additional escapes.
    Template,
    /// A string represeting a boolean predicate.
    Condition,
    /// A hyphenated, uppercase GUID string, enclosed in curly braces.
    ///
    /// # Examples
    ///
    /// ```
    /// assert!(msi::ColumnCategory::Guid.validate(
    ///     "{34AB5C53-9B30-4E14-AEF0-2C1C7BA826C0}"));
    /// assert!(!msi::ColumnCategory::Guid.validate(
    ///     "{34AB5C539B304E14AEF02C1C7BA826C0}")); // Must be hyphenated
    /// assert!(!msi::ColumnCategory::Guid.validate(
    ///     "{34ab5c53-9b30-4e14-aef0-2c1c7ba826c0}")); // Must be uppercase
    /// assert!(!msi::ColumnCategory::Guid.validate(
    ///     "34AB5C53-9B30-4E14-AEF0-2C1C7BA826C0")); // Must have braces
    /// assert!(!msi::ColumnCategory::Guid.validate(
    ///     "{HELLOWO-RLDH-ELLO-WORL-DHELLOWORLD0}"));
    /// ```
    Guid,
    /// A string containing a version number.  The string must consist of at
    /// most four period-separated numbers, with each number being at most
    /// 65535.
    ///
    /// # Examples
    ///
    /// ```
    /// assert!(msi::ColumnCategory::Version.validate("1"));
    /// assert!(msi::ColumnCategory::Version.validate("1.22"));
    /// assert!(msi::ColumnCategory::Version.validate("1.22.3"));
    /// assert!(msi::ColumnCategory::Version.validate("1.22.3.444"));
    /// assert!(!msi::ColumnCategory::Version.validate("1.99999"));
    /// assert!(!msi::ColumnCategory::Version.validate(".12"));
    /// assert!(!msi::ColumnCategory::Version.validate("1.2.3.4.5"));
    /// ```
    Version,
    /// A string containing a comma-separated list of deciaml language ID
    /// numbers.
    Language,
    /// A string that refers to a binary data stream.
    Binary,
    /// A string that refers to a custom source.
    CustomSource,
    /// A string that refers to a cabinet.  If it starts with a `#` character,
    /// then the rest of the string is an identifier (see above) indicating a
    /// data stream in the package where the cabinet is stored.  Otherwise, the
    /// string is a short filename (at most eight characters, a period, and a
    /// three-character extension).
    ///
    /// # Examples
    ///
    /// ```
    /// assert!(msi::ColumnCategory::Cabinet.validate("hello.txt"));
    /// assert!(msi::ColumnCategory::Cabinet.validate("#HelloWorld"));
    /// assert!(!msi::ColumnCategory::Cabinet.validate("longfilename.long"));
    /// assert!(!msi::ColumnCategory::Cabinet.validate("#123.456"));
    /// ```
    Cabinet,
    /// A string that refers to a shortcut.
    Shortcut,
    /// A string containing a URL.
    Url,
}

impl ColumnCategory {
    pub(crate) fn all() -> Vec<ColumnCategory> {
        vec![
            ColumnCategory::Text,
            ColumnCategory::UpperCase,
            ColumnCategory::LowerCase,
            ColumnCategory::Integer,
            ColumnCategory::DoubleInteger,
            ColumnCategory::Identifier,
            ColumnCategory::Property,
            ColumnCategory::Filename,
            ColumnCategory::WildCardFilename,
            ColumnCategory::Path,
            ColumnCategory::Paths,
            ColumnCategory::AnyPath,
            ColumnCategory::DefaultDir,
            ColumnCategory::RegPath,
            ColumnCategory::Formatted,
            ColumnCategory::KeyFormatted,
            ColumnCategory::Template,
            ColumnCategory::Condition,
            ColumnCategory::Guid,
            ColumnCategory::Version,
            ColumnCategory::Language,
            ColumnCategory::Binary,
            ColumnCategory::CustomSource,
            ColumnCategory::Cabinet,
            ColumnCategory::Shortcut,
            ColumnCategory::Url,
        ]
    }

    pub(crate) fn as_str(&self) -> &'static str {
        match *self {
            ColumnCategory::AnyPath => "AnyPath",
            ColumnCategory::Binary => "Binary",
            ColumnCategory::Cabinet => "Cabinet",
            ColumnCategory::Condition => "Condition",
            ColumnCategory::CustomSource => "CustomSource",
            ColumnCategory::DefaultDir => "DefaultDir",
            ColumnCategory::DoubleInteger => "DoubleInteger",
            ColumnCategory::Filename => "Filename",
            ColumnCategory::Formatted => "Formatted",
            ColumnCategory::Guid => "GUID",
            ColumnCategory::Identifier => "Identifier",
            ColumnCategory::Integer => "Integer",
            ColumnCategory::KeyFormatted => "KeyFormatted",
            ColumnCategory::Language => "Language",
            ColumnCategory::LowerCase => "LowerCase",
            ColumnCategory::Path => "Path",
            ColumnCategory::Paths => "Paths",
            ColumnCategory::Property => "Property",
            ColumnCategory::RegPath => "RegPath",
            ColumnCategory::Shortcut => "Shortcut",
            ColumnCategory::Template => "Template",
            ColumnCategory::Text => "Text",
            ColumnCategory::UpperCase => "UpperCase",
            ColumnCategory::Url => "URL",
            ColumnCategory::Version => "Version",
            ColumnCategory::WildCardFilename => "WildCardFilename",
        }
    }

    /// Returns true if the given string is valid to store in a database column
    /// with this category.
    pub fn validate(&self, string: &str) -> bool {
        match *self {
            ColumnCategory::Text => true,
            ColumnCategory::UpperCase => {
                !string.chars().any(|chr| chr >= 'a' && chr <= 'z')
            }
            ColumnCategory::LowerCase => {
                !string.chars().any(|chr| chr >= 'A' && chr <= 'Z')
            }
            ColumnCategory::Integer => string.parse::<i16>().is_ok(),
            ColumnCategory::DoubleInteger => string.parse::<i32>().is_ok(),
            ColumnCategory::Identifier => {
                string.starts_with(|chr| {
                                       chr >= 'A' && chr <= 'Z' ||
                                           chr >= 'a' && chr <= 'z' ||
                                           chr == '_'
                                   }) &&
                    !string.contains(|chr| {
                                         !(chr >= 'A' && chr <= 'Z' ||
                                               chr >= 'a' && chr <= 'z' ||
                                               chr >= '0' && chr <= '9' ||
                                               chr == '_' ||
                                               chr == '.')
                                     })
            }
            ColumnCategory::Property => {
                let substr = if string.starts_with('%') {
                    &string[1..]
                } else {
                    string
                };
                ColumnCategory::Identifier.validate(substr)
            }
            ColumnCategory::Guid => {
                string.len() == 38 && string.starts_with('{') &&
                    string.ends_with('}') &&
                    !string.chars().any(|chr| chr >= 'a' && chr <= 'z') &&
                    Uuid::parse_str(&string[1..37]).is_ok()
            }
            ColumnCategory::Version => {
                let mut parts = string.split('.');
                parts.clone().count() <= 4 &&
                    parts.all(|part| part.parse::<u16>().is_ok())
            }
            ColumnCategory::Cabinet => {
                if string.starts_with('#') {
                    ColumnCategory::Identifier.validate(&string[1..])
                } else {
                    let mut parts: Vec<&str> =
                        string.rsplitn(2, '.').collect();
                    parts.reverse();
                    parts.len() > 0 && parts[0].len() > 0 &&
                        parts[0].len() <= 8 &&
                        (parts.len() < 2 || parts[1].len() <= 3)
                }
            }
            // TODO: Validate other categories.
            _ => true,
        }
    }
}

impl fmt::Display for ColumnCategory {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        self.as_str().fmt(formatter)
    }
}

impl str::FromStr for ColumnCategory {
    type Err = io::Error;

    fn from_str(string: &str) -> io::Result<ColumnCategory> {
        match string {
            "AnyPath" => Ok(ColumnCategory::AnyPath),
            "Binary" => Ok(ColumnCategory::Binary),
            "Cabinet" => Ok(ColumnCategory::Cabinet),
            "Condition" => Ok(ColumnCategory::Condition),
            "CustomSource" => Ok(ColumnCategory::CustomSource),
            "DefaultDir" => Ok(ColumnCategory::DefaultDir),
            "DoubleInteger" => Ok(ColumnCategory::DoubleInteger),
            "Filename" => Ok(ColumnCategory::Filename),
            "Formatted" => Ok(ColumnCategory::Formatted),
            "GUID" => Ok(ColumnCategory::Guid),
            "Guid" => Ok(ColumnCategory::Guid),
            "Identifier" => Ok(ColumnCategory::Identifier),
            "Integer" => Ok(ColumnCategory::Integer),
            "KeyFormatted" => Ok(ColumnCategory::KeyFormatted),
            "Language" => Ok(ColumnCategory::Language),
            "LowerCase" => Ok(ColumnCategory::LowerCase),
            "Path" => Ok(ColumnCategory::Path),
            "Paths" => Ok(ColumnCategory::Paths),
            "Property" => Ok(ColumnCategory::Property),
            "RegPath" => Ok(ColumnCategory::RegPath),
            "Shortcut" => Ok(ColumnCategory::Shortcut),
            "Template" => Ok(ColumnCategory::Template),
            "Text" => Ok(ColumnCategory::Text),
            "UpperCase" => Ok(ColumnCategory::UpperCase),
            "URL" => Ok(ColumnCategory::Url),
            "Url" => Ok(ColumnCategory::Url),
            "Version" => Ok(ColumnCategory::Version),
            "WildCardFilename" => Ok(ColumnCategory::WildCardFilename),
            _ => invalid_data!("Invalid category: {:?}", string),
        }
    }
}

// ========================================================================= //

#[cfg(test)]
mod tests {
    use super::ColumnCategory;

    #[test]
    fn category_string_round_trip() {
        for category in ColumnCategory::all() {
            assert_eq!(category
                           .to_string()
                           .parse::<ColumnCategory>()
                           .unwrap(),
                       category);
        }
    }
}

// ========================================================================= //
