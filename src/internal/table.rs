use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use internal::streamname;
use internal::stringpool::{StringPool, StringRef};
use internal::value::{Value, ValueRef};
use std::{fmt, i16, i32, usize};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::ops::Index;

// ========================================================================= //

const COL_FIELD_SIZE_MASK: i32 = 0xff;
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

    fn read_value<R: Read>(&self, reader: &mut R, long_string_refs: bool)
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

    fn write_value<W: Write>(&self, writer: &mut W, value_ref: ValueRef,
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

    fn width(&self, long_string_refs: bool) -> u64 {
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
    is_primary_key: bool,
    is_nullable: bool,
}

impl Column {
    /// Creates a new column object with the given name, type, and primary key
    /// status.
    pub fn new(name: &str, coltype: ColumnType, is_key: bool) -> Column {
        Column {
            name: name.to_string(),
            coltype: coltype,
            is_primary_key: is_key,
            is_nullable: false,
        }
    }

    /// Creates a new column object with the given name, and with other
    /// attributes determened from the given bitfield (taken from the
    /// `_Columns` table).
    pub(crate) fn from_bitfield(name: String, type_bits: i32)
                                -> io::Result<Column> {
        Ok(Column {
               name: name,
               coltype: ColumnType::from_bitfield(type_bits)?,
               is_primary_key: (type_bits & COL_PRIMARY_KEY_BIT) != 0,
               is_nullable: (type_bits & COL_NULLABLE_BIT) != 0,
           })
    }

    /// Returns the name of the column.
    pub fn name(&self) -> &str { &self.name }

    /// Returns the type of data stored in the column.
    pub fn coltype(&self) -> ColumnType { self.coltype }

    /// Returns true if this is primary key column.
    pub fn is_primary_key(&self) -> bool { self.is_primary_key }

    /// Returns true if values in this column can be null.
    pub fn is_nullable(&self) -> bool { self.is_nullable }

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

/// A database table.
pub struct Table {
    name: String,
    columns: Vec<Column>,
    long_string_refs: bool,
}

impl Table {
    /// Creates a new table object with the given name and columns.  The
    /// `long_string_refs` argument indicates the size of any encoded string
    /// refs.
    pub fn new(name: String, columns: Vec<Column>, long_string_refs: bool)
               -> Table {
        Table {
            name: name,
            columns: columns,
            long_string_refs: long_string_refs,
        }
    }

    /// Returns the name of the table.
    pub fn name(&self) -> &str { &self.name }

    /// Returns the name of the CFB stream that holds this table's data.
    pub(crate) fn stream_name(&self) -> String {
        streamname::encode(&self.name, true)
    }

    /// Returns the list of columns in this table.
    pub fn columns(&self) -> &[Column] { &self.columns }

    /// Returns the indices of table's primary key columns.
    pub fn primary_key_indices(&self) -> Vec<usize> {
        self.columns
            .iter()
            .enumerate()
            .filter_map(|(index, column)| if column.is_primary_key() {
                            Some(index)
                        } else {
                            None
                        })
            .collect()
    }

    fn index_for_column_name(&self, column_name: &str) -> usize {
        for (index, column) in self.columns.iter().enumerate() {
            if column.name.as_str() == column_name {
                return index;
            }
        }
        panic!("Table {:?} has no column named {:?}",
               self.name,
               column_name);
    }

    /// Parses row data from the given data source and returns an interator
    /// over the rows.
    pub(crate) fn read_rows<R: Read + Seek>(
        &self, mut reader: R)
        -> io::Result<Vec<Vec<ValueRef>>> {
        let data_length = reader.seek(SeekFrom::End(0))?;
        reader.seek(SeekFrom::Start(0))?;
        let row_size = self.columns
            .iter()
            .map(|col| col.coltype.width(self.long_string_refs))
            .sum::<u64>();
        let num_columns = self.columns.len();
        let num_rows = if row_size > 0 {
            (data_length / row_size) as usize
        } else {
            0
        };
        let mut rows =
            vec![Vec::<ValueRef>::with_capacity(num_columns); num_rows];
        for column in self.columns.iter() {
            let coltype = column.coltype;
            for row in rows.iter_mut() {
                row.push(coltype
                             .read_value(&mut reader, self.long_string_refs)?);
            }
        }
        Ok(rows)
    }

    pub(crate) fn write_rows<W: Write>(&self, mut writer: W,
                                       rows: Vec<Vec<ValueRef>>)
                                       -> io::Result<()> {
        for (index, column) in self.columns.iter().enumerate() {
            let coltype = column.coltype;
            for row in rows.iter() {
                coltype
                    .write_value(&mut writer,
                                 row[index],
                                 self.long_string_refs)?;
            }
        }
        Ok(())
    }
}

// ========================================================================= //

/// One row from a database table.
pub struct Row<'a> {
    table: &'a Table,
    values: Vec<Value>,
}

impl<'a> Row<'a> {
    /// Returns the number of columns in the row.
    pub fn len(&self) -> usize { self.table.columns().len() }
}

impl<'a> Index<usize> for Row<'a> {
    type Output = Value;

    fn index(&self, index: usize) -> &Value { &self.values[index] }
}

impl<'a, 'b> Index<&'b str> for Row<'a> {
    type Output = Value;

    fn index(&self, column_name: &str) -> &Value {
        let index = self.table.index_for_column_name(column_name);
        &self.values[index]
    }
}

// ========================================================================= //

/// An iterator over the rows in a database table.
pub struct Rows<'a> {
    string_pool: &'a StringPool,
    table: &'a Table,
    rows: Vec<Vec<ValueRef>>,
    next_row_index: usize,
}

impl<'a> Rows<'a> {
    pub(crate) fn new(string_pool: &'a StringPool, table: &'a Table,
                      rows: Vec<Vec<ValueRef>>)
                      -> Rows<'a> {
        Rows {
            table: table,
            string_pool: string_pool,
            rows: rows,
            next_row_index: 0,
        }
    }
}

impl<'a> Iterator for Rows<'a> {
    type Item = Row<'a>;

    fn next(&mut self) -> Option<Row<'a>> {
        if self.next_row_index < self.rows.len() {
            let values: Vec<Value> = self.rows[self.next_row_index]
                .iter()
                .map(|value_ref| value_ref.to_value(self.string_pool))
                .collect();
            self.next_row_index += 1;
            Some(Row {
                     table: self.table,
                     values: values,
                 })
        } else {
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        debug_assert!(self.next_row_index <= self.rows.len());
        let remaining_rows = self.rows.len() - self.next_row_index;
        (remaining_rows, Some(remaining_rows))
    }
}

impl<'a> ExactSizeIterator for Rows<'a> {}

// ========================================================================= //

#[cfg(test)]
mod tests {
    use super::{ColumnType, ValueRef};
    use internal::codepage::CodePage;
    use internal::stringpool::StringPool;

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
}

// ========================================================================= //
