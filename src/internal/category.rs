use std::fmt;
use std::io;
use std::str;
use uuid::Uuid;

// ========================================================================= //

/// Indicates the format of a string-typed database column.
///
/// This list of categories comes from the [column data
/// types](https://docs.microsoft.com/en-us/windows/win32/msi/column-data-types)
/// listed in the MSI docs.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Category {
    /// An unrestricted text string.
    ///
    /// For more details, see the [MSI
    /// docs](https://docs.microsoft.com/en-us/windows/win32/msi/text) for this
    /// data type.
    ///
    /// # Examples
    ///
    /// ```
    /// assert!(msi::Category::Text.validate("Hello, World!"));
    /// ```
    Text,
    /// A text string containing no lowercase letters.
    ///
    /// For more details, see the [MSI
    /// docs](https://docs.microsoft.com/en-us/windows/win32/msi/uppercase) for
    /// this data type.
    ///
    /// # Examples
    ///
    /// ```
    /// // Valid:
    /// assert!(msi::Category::UpperCase.validate("HELLO, WORLD!"));
    /// // Invalid:
    /// assert!(!msi::Category::UpperCase.validate("Hello, World!"));
    /// ```
    UpperCase,
    /// A text string containing no uppercase letters.
    ///
    /// For more details, see the [MSI
    /// docs](https://docs.microsoft.com/en-us/windows/win32/msi/lowercase) for
    /// this data type.
    ///
    /// # Examples
    ///
    /// ```
    /// // Valid:
    /// assert!(msi::Category::LowerCase.validate("hello, world!"));
    /// // Invalid:
    /// assert!(!msi::Category::LowerCase.validate("Hello, World!"));
    /// ```
    LowerCase,
    /// A signed 16-bit integer.
    ///
    /// For more details, see the [MSI
    /// docs](https://docs.microsoft.com/en-us/windows/win32/msi/integer) for
    /// this data type.
    ///
    /// # Examples
    ///
    /// ```
    /// // Valid:
    /// assert!(msi::Category::Integer.validate("32767"));
    /// assert!(msi::Category::Integer.validate("-47"));
    /// // Invalid:
    /// assert!(!msi::Category::Integer.validate("40000"));
    /// ```
    Integer,
    /// A signed 32-bit integer.
    ///
    /// For more details, see the [MSI
    /// docs](https://docs.microsoft.com/en-us/windows/win32/msi/doubleinteger)
    /// for this data type.
    ///
    /// # Examples
    ///
    /// ```
    /// // Valid:
    /// assert!(msi::Category::DoubleInteger.validate("2147483647"));
    /// assert!(msi::Category::DoubleInteger.validate("-99999"));
    /// // Invalid:
    /// assert!(!msi::Category::DoubleInteger.validate("3000000000"));
    /// ```
    DoubleInteger,
    /// Stores a civil datetime, with a 2-second resolution.
    ///
    /// For more details, see the [MSI
    /// docs](https://docs.microsoft.com/en-us/windows/win32/msi/time-date) for
    /// this data type.
    TimeDate,
    /// A string identifier (such as a table or column name).  May only contain
    /// alphanumerics, underscores, and periods, and must start with a letter
    /// or underscore.
    ///
    /// For more details, see the [MSI
    /// docs](https://docs.microsoft.com/en-us/windows/win32/msi/identifier)
    /// for this data type.
    ///
    /// # Examples
    ///
    /// ```
    /// // Valid:
    /// assert!(msi::Category::Identifier.validate("HelloWorld"));
    /// assert!(msi::Category::Identifier.validate("_99.Bottles"));
    /// // Invalid:
    /// assert!(!msi::Category::Identifier.validate("$HELLO"));
    /// assert!(!msi::Category::Identifier.validate("3.14159"));
    /// ```
    Identifier,
    /// A string that is either an identifier (see above), or a reference to an
    /// environment variable (which consists of a `%` character followed by an
    /// identifier).
    ///
    /// For more details, see the [MSI
    /// docs](https://docs.microsoft.com/en-us/windows/win32/msi/property) for
    /// this data type.
    ///
    /// # Examples
    ///
    /// ```
    /// // Valid:
    /// assert!(msi::Category::Property.validate("HelloWorld"));
    /// assert!(msi::Category::Property.validate("%HelloWorld"));
    /// // Invalid:
    /// assert!(!msi::Category::Property.validate("%"));
    /// assert!(!msi::Category::Property.validate("Hello%World"));
    /// ```
    Property,
    /// The name of a file or directory.
    ///
    /// For more details, see the [MSI
    /// docs](https://docs.microsoft.com/en-us/windows/win32/msi/filename) for
    /// this data type.
    Filename,
    /// A filename that can contain shell glob wildcards.
    ///
    /// For more details, see the [MSI
    /// docs](https://docs.microsoft.com/en-us/windows/win32/msi/wildcardfilename)
    /// for this data type.
    WildCardFilename,
    /// A string containing an absolute filepath.
    ///
    /// For more details, see the [MSI
    /// docs](https://docs.microsoft.com/en-us/windows/win32/msi/path) for this
    /// data type.
    Path,
    /// A string containing a semicolon-separated list of absolute filepaths.
    ///
    /// For more details, see the [MSI
    /// docs](https://docs.microsoft.com/en-us/windows/win32/msi/paths) for
    /// this data type.
    Paths,
    /// A string containing an absolute or relative filepath.
    ///
    /// For more details, see the [MSI
    /// docs](https://docs.microsoft.com/en-us/windows/win32/msi/anypath) for
    /// this data type.
    AnyPath,
    /// A string containing either a filename or an identifier.
    ///
    /// For more details, see the [MSI
    /// docs](https://docs.microsoft.com/en-us/windows/win32/msi/defaultdir)
    /// for this data type.
    DefaultDir,
    /// A string containing a registry path.
    ///
    /// For more details, see the [MSI
    /// docs](https://docs.microsoft.com/en-us/windows/win32/msi/regpath) for
    /// this data type.
    RegPath,
    /// A string containing special formatting escapes, such as environment
    /// variables.
    ///
    /// For more details, see the [MSI
    /// docs](https://docs.microsoft.com/en-us/windows/win32/msi/formatted) for
    /// this data type.
    Formatted,
    /// A security descriptor definition language (SDDL) text string written in
    /// valid [Security Descriptor String
    /// Format](https://docs.microsoft.com/en-us/windows/win32/secauthz/security-descriptor-string-format).
    ///
    /// For more details, see the [MSI
    /// docs](https://docs.microsoft.com/en-us/windows/win32/msi/formattedsddltext)
    /// for this data type.
    FormattedSddlText,
    /// Like `Formatted`, but allows additional escapes.
    ///
    /// For more details, see the [MSI
    /// docs](https://docs.microsoft.com/en-us/windows/win32/msi/template) for
    /// this data type.
    Template,
    /// A string represeting a boolean predicate.
    ///
    /// For more details, see the [MSI
    /// docs](https://docs.microsoft.com/en-us/windows/win32/msi/condition) for
    /// this data type.
    Condition,
    /// A hyphenated, uppercase GUID string, enclosed in curly braces.
    ///
    /// For more details, see the [MSI
    /// docs](https://docs.microsoft.com/en-us/windows/win32/msi/guid) for
    /// this data type.
    ///
    /// # Examples
    ///
    /// ```
    /// // Valid:
    /// assert!(msi::Category::Guid.validate(
    ///     "{34AB5C53-9B30-4E14-AEF0-2C1C7BA826C0}"));
    /// // Invalid:
    /// assert!(!msi::Category::Guid.validate(
    ///     "{34AB5C539B304E14AEF02C1C7BA826C0}")); // Must be hyphenated
    /// assert!(!msi::Category::Guid.validate(
    ///     "{34ab5c53-9b30-4e14-aef0-2c1c7ba826c0}")); // Must be uppercase
    /// assert!(!msi::Category::Guid.validate(
    ///     "34AB5C53-9B30-4E14-AEF0-2C1C7BA826C0")); // Must have braces
    /// assert!(!msi::Category::Guid.validate(
    ///     "{HELLOWO-RLDH-ELLO-WORL-DHELLOWORLD0}"));
    /// ```
    Guid,
    /// A string containing a version number.  The string must consist of at
    /// most four period-separated numbers, with the value of each number being
    /// at most 65535.
    ///
    /// For more details, see the [MSI
    /// docs](https://docs.microsoft.com/en-us/windows/win32/msi/version) for
    /// this data type.
    ///
    /// # Examples
    ///
    /// ```
    /// // Valid:
    /// assert!(msi::Category::Version.validate("1"));
    /// assert!(msi::Category::Version.validate("1.22"));
    /// assert!(msi::Category::Version.validate("1.22.3"));
    /// assert!(msi::Category::Version.validate("1.22.3.444"));
    /// // Invalid:
    /// assert!(!msi::Category::Version.validate("1.99999"));
    /// assert!(!msi::Category::Version.validate(".12"));
    /// assert!(!msi::Category::Version.validate("1.2.3.4.5"));
    /// ```
    Version,
    /// A string containing a comma-separated list of decimal language ID
    /// numbers.
    ///
    /// For more details, see the [MSI
    /// docs](https://docs.microsoft.com/en-us/windows/win32/msi/language) for
    /// this data type.
    ///
    /// # Examples
    ///
    /// ```
    /// // Valid:
    /// assert!(msi::Category::Language.validate("1033"));
    /// assert!(msi::Category::Language.validate("1083,2107,3131"));
    /// // Invalid:
    /// assert!(!msi::Category::Language.validate(""));
    /// assert!(!msi::Category::Language.validate("1083,2107,3131,"));
    /// assert!(!msi::Category::Language.validate("1083,,3131"));
    /// assert!(!msi::Category::Language.validate("en-US"));
    /// ```
    Language,
    /// A string that refers to a binary data stream.
    ///
    /// For more details, see the [MSI
    /// docs](https://docs.microsoft.com/en-us/windows/win32/msi/binary) for
    /// this data type.
    Binary,
    /// A string that refers to a custom source.
    ///
    /// For more details, see the [MSI
    /// docs](https://docs.microsoft.com/en-us/windows/win32/msi/customsource)
    /// for this data type.
    CustomSource,
    /// A string that refers to a cabinet.  If it starts with a `#` character,
    /// then the rest of the string is an identifier (see above) indicating a
    /// data stream in the package where the cabinet is stored.  Otherwise, the
    /// string is a short filename (at most eight characters, a period, and a
    /// three-character extension).
    ///
    /// For more details, see the [MSI
    /// docs](https://docs.microsoft.com/en-us/windows/win32/msi/cabinet) for
    /// this data type.
    ///
    /// # Examples
    ///
    /// ```
    /// // Valid:
    /// assert!(msi::Category::Cabinet.validate("hello.txt"));
    /// assert!(msi::Category::Cabinet.validate("#HelloWorld"));
    /// // Invalid:
    /// assert!(!msi::Category::Cabinet.validate("longfilename.long"));
    /// assert!(!msi::Category::Cabinet.validate("#123.456"));
    /// ```
    Cabinet,
    /// A string that refers to a shortcut.
    ///
    /// For more details, see the [MSI
    /// docs](https://docs.microsoft.com/en-us/windows/win32/msi/shortcut) for
    /// this data type.
    Shortcut,
}

