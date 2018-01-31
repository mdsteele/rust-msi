use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use internal::stringpool::StringRef;
use internal::value::{Value, ValueRef};
use std::{fmt, i16, i32};
use std::io::{self, Read, Write};
use std::str;
use uuid::Uuid;

// ========================================================================= //

// Constants for the _Columns table's Type column bitfield:
const COL_FIELD_SIZE_MASK: i32 = 0xff;
const COL_LOCALIZABLE_BIT: i32 = 0x200;
const COL_STRING_BIT: i32 = 0x800;
const COL_NULLABLE_BIT: i32 = 0x1000;
const COL_PRIMARY_KEY_BIT: i32 = 0x2000;
// I haven't yet been able to find any clear documentation on what these two
// bits in the column type bitfield do, so both the constant names and the way
// this library handles them are laregly speculative right now:
const COL_VALID_BIT: i32 = 0x100;
const COL_NONBINARY_BIT: i32 = 0x400;

// ========================================================================= //

/// A database column data type.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ColumnType {
    /// A 16-bit integer.
    Int16,
    /// A 32-bit integer.
    Int32,
    /// A string, with the specified maximum length (or zero for no max).
    Str(usize),
}

impl ColumnType {
    fn from_bitfield(type_bits: i32) -> io::Result<ColumnType> {
        let field_size = (type_bits & COL_FIELD_SIZE_MASK) as usize;
        if (type_bits & COL_STRING_BIT) != 0 {
            Ok(ColumnType::Str(field_size))
        } else if field_size == 2 {
            Ok(ColumnType::Int16)
        } else if field_size == 4 {
            Ok(ColumnType::Int32)
        } else {
            invalid_data!("Invalid field size for integer column ({})",
                          field_size);
        }
    }

    fn bitfield(&self) -> i32 {
        match *self {
            ColumnType::Int16 => 0x2,
            ColumnType::Int32 => 0x4,
            ColumnType::Str(max_len) => COL_STRING_BIT | (max_len as i32),
        }
    }

    pub(crate) fn read_value<R: Read>(&self, reader: &mut R,
                                      long_string_refs: bool)
                                      -> io::Result<ValueRef> {
        match *self {
            ColumnType::Int16 => {
                match reader.read_i16::<LittleEndian>()? {
                    0 => Ok(ValueRef::Null),
                    number => Ok(ValueRef::Int((number ^ -0x8000) as i32)),
                }
            }
            ColumnType::Int32 => {
                match reader.read_i32::<LittleEndian>()? {
                    0 => Ok(ValueRef::Null),
                    number => Ok(ValueRef::Int(number ^ -0x8000_0000)),
                }
            }
            ColumnType::Str(_) => {
                match StringRef::read(reader, long_string_refs)? {
                    Some(string_ref) => Ok(ValueRef::Str(string_ref)),
                    None => Ok(ValueRef::Null),
                }
            }
        }
    }

    pub(crate) fn write_value<W: Write>(&self, writer: &mut W,
                                        value_ref: ValueRef,
                                        long_string_refs: bool)
                                        -> io::Result<()> {
        match *self {
            ColumnType::Int16 => {
                match value_ref {
                    ValueRef::Null => writer.write_i16::<LittleEndian>(0)?,
                    ValueRef::Int(number) => {
                        let number = (number as i16) ^ -0x8000;
                        writer.write_i16::<LittleEndian>(number)?
                    }
                    ValueRef::Str(_) => {
                        invalid_input!("Cannot write {:?} to {} column",
                                       value_ref,
                                       self)
                    }
                }
            }
            ColumnType::Int32 => {
                match value_ref {
                    ValueRef::Null => writer.write_i32::<LittleEndian>(0)?,
                    ValueRef::Int(number) => {
                        let number = number ^ -0x8000_0000;
                        writer.write_i32::<LittleEndian>(number)?
                    }
                    ValueRef::Str(_) => {
                        invalid_input!("Cannot write {:?} to {} column",
                                       value_ref,
                                       self)
                    }
                }
            }
            ColumnType::Str(_) => {
                let string_ref = match value_ref {
                    ValueRef::Null => None,
                    ValueRef::Int(_) => {
                        invalid_input!("Cannot write {:?} to {} column",
                                       value_ref,
                                       self)
                    }
                    ValueRef::Str(string_ref) => Some(string_ref),
                };
                StringRef::write(writer, string_ref, long_string_refs)?;
            }
        }
        Ok(())
    }

