use crate::internal::category::Category;
use crate::internal::stringpool::StringRef;
use crate::internal::value::{Value, ValueRef};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use std::fmt;
use std::io::{self, Read, Write};
use std::str;

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
    #[allow(clippy::if_same_then_else)]
    fn from_bitfield(type_bits: i32) -> io::Result<ColumnType> {
        let field_size = (type_bits & COL_FIELD_SIZE_MASK) as usize;
        if (type_bits & COL_STRING_BIT) != 0 {
            Ok(ColumnType::Str(field_size))
        } else if field_size == 4 {
            Ok(ColumnType::Int32)
        } else if field_size == 2 {
            Ok(ColumnType::Int16)
        } else if field_size == 1 {
            // Some implementations seem to set the integer field size to 1 for
            // certain columns, but still store the data with 2 bytes?  See
            // https://github.com/mdsteele/rust-msi/issues/8.
            Ok(ColumnType::Int16)
        } else {
            invalid_data!(
                "Invalid field size for integer column ({})",
                field_size
            );
        }
    }

    fn bitfield(&self) -> i32 {
        match *self {
            ColumnType::Int16 => 0x2,
            ColumnType::Int32 => 0x4,
            ColumnType::Str(max_len) => COL_STRING_BIT | (max_len as i32),
        }
    }

    pub(crate) fn read_value<R: Read>(
        &self,
        reader: &mut R,
        long_string_refs: bool,
    ) -> io::Result<ValueRef> {
        match *self {
            ColumnType::Int16 => match reader.read_i16::<LittleEndian>()? {
                0 => Ok(ValueRef::Null),
                number => Ok(ValueRef::Int((number ^ -0x8000) as i32)),
            },
            ColumnType::Int32 => match reader.read_i32::<LittleEndian>()? {
                0 => Ok(ValueRef::Null),
                number => Ok(ValueRef::Int(number ^ -0x8000_0000)),
            },
            ColumnType::Str(_) => {
                match StringRef::read(reader, long_string_refs)? {
                    Some(string_ref) => Ok(ValueRef::Str(string_ref)),
                    None => Ok(ValueRef::Null),
                }
            }
        }
    }

    pub(crate) fn write_value<W: Write>(
        &self,
        writer: &mut W,
        value_ref: ValueRef,
        long_string_refs: bool,
    ) -> io::Result<()> {
        match *self {
            ColumnType::Int16 => match value_ref {
                ValueRef::Null => writer.write_i16::<LittleEndian>(0)?,
                ValueRef::Int(number) => {
                    let number = (number as i16) ^ -0x8000;
                    writer.write_i16::<LittleEndian>(number)?
                }
                ValueRef::Str(_) => invalid_input!(
                    "Cannot write {:?} to {} column",
                    value_ref,
                    self
                ),
            },
            ColumnType::Int32 => match value_ref {
                ValueRef::Null => writer.write_i32::<LittleEndian>(0)?,
                ValueRef::Int(number) => {
                    let number = number ^ -0x8000_0000;
                    writer.write_i32::<LittleEndian>(number)?
                }
                ValueRef::Str(_) => invalid_input!(
                    "Cannot write {:?} to {} column",
                    value_ref,
                    self
                ),
            },
            ColumnType::Str(_) => {
                let string_ref = match value_ref {
                    ValueRef::Null => None,
                    ValueRef::Int(_) => invalid_input!(
                        "Cannot write {:?} to {} column",
                        value_ref,
                        self
                    ),
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
            ColumnType::Str(_) => {
                if long_string_refs {
                    3
                } else {
                    2
                }
            }
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
    category: Option<Category>,
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
        if prefix.is_empty() {
            self.clone()
        } else {
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
            ColumnType::Str(0) => self.category != Some(Category::Binary),
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
        Category::Identifier.validate(name)
    }

    /// Returns the name of the column.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the type of data stored in the column.
    pub fn coltype(&self) -> ColumnType {
        self.coltype
    }

    /// Returns true if values in this column can be localized.
    pub fn is_localizable(&self) -> bool {
        self.is_localizable
    }

    /// Returns true if values in this column can be null.
    pub fn is_nullable(&self) -> bool {
        self.is_nullable
    }

    /// Returns true if this is primary key column.
    pub fn is_primary_key(&self) -> bool {
        self.is_primary_key
    }

    /// Returns the (min, max) integer value range for this column, if any.
    pub fn value_range(&self) -> Option<(i32, i32)> {
        self.value_range
    }

    pub(crate) fn foreign_key(&self) -> Option<(&str, i32)> {
        self.foreign_key
            .as_ref()
            .map(|&(ref name, index)| (name.as_str(), index))
    }

    /// Returns the string value category for this column, if any.
    pub fn category(&self) -> Option<Category> {
        self.category
    }

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
                        number > (i16::MIN as i32)
                            && number <= (i16::MAX as i32)
                    }
                    ColumnType::Int32 => number > i32::MIN,
                    ColumnType::Str(_) => false,
                }
            }
            Value::Str(ref string) => match self.coltype {
                ColumnType::Int16 | ColumnType::Int32 => false,
                ColumnType::Str(max_len) => {
                    if let Some(category) = self.category {
                        if !category.validate(string) {
                            return false;
                        }
                    }
                    if !self.enum_values.is_empty()
                        && !self.enum_values.contains(string)
                    {
                        return false;
                    }
                    max_len == 0 || string.chars().count() <= max_len
                }
            },
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
    category: Option<Category>,
    enum_values: Vec<String>,
}

