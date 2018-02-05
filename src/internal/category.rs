use std::{fmt, i16, i32};
use std::io;
use std::str;
use uuid::Uuid;

// ========================================================================= //

/// Indicates the format of a string-typed database column.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Category {
    /// An unrestricted text string.
    ///
    /// # Examples
    ///
    /// ```
    /// assert!(msi::Category::Text.validate("Hello, World!"));
    /// ```
    Text,
    /// A text string containing no lowercase letters.
    ///
    /// # Examples
    ///
    /// ```
    /// assert!(msi::Category::UpperCase.validate("HELLO, WORLD!"));
    /// assert!(!msi::Category::UpperCase.validate("Hello, World!"));
    /// ```
    UpperCase,
    /// A text string containing no uppercase letters.
    ///
    /// # Examples
    ///
    /// ```
    /// assert!(msi::Category::LowerCase.validate("hello, world!"));
    /// assert!(!msi::Category::LowerCase.validate("Hello, World!"));
    /// ```
    LowerCase,
    /// A signed 16-bit integer.
    ///
    /// # Examples
    ///
    /// ```
    /// assert!(msi::Category::Integer.validate("32767"));
    /// assert!(msi::Category::Integer.validate("-47"));
    /// assert!(!msi::Category::Integer.validate("40000"));
    /// ```
    Integer,
    /// A signed 32-bit integer.
    ///
    /// # Examples
    ///
    /// ```
    /// assert!(msi::Category::DoubleInteger.validate("2147483647"));
    /// assert!(msi::Category::DoubleInteger.validate("-99999"));
    /// assert!(!msi::Category::DoubleInteger.validate("3000000000"));
    /// ```
    DoubleInteger,
    /// A string identifier (such as a table or column name).  May only contain
    /// alphanumerics, underscores, and periods, and must start with a letter
    /// or underscore.
    ///
    /// # Examples
    ///
    /// ```
    /// assert!(msi::Category::Identifier.validate("HelloWorld"));
    /// assert!(msi::Category::Identifier.validate("_99.Bottles"));
    /// assert!(!msi::Category::Identifier.validate("3.14159"));
    /// ```
    Identifier,
    /// A string that is either an identifier (see above), or a reference to an
    /// environment variable (which consists of a `%` character followed by an
    /// identifier).
    ///
    /// # Examples
    ///
    /// ```
    /// assert!(msi::Category::Property.validate("HelloWorld"));
    /// assert!(msi::Category::Property.validate("%HelloWorld"));
    /// assert!(!msi::Category::Property.validate("Hello%World"));
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
    /// assert!(msi::Category::Guid.validate(
    ///     "{34AB5C53-9B30-4E14-AEF0-2C1C7BA826C0}"));
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
    /// most four period-separated numbers, with each number being at most
    /// 65535.
    ///
    /// # Examples
    ///
    /// ```
    /// assert!(msi::Category::Version.validate("1"));
    /// assert!(msi::Category::Version.validate("1.22"));
    /// assert!(msi::Category::Version.validate("1.22.3"));
    /// assert!(msi::Category::Version.validate("1.22.3.444"));
    /// assert!(!msi::Category::Version.validate("1.99999"));
    /// assert!(!msi::Category::Version.validate(".12"));
    /// assert!(!msi::Category::Version.validate("1.2.3.4.5"));
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
    /// assert!(msi::Category::Cabinet.validate("hello.txt"));
    /// assert!(msi::Category::Cabinet.validate("#HelloWorld"));
    /// assert!(!msi::Category::Cabinet.validate("longfilename.long"));
    /// assert!(!msi::Category::Cabinet.validate("#123.456"));
    /// ```
    Cabinet,
    /// A string that refers to a shortcut.
    Shortcut,
    /// A string containing a URL.
    Url,
}

impl Category {
    pub(crate) fn all() -> Vec<Category> {
        vec![
            Category::Text,
            Category::UpperCase,
            Category::LowerCase,
            Category::Integer,
            Category::DoubleInteger,
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
            Category::KeyFormatted,
            Category::Template,
            Category::Condition,
            Category::Guid,
            Category::Version,
            Category::Language,
            Category::Binary,
            Category::CustomSource,
            Category::Cabinet,
            Category::Shortcut,
            Category::Url,
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
            Category::Guid => "GUID",
            Category::Identifier => "Identifier",
            Category::Integer => "Integer",
            Category::KeyFormatted => "KeyFormatted",
            Category::Language => "Language",
            Category::LowerCase => "LowerCase",
            Category::Path => "Path",
            Category::Paths => "Paths",
            Category::Property => "Property",
            Category::RegPath => "RegPath",
            Category::Shortcut => "Shortcut",
            Category::Template => "Template",
            Category::Text => "Text",
            Category::UpperCase => "UpperCase",
            Category::Url => "URL",
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
                !string.chars().any(|chr| chr >= 'a' && chr <= 'z')
            }
            Category::LowerCase => {
                !string.chars().any(|chr| chr >= 'A' && chr <= 'Z')
            }
            Category::Integer => string.parse::<i16>().is_ok(),
            Category::DoubleInteger => string.parse::<i32>().is_ok(),
            Category::Identifier => {
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
            Category::Property => {
                let substr = if string.starts_with('%') {
                    &string[1..]
                } else {
                    string
                };
                Category::Identifier.validate(substr)
            }
            Category::Guid => {
                string.len() == 38 && string.starts_with('{') &&
                    string.ends_with('}') &&
                    !string.chars().any(|chr| chr >= 'a' && chr <= 'z') &&
                    Uuid::parse_str(&string[1..37]).is_ok()
            }
            Category::Version => {
                let mut parts = string.split('.');
                parts.clone().count() <= 4 &&
                    parts.all(|part| part.parse::<u16>().is_ok())
            }
            Category::Cabinet => {
                if string.starts_with('#') {
                    Category::Identifier.validate(&string[1..])
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
            "GUID" => Ok(Category::Guid),
            "Guid" => Ok(Category::Guid),
            "Identifier" => Ok(Category::Identifier),
            "Integer" => Ok(Category::Integer),
            "KeyFormatted" => Ok(Category::KeyFormatted),
            "Language" => Ok(Category::Language),
            "LowerCase" => Ok(Category::LowerCase),
            "Path" => Ok(Category::Path),
            "Paths" => Ok(Category::Paths),
            "Property" => Ok(Category::Property),
            "RegPath" => Ok(Category::RegPath),
            "Shortcut" => Ok(Category::Shortcut),
            "Template" => Ok(Category::Template),
            "Text" => Ok(Category::Text),
            "UpperCase" => Ok(Category::UpperCase),
            "URL" => Ok(Category::Url),
            "Url" => Ok(Category::Url),
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
            assert_eq!(category.to_string().parse::<Category>().unwrap(),
                       category);
        }
    }
}

// ========================================================================= //