    pub(crate) fn width(&self, long_string_refs: bool) -> u64 {
        match *self {
            ColumnType::Int16 => 2,
            ColumnType::Int32 => 4,
            ColumnType::Str(_) => if long_string_refs { 3 } else { 2 },
        }
    }
}

impl fmt::Display for ColumnType {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match *self {
            ColumnType::Int16 => formatter.write_str("SMALLINT"),
            ColumnType::Int32 => formatter.write_str("INTEGER"),
            ColumnType::Str(max_len) => {
                formatter.write_str("VARCHAR(")?;
                max_len.fmt(formatter)?;
                formatter.write_str(")")?;
                Ok(())
            }
        }
    }
}

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

/// A database column.
#[derive(Clone)]
pub struct Column {
    name: String,
    coltype: ColumnType,
    is_localizable: bool,
    is_nullable: bool,
    is_primary_key: bool,
    value_range: Option<(i32, i32)>,
    foreign_key: Option<(String, i32)>,
    category: Option<ColumnCategory>,
    enum_values: Vec<String>,
}

impl Column {
    /// Begins building a new column with the given name.
    ///
    /// # Examples
    ///
    /// ```
    /// let column = msi::Column::build("Foo").nullable().int16();
    /// assert_eq!(column.name(), "Foo");
    /// assert!(column.is_nullable());
    /// assert_eq!(column.coltype(), msi::ColumnType::Int16);
    /// ```
    pub fn build<S: Into<String>>(name: S) -> ColumnBuilder {
        ColumnBuilder::new(name.into())
    }

    pub(crate) fn with_name_prefix(&self, prefix: &str) -> Column {
        Column {
            name: format!("{}.{}", prefix, self.name),
            coltype: self.coltype,
            is_localizable: self.is_localizable,
            is_nullable: self.is_nullable,
            is_primary_key: self.is_primary_key,
            value_range: self.value_range,
            foreign_key: self.foreign_key.clone(),
            category: self.category,
            enum_values: self.enum_values.clone(),
        }
    }

    pub(crate) fn but_nullable(mut self) -> Column {
        self.is_nullable = true;
        self
    }

    pub(crate) fn bitfield(&self) -> i32 {
        let mut bits = self.coltype.bitfield() | COL_VALID_BIT;
        if self.is_localizable {
            bits |= COL_LOCALIZABLE_BIT;
        }
        if self.is_nullable {
            bits |= COL_NULLABLE_BIT;
        }
        let nonbinary = match self.coltype {
            ColumnType::Int16 => true,
            ColumnType::Int32 => false,
            ColumnType::Str(0) => {
                self.category != Some(ColumnCategory::Binary)
            }
            ColumnType::Str(_) => true,
        };
        if nonbinary {
            bits |= COL_NONBINARY_BIT;
        }
        if self.is_primary_key {
            bits |= COL_PRIMARY_KEY_BIT;
        }
        bits
    }

    /// Returns true if the given string is a valid column name.
    pub(crate) fn is_valid_name(name: &str) -> bool {
        ColumnCategory::Identifier.validate(name)
    }

    /// Returns the name of the column.
    pub fn name(&self) -> &str { &self.name }

    /// Returns the type of data stored in the column.
    pub fn coltype(&self) -> ColumnType { self.coltype }

    /// Returns true if values in this column can be localized.
    pub fn is_localizable(&self) -> bool { self.is_localizable }

    /// Returns true if values in this column can be null.
    pub fn is_nullable(&self) -> bool { self.is_nullable }

    /// Returns true if this is primary key column.
    pub fn is_primary_key(&self) -> bool { self.is_primary_key }

    /// Returns the (min, max) integer value range for this column, if any.
    pub fn value_range(&self) -> Option<(i32, i32)> { self.value_range }

    pub(crate) fn foreign_key(&self) -> Option<(&str, i32)> {
        self.foreign_key
            .as_ref()
            .map(|&(ref name, index)| (name.as_str(), index))
    }

    /// Returns the string value category for this column, if any.
    pub fn category(&self) -> Option<ColumnCategory> { self.category }

    /// Returns the list of valid enum values for this column, if any.
    pub fn enum_values(&self) -> Option<&[String]> {
        if self.enum_values.is_empty() {
            None
        } else {
            Some(&self.enum_values)
        }
    }