impl Category {
    pub(crate) fn all() -> Vec<Category> {
        vec![
            Category::Text,
            Category::UpperCase,
            Category::LowerCase,
            Category::Integer,
            Category::DoubleInteger,
            Category::TimeDate,
            Category::Identifier,
            Category::Property,
            Category::Filename,
            Category::WildCardFilename,
            Category::Path,
            Category::Paths,
            Category::AnyPath,
            Category::DefaultDir,
            Category::RegPath,
            Category::Formatted,
            Category::FormattedSddlText,
            Category::Template,
            Category::Condition,
            Category::Guid,
            Category::Version,
            Category::Language,
            Category::Binary,
            Category::CustomSource,
            Category::Cabinet,
            Category::Shortcut,
        ]
    }

    pub(crate) fn as_str(&self) -> &'static str {
        match *self {
            Category::AnyPath => "AnyPath",
            Category::Binary => "Binary",
            Category::Cabinet => "Cabinet",
            Category::Condition => "Condition",
            Category::CustomSource => "CustomSource",
            Category::DefaultDir => "DefaultDir",
            Category::DoubleInteger => "DoubleInteger",
            Category::Filename => "Filename",
            Category::Formatted => "Formatted",
            Category::FormattedSddlText => "FormattedSDDLText",
            Category::Guid => "GUID",
            Category::Identifier => "Identifier",
            Category::Integer => "Integer",
            Category::Language => "Language",
            Category::LowerCase => "LowerCase",
            Category::Path => "Path",
            Category::Paths => "Paths",
            Category::Property => "Property",
            Category::RegPath => "RegPath",
            Category::Shortcut => "Shortcut",
            Category::Template => "Template",
            Category::Text => "Text",
            Category::TimeDate => "TimeDate",
            Category::UpperCase => "UpperCase",
            Category::Version => "Version",
            Category::WildCardFilename => "WildCardFilename",
        }
    }

    /// Returns true if the given string is valid to store in a database column
    /// with this category.
    pub fn validate(&self, string: &str) -> bool {
        match *self {
            Category::Text => true,
            Category::UpperCase => {
                !string.chars().any(|chr| chr.is_ascii_lowercase())
            }
            Category::LowerCase => {
                !string.chars().any(|chr| chr.is_ascii_uppercase())
            }
            Category::Integer => string.parse::<i16>().is_ok(),
            Category::DoubleInteger => string.parse::<i32>().is_ok(),
            Category::Identifier => {
                string.starts_with(|chr: char| {
                    chr.is_ascii_alphabetic() || chr == '_'
                }) && !string.contains(|chr: char| {
                    !(chr.is_ascii_alphanumeric() || chr == '_' || chr == '.')
                })
            }
            Category::Property => {
                let substr = if let Some(substr) = string.strip_prefix('%') {
                    substr
                } else {
                    string
                };
                Category::Identifier.validate(substr)
            }
            Category::Guid => {
                string.len() == 38
                    && string.starts_with('{')
                    && string.ends_with('}')
                    && !string.chars().any(|chr| chr.is_ascii_lowercase())
                    && Uuid::parse_str(&string[1..37]).is_ok()
            }
            Category::Version => {
                let mut parts = string.split('.');
                parts.clone().count() <= 4
                    && parts.all(|part| part.parse::<u16>().is_ok())
            }
            Category::Language => {
                let mut parts = string.split(',');
                parts.all(|part| part.parse::<u16>().is_ok())
            }
            Category::Cabinet => {
                if let Some(substr) = string.strip_prefix('#') {
                    Category::Identifier.validate(substr)
                } else {
                    let mut parts: Vec<&str> =
                        string.rsplitn(2, '.').collect();
                    parts.reverse();
                    !parts.is_empty()
                        && !parts[0].is_empty()
                        && parts[0].len() <= 8
                        && (parts.len() < 2 || parts[1].len() <= 3)
                }
            }
            // TODO: Validate other categories.
            _ => true,
        }
    }
}

