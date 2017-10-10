use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use internal::stringpool::StringRef;
use internal::value::{Value, ValueRef};
use std::{fmt, i16, i32};
use std::io::{self, Read, Write};

// ========================================================================= //

const COL_FIELD_SIZE_MASK: i32 = 0xff;
const COL_LOCALIZABLE_BIT: i32 = 0x200;
const COL_STRING_BIT: i32 = 0x800;
const COL_NULLABLE_BIT: i32 = 0x1000;
const COL_PRIMARY_KEY_BIT: i32 = 0x2000;

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

/// A database column.
pub struct Column {
    name: String,
    coltype: ColumnType,
    is_localizable: bool,
    is_nullable: bool,
    is_primary_key: bool,
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
    pub fn build(name: &str) -> ColumnBuilder {
        ColumnBuilder::new(name.to_string())
    }

    /// Creates a new column object with the given name, and with other
    /// attributes determened from the given bitfield (taken from the
    /// `_Columns` table).
    pub(crate) fn from_bitfield(name: String, type_bits: i32)
                                -> io::Result<Column> {
        Ok(Column {
               name: name,
               coltype: ColumnType::from_bitfield(type_bits)?,
               is_localizable: (type_bits & COL_LOCALIZABLE_BIT) != 0,
               is_nullable: (type_bits & COL_NULLABLE_BIT) != 0,
               is_primary_key: (type_bits & COL_PRIMARY_KEY_BIT) != 0,
           })
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

    /// Returns true if the given value is valid for this column.
    pub fn is_valid_value(&self, value: &Value) -> bool {
        match *value {
            Value::Null => self.is_nullable,
            Value::Int(number) => {
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
}

impl ColumnBuilder {
    fn new(name: String) -> ColumnBuilder {
        ColumnBuilder {
            name: name,
            is_localizable: false,
            is_nullable: false,
            is_primary_key: false,
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

    /// Builds a column that stores a 16-bit integer.
    pub fn int16(self) -> Column { self.with_type(ColumnType::Int16) }

    /// Builds a column that stores a 32-bit integer.
    pub fn int32(self) -> Column { self.with_type(ColumnType::Int32) }

    /// Builds a column that stores a string.
    pub fn string(self, max_len: usize) -> Column {
        self.with_type(ColumnType::Str(max_len))
    }

    fn with_type(self, coltype: ColumnType) -> Column {
        Column {
            name: self.name,
            coltype: coltype,
            is_localizable: self.is_localizable,
            is_nullable: self.is_nullable,
            is_primary_key: self.is_primary_key,
        }
    }
}

// ========================================================================= //

#[cfg(test)]
mod tests {
    use super::{Column, ColumnType};
    use internal::codepage::CodePage;
    use internal::stringpool::StringPool;
    use internal::value::{Value, ValueRef};

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

        let column = Column::build("Bar").string(8);
        assert!(!column.is_valid_value(&Value::Null));
        assert!(!column.is_valid_value(&Value::Int(0)));
        assert!(column.is_valid_value(&Value::Str("".to_string())));
        assert!(column.is_valid_value(&Value::Str("1234".to_string())));
        assert!(column.is_valid_value(&Value::Str("12345678".to_string())));
        assert!(!column.is_valid_value(&Value::Str("123456789".to_string())));

        let column = Column::build("Quux").string(0);
        assert!(column.is_valid_value(&Value::Str("".to_string())));
        assert!(column.is_valid_value(&Value::Str("123456789".to_string())));
    }
}

// ========================================================================= //