    /// Returns true if the given value is valid for this column.
    pub fn is_valid_value(&self, value: &Value) -> bool {
        match *value {
            Value::Null => self.is_nullable,
            Value::Int(number) => {
                if let Some((min, max)) = self.value_range {
                    if number < min || number > max {
                        return false;
                    }
                }
                match self.coltype {
                    ColumnType::Int16 => {
                        number > (i16::MIN as i32) &&
                            number <= (i16::MAX as i32)
                    }
                    ColumnType::Int32 => number > i32::MIN,
                    ColumnType::Str(_) => false,
                }
            }
            Value::Str(ref string) => {
                match self.coltype {
                    ColumnType::Int16 |
                    ColumnType::Int32 => false,
                    ColumnType::Str(max_len) => {
                        if let Some(category) = self.category {
                            if !category.validate(&string) {
                                return false;
                            }
                        }
                        if !self.enum_values.is_empty() &&
                            !self.enum_values.contains(string)
                        {
                            return false;
                        }
                        max_len == 0 || string.chars().count() <= max_len
                    }
                }
            }
        }
    }
}

// ========================================================================= //

/// A factory for configuring a new database column.
pub struct ColumnBuilder {
    name: String,
    is_localizable: bool,
    is_nullable: bool,
    is_primary_key: bool,
    value_range: Option<(i32, i32)>,
    foreign_key: Option<(String, i32)>,
    category: Option<ColumnCategory>,
    enum_values: Vec<String>,
}

impl ColumnBuilder {
    fn new(name: String) -> ColumnBuilder {
        ColumnBuilder {
            name: name,
            is_localizable: false,
            is_nullable: false,
            is_primary_key: false,
            value_range: None,
            foreign_key: None,
            category: None,
            enum_values: Vec::new(),
        }
    }

    /// Makes the column be localizable.
    pub fn localizable(mut self) -> ColumnBuilder {
        self.is_localizable = true;
        self
    }

    /// Makes the column allow null values.
    pub fn nullable(mut self) -> ColumnBuilder {
        self.is_nullable = true;
        self
    }

    /// Makes the column be a primary key column.
    pub fn primary_key(mut self) -> ColumnBuilder {
        self.is_primary_key = true;
        self
    }

    /// Makes the column only permit values in the given range.
    pub fn range(mut self, min: i32, max: i32) -> ColumnBuilder {
        self.value_range = Some((min, max));
        self
    }

    /// Makes the column refer to a key column in another table.
    pub fn foreign_key(mut self, table_name: &str, column_index: i32)
                       -> ColumnBuilder {
        self.foreign_key = Some((table_name.to_string(), column_index));
        self
    }

    /// For string columns, makes the column use the specified data format.
    pub fn category(mut self, category: ColumnCategory) -> ColumnBuilder {
        self.category = Some(category);
        self
    }

    /// Makes the column only permit the given values.
    pub fn enum_values(mut self, values: &[&str]) -> ColumnBuilder {
        self.enum_values = values.iter().map(|val| val.to_string()).collect();
        self
    }

    /// Builds a column that stores a 16-bit integer.
    pub fn int16(self) -> Column { self.with_type(ColumnType::Int16) }

    /// Builds a column that stores a 32-bit integer.
    pub fn int32(self) -> Column { self.with_type(ColumnType::Int32) }

    /// Builds a column that stores a string.
    pub fn string(self, max_len: usize) -> Column {
        self.with_type(ColumnType::Str(max_len))
    }

    /// Builds a column that stores an identifier string.  This is equivalent
    /// to `self.category(ColumnCategory::Identifier).string(max_len)`.
    pub fn id_string(self, max_len: usize) -> Column {
        self.category(ColumnCategory::Identifier).string(max_len)
    }

    /// Builds a column that stores a text string.  This is equivalent to
    /// `self.category(ColumnCategory::Text).string(max_len)`.
    pub fn text_string(self, max_len: usize) -> Column {
        self.category(ColumnCategory::Text).string(max_len)
    }

    /// Builds a column that stores a formatted string.  This is equivalent to
    /// `self.category(ColumnCategory::Formatted).string(max_len)`.
    pub fn formatted_string(self, max_len: usize) -> Column {
        self.category(ColumnCategory::Formatted).string(max_len)
    }

    /// Builds a column that refers to a binary data stream.  This sets the
    /// category to `ColumnCategory::Binary` in addition to setting the column
    /// type.
    pub fn binary(self) -> Column {
        self.category(ColumnCategory::Binary).string(0)
    }

    fn with_type(self, coltype: ColumnType) -> Column {
        Column {
            name: self.name,
            coltype: coltype,
            is_localizable: self.is_localizable,
            is_nullable: self.is_nullable,
            is_primary_key: self.is_primary_key,
            value_range: self.value_range,
            foreign_key: self.foreign_key,
            category: self.category,
            enum_values: self.enum_values,
        }
    }