impl fmt::Display for Category {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        self.as_str().fmt(formatter)
    }
}

impl str::FromStr for Category {
    type Err = io::Error;

    fn from_str(string: &str) -> io::Result<Category> {
        match string {
            "AnyPath" => Ok(Category::AnyPath),
            "Binary" => Ok(Category::Binary),
            "Cabinet" => Ok(Category::Cabinet),
            "Condition" => Ok(Category::Condition),
            "CustomSource" => Ok(Category::CustomSource),
            "DefaultDir" => Ok(Category::DefaultDir),
            "DoubleInteger" => Ok(Category::DoubleInteger),
            "Filename" => Ok(Category::Filename),
            "Formatted" => Ok(Category::Formatted),
            "FormattedSDDLText" => Ok(Category::FormattedSddlText),
            "FormattedSddlText" => Ok(Category::FormattedSddlText),
            "GUID" => Ok(Category::Guid),
            "Guid" => Ok(Category::Guid),
            "Identifier" => Ok(Category::Identifier),
            "Integer" => Ok(Category::Integer),
            "Language" => Ok(Category::Language),
            "LowerCase" => Ok(Category::LowerCase),
            "Path" => Ok(Category::Path),
            "Paths" => Ok(Category::Paths),
            "Property" => Ok(Category::Property),
            "RegPath" => Ok(Category::RegPath),
            "Shortcut" => Ok(Category::Shortcut),
            "Template" => Ok(Category::Template),
            "Text" => Ok(Category::Text),
            "TimeDate" => Ok(Category::TimeDate),
            "UpperCase" => Ok(Category::UpperCase),
            "Version" => Ok(Category::Version),
            "WildCardFilename" => Ok(Category::WildCardFilename),
            _ => invalid_data!("Invalid category: {:?}", string),
        }
    }
}

// ========================================================================= //

#[cfg(test)]
mod tests {
    use super::Category;

    #[test]
    fn category_string_round_trip() {
        for category in Category::all() {
            assert_eq!(
                category.to_string().parse::<Category>().unwrap(),
                category
            );
        }
    }
}

// ========================================================================= //
