use byteorder::{LittleEndian, ReadBytesExt};
use internal::streamname;
use internal::stringpool::{StringPool, StringRef};
use std::fmt;
use std::io::{self, Read, Seek, SeekFrom};
use std::ops::Index;
use std::usize;

// ========================================================================= //

const COL_FIELD_SIZE_MASK: i32 = 0xff;
const COL_STRING_BIT: i32 = 0x800;
const COL_NULLABLE_BIT: i32 = 0x1000;
const COL_PRIMARY_KEY_BIT: i32 = 0x2000;

// ========================================================================= //

/// A value from one cell in a table row.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Value {
    /// A null value.
    Null,
    /// An integer value.
    Int(i32),
    /// A string value.
    Str(String),
}

impl Value {
    /// Returns true if this is a null value.
    pub fn is_null(&self) -> bool {
        match *self {
            Value::Null => true,
            _ => false,
        }
    }

    /// Returns true if this is an integer value.
    pub fn is_int(&self) -> bool {
        match *self {
            Value::Int(_) => true,
            _ => false,
        }
    }

    /// Extracts the integer value if it is an integer.
    pub fn as_int(&self) -> Option<i32> {
        match *self {
            Value::Null => None,
            Value::Int(value) => Some(value),
            Value::Str(_) => None,
        }
    }

    /// Returns true if this is a string value.
    pub fn is_str(&self) -> bool {
        match *self {
            Value::Str(_) => true,
            _ => false,
        }
    }

    /// Extracts the string value if it is a string.
    pub fn as_str(&self) -> Option<&str> {
        match *self {
            Value::Null => None,
            Value::Int(_) => None,
            Value::Str(ref string) => Some(string.as_str()),
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match *self {
            Value::Null => formatter.write_str("NULL"),
            Value::Int(value) => value.fmt(formatter),
            Value::Str(ref string) => formatter.write_str(&string),
        }
    }
}

// ========================================================================= //

/// An indirect value from one cell in a table row.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ValueRef {
    /// A null value.
    Null,
    /// An integer value.
    Int(i32),
    /// A string value.
    Str(StringRef),
}

impl ValueRef {
    /// Dereferences the `ValueRef` into a `Value`.
    pub fn to_value(&self, string_pool: &StringPool) -> Value {
        match *self {
            ValueRef::Null => Value::Null,
            ValueRef::Int(value) => Value::Int(value),
            ValueRef::Str(string_ref) => {
                Value::Str(string_pool.get(string_ref).to_string())
            }
        }
    }
}

// ========================================================================= //

/// A column data type.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ColumnType {
    /// A 16-bit integer.
    Int16,
    /// A 32-bit integer.
    Int32,
    /// A string, with the specified maximum length.
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
                    value => Ok(ValueRef::Int((value ^ -0x8000) as i32)),
                }
            }
            ColumnType::Int32 => {
                match reader.read_i32::<LittleEndian>()? {
                    0 => Ok(ValueRef::Null),
                    value => Ok(ValueRef::Int(value ^ -0x8000_0000)),
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
            ColumnType::Str(field_size) => {
                formatter.write_str("VARCHAR(")?;
                field_size.fmt(formatter)?;
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

    /// Returns true if this is primary key column, false otherwise.
    pub fn is_primary_key(&self) -> bool { self.is_primary_key }

    /// Returns true if values in this column can be null, false otherwise.
    pub fn is_nullable(&self) -> bool { self.is_nullable }
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
            for row in rows.iter_mut() {
                row.push(column
                             .coltype
                             .read_value(&mut reader, self.long_string_refs)?);
            }
        }
        Ok(rows)
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

/// An iterator over the rows in a table.
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