    pub(crate) fn with_bitfield(self, type_bits: i32) -> io::Result<Column> {
        let is_nullable = (type_bits & COL_NULLABLE_BIT) != 0;
        Ok(Column {
               name: self.name,
               coltype: ColumnType::from_bitfield(type_bits)?,
               is_localizable: (type_bits & COL_LOCALIZABLE_BIT) != 0,
               is_nullable: is_nullable || self.is_nullable,
               is_primary_key: (type_bits & COL_PRIMARY_KEY_BIT) != 0,
               value_range: self.value_range,
               foreign_key: self.foreign_key,
               category: self.category,
               enum_values: self.enum_values,
           })
    }
}

// ========================================================================= //

#[cfg(test)]
mod tests {
    use super::{Column, ColumnCategory, ColumnType};
    use internal::codepage::CodePage;
    use internal::stringpool::StringPool;
    use internal::value::{Value, ValueRef};

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

    #[test]
    fn valid_column_name() {
        assert!(Column::is_valid_name("fooBar"));
        assert!(Column::is_valid_name("_Whatever"));
        assert!(Column::is_valid_name("Catch22"));
        assert!(Column::is_valid_name("Foo.Bar"));

        assert!(!Column::is_valid_name(""));
        assert!(!Column::is_valid_name("99Bottles"));
    }

    #[test]
    fn read_column_value() {
        let mut input: &[u8] = b"\x00\x00";
        assert_eq!(ColumnType::Int16.read_value(&mut input, false).unwrap(),
                   ValueRef::Null);

        let mut input: &[u8] = b"\x23\x81";
        assert_eq!(ColumnType::Int16.read_value(&mut input, false).unwrap(),
                   ValueRef::Int(0x123));

        let mut input: &[u8] = b"\xff\x7f";
        assert_eq!(ColumnType::Int16.read_value(&mut input, false).unwrap(),
                   ValueRef::Int(-1));

        let mut input: &[u8] = b"\x00\x00\x00\x00";
        assert_eq!(ColumnType::Int32.read_value(&mut input, false).unwrap(),
                   ValueRef::Null);

        let mut input: &[u8] = b"\x67\x45\x23\x81";
        assert_eq!(ColumnType::Int32.read_value(&mut input, false).unwrap(),
                   ValueRef::Int(0x1234567));

        let mut input: &[u8] = b"\xff\xff\xff\x7f";
        assert_eq!(ColumnType::Int32.read_value(&mut input, false).unwrap(),
                   ValueRef::Int(-1));

        let mut string_pool = StringPool::new(CodePage::default());
        let string_ref = string_pool.incref("Hello, world!".to_string());
        assert_eq!(string_ref.number(), 1);

        let mut input: &[u8] = b"\x00\x00";
        assert_eq!(ColumnType::Str(24).read_value(&mut input, false).unwrap(),
                   ValueRef::Null);

        let mut input: &[u8] = b"\x01\x00";
        assert_eq!(ColumnType::Str(24).read_value(&mut input, false).unwrap(),
                   ValueRef::Str(string_ref));

        let mut input: &[u8] = b"\x00\x00\x00";
        assert_eq!(ColumnType::Str(24).read_value(&mut input, true).unwrap(),
                   ValueRef::Null);

        let mut input: &[u8] = b"\x01\x00\x00";
        assert_eq!(ColumnType::Str(24).read_value(&mut input, true).unwrap(),
                   ValueRef::Str(string_ref));
    }