impl ColumnBuilder {
    fn new(name: String) -> ColumnBuilder {
        ColumnBuilder {
            name,
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
    pub fn foreign_key(
        mut self,
        table_name: &str,
        column_index: i32,
    ) -> ColumnBuilder {
        self.foreign_key = Some((table_name.to_string(), column_index));
        self
    }

    /// For string columns, makes the column use the specified data format.
    pub fn category(mut self, category: Category) -> ColumnBuilder {
        self.category = Some(category);
        self
    }

    /// Makes the column only permit the given values.
    pub fn enum_values(mut self, values: &[&str]) -> ColumnBuilder {
        self.enum_values = values.iter().map(|val| val.to_string()).collect();
        self
    }

    /// Builds a column that stores a 16-bit integer.
    pub fn int16(self) -> Column {
        self.with_type(ColumnType::Int16)
    }

    /// Builds a column that stores a 32-bit integer.
    pub fn int32(self) -> Column {
        self.with_type(ColumnType::Int32)
    }

    /// Builds a column that stores a string.
    pub fn string(self, max_len: usize) -> Column {
        self.with_type(ColumnType::Str(max_len))
    }

    /// Builds a column that stores an identifier string.  This is equivalent
    /// to `self.category(Category::Identifier).string(max_len)`.
    pub fn id_string(self, max_len: usize) -> Column {
        self.category(Category::Identifier).string(max_len)
    }

    /// Builds a column that stores a text string.  This is equivalent to
    /// `self.category(Category::Text).string(max_len)`.
    pub fn text_string(self, max_len: usize) -> Column {
        self.category(Category::Text).string(max_len)
    }

    /// Builds a column that stores a formatted string.  This is equivalent to
    /// `self.category(Category::Formatted).string(max_len)`.
    pub fn formatted_string(self, max_len: usize) -> Column {
        self.category(Category::Formatted).string(max_len)
    }

    /// Builds a column that refers to a binary data stream.  This sets the
    /// category to `Category::Binary` in addition to setting the column
    /// type.
    pub fn binary(self) -> Column {
        self.category(Category::Binary).string(0)
    }

    fn with_type(self, coltype: ColumnType) -> Column {
        Column {
            name: self.name,
            coltype,
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
    use super::{Column, ColumnType};
    use crate::internal::codepage::CodePage;
    use crate::internal::stringpool::StringPool;
    use crate::internal::value::{Value, ValueRef};

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
        assert_eq!(
            ColumnType::Int16.read_value(&mut input, false).unwrap(),
            ValueRef::Null
        );

        let mut input: &[u8] = b"\x23\x81";
        assert_eq!(
            ColumnType::Int16.read_value(&mut input, false).unwrap(),
            ValueRef::Int(0x123)
        );

        let mut input: &[u8] = b"\xff\x7f";
        assert_eq!(
            ColumnType::Int16.read_value(&mut input, false).unwrap(),
            ValueRef::Int(-1)
        );

        let mut input: &[u8] = b"\x00\x00\x00\x00";
        assert_eq!(
            ColumnType::Int32.read_value(&mut input, false).unwrap(),
            ValueRef::Null
        );

        let mut input: &[u8] = b"\x67\x45\x23\x81";
        assert_eq!(
            ColumnType::Int32.read_value(&mut input, false).unwrap(),
            ValueRef::Int(0x1234567)
        );

        let mut input: &[u8] = b"\xff\xff\xff\x7f";
        assert_eq!(
            ColumnType::Int32.read_value(&mut input, false).unwrap(),
            ValueRef::Int(-1)
        );

        let mut string_pool = StringPool::new(CodePage::default());
        let string_ref = string_pool.incref("Hello, world!".to_string());
        assert_eq!(string_ref.number(), 1);

        let mut input: &[u8] = b"\x00\x00";
        assert_eq!(
            ColumnType::Str(24).read_value(&mut input, false).unwrap(),
            ValueRef::Null
        );

        let mut input: &[u8] = b"\x01\x00";
        assert_eq!(
            ColumnType::Str(24).read_value(&mut input, false).unwrap(),
            ValueRef::Str(string_ref)
        );

        let mut input: &[u8] = b"\x00\x00\x00";
        assert_eq!(
            ColumnType::Str(24).read_value(&mut input, true).unwrap(),
            ValueRef::Null
        );

        let mut input: &[u8] = b"\x01\x00\x00";
        assert_eq!(
            ColumnType::Str(24).read_value(&mut input, true).unwrap(),
            ValueRef::Str(string_ref)
        );
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