    #[test]
    fn write_column_value() {
        let mut output = Vec::<u8>::new();
        let value_ref = ValueRef::Null;
        ColumnType::Int16.write_value(&mut output, value_ref, false).unwrap();
        assert_eq!(&output as &[u8], b"\x00\x00");

        let mut output = Vec::<u8>::new();
        let value_ref = ValueRef::Int(0x123);
        ColumnType::Int16.write_value(&mut output, value_ref, false).unwrap();
        assert_eq!(&output as &[u8], b"\x23\x81");

        let mut output = Vec::<u8>::new();
        let value_ref = ValueRef::Int(-1);
        ColumnType::Int16.write_value(&mut output, value_ref, false).unwrap();
        assert_eq!(&output as &[u8], b"\xff\x7f");

        let mut output = Vec::<u8>::new();
        let value_ref = ValueRef::Null;
        ColumnType::Int32.write_value(&mut output, value_ref, false).unwrap();
        assert_eq!(&output as &[u8], b"\x00\x00\x00\x00");

        let mut output = Vec::<u8>::new();
        let value_ref = ValueRef::Int(0x1234567);
        ColumnType::Int32.write_value(&mut output, value_ref, false).unwrap();
        assert_eq!(&output as &[u8], b"\x67\x45\x23\x81");

        let mut output = Vec::<u8>::new();
        let value_ref = ValueRef::Int(-1);
        ColumnType::Int32.write_value(&mut output, value_ref, false).unwrap();
        assert_eq!(&output as &[u8], b"\xff\xff\xff\x7f");

        let mut string_pool = StringPool::new(CodePage::default());
        let string_ref = string_pool.incref("Hello, world!".to_string());
        assert_eq!(string_ref.number(), 1);

        let mut output = Vec::<u8>::new();
        let value_ref = ValueRef::Null;
        ColumnType::Str(9).write_value(&mut output, value_ref, false).unwrap();
        assert_eq!(&output as &[u8], b"\x00\x00");

        let mut output = Vec::<u8>::new();
        let value_ref = ValueRef::Str(string_ref);
        ColumnType::Str(9).write_value(&mut output, value_ref, false).unwrap();
        assert_eq!(&output as &[u8], b"\x01\x00");

        let mut output = Vec::<u8>::new();
        let value_ref = ValueRef::Null;
        ColumnType::Str(9).write_value(&mut output, value_ref, true).unwrap();
        assert_eq!(&output as &[u8], b"\x00\x00\x00");

        let mut output = Vec::<u8>::new();
        let value_ref = ValueRef::Str(string_ref);
        ColumnType::Str(9).write_value(&mut output, value_ref, true).unwrap();
        assert_eq!(&output as &[u8], b"\x01\x00\x00");
    }

    #[test]
    fn valid_column_value() {
        let column = Column::build("Foo").nullable().int16();
        assert!(column.is_valid_value(&Value::Null));
        assert!(column.is_valid_value(&Value::Int(0x7fff)));
        assert!(!column.is_valid_value(&Value::Int(0x8000)));
        assert!(column.is_valid_value(&Value::Int(-0x7fff)));
        assert!(!column.is_valid_value(&Value::Int(-0x8000)));
        assert!(!column.is_valid_value(&Value::Str("1234".to_string())));

        let column = Column::build("Bar").int32();
        assert!(!column.is_valid_value(&Value::Null));
        assert!(column.is_valid_value(&Value::Int(0x7fff_ffff)));
        assert!(column.is_valid_value(&Value::Int(-0x7fff_ffff)));
        assert!(!column.is_valid_value(&Value::Int(-0x8000_0000)));
        assert!(!column.is_valid_value(&Value::Str("1234".to_string())));

        let column = Column::build("Bar").range(1, 32).int32();
        assert!(!column.is_valid_value(&Value::Int(0)));
        assert!(column.is_valid_value(&Value::Int(1)));
        assert!(column.is_valid_value(&Value::Int(7)));
        assert!(column.is_valid_value(&Value::Int(32)));
        assert!(!column.is_valid_value(&Value::Int(33)));

        let column = Column::build("Baz").string(8);
        assert!(!column.is_valid_value(&Value::Null));
        assert!(!column.is_valid_value(&Value::Int(0)));
        assert!(column.is_valid_value(&Value::Str("".to_string())));
        assert!(column.is_valid_value(&Value::Str("1234".to_string())));
        assert!(column.is_valid_value(&Value::Str("12345678".to_string())));
        assert!(!column.is_valid_value(&Value::Str("123456789".to_string())));

        let column = Column::build("Quux").string(0);
        assert!(column.is_valid_value(&Value::Str("".to_string())));
        assert!(column.is_valid_value(&Value::Str("123456789".to_string())));

        let column = Column::build("Foo").id_string(0);
        assert!(column.is_valid_value(&Value::Str("FooBar".to_string())));
        assert!(!column.is_valid_value(&Value::Str("".to_string())));
        assert!(!column.is_valid_value(&Value::Str("1234".to_string())));

        let column = Column::build("Bar").enum_values(&["Y", "N"]).string(1);
        assert!(column.is_valid_value(&Value::Str("Y".to_string())));
        assert!(column.is_valid_value(&Value::Str("N".to_string())));
        assert!(!column.is_valid_value(&Value::Str("X".to_string())));
    }
}

// ========================================================================= //
