use byteorder::{LittleEndian, ReadBytesExt};
use internal::streamname;
use internal::stringpool::{StringPool, StringRef};
use std::fmt;
use std::io::{self, Read, Seek, SeekFrom};
use std::usize;

// ========================================================================= //

const COL_FIELD_SIZE_MASK: i32 = 0xff;
const COL_STRING_BIT: i32 = 0x800;
const COL_NULLABLE_BIT: i32 = 0x1000;
const COL_PRIMARY_KEY_BIT: i32 = 0x2000;

// ========================================================================= //

/// A value from one cell in a table row.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RowValue {
    /// A null value.
    Null,
    /// An integer value.
    Int(i32),
    /// A string value.
    Str(StringRef),
}

impl RowValue {
    /// Formats the value as a string, using the given string pool to look up
    /// string references.
    pub fn to_string(&self, string_pool: &StringPool) -> String {
        match *self {
            RowValue::Null => "NULL".to_string(),
            RowValue::Int(value) => format!("{}", value),
            RowValue::Str(string_ref) => {
                string_pool.get(string_ref).to_string()
            }
        }
    }

    /// Returns the value as an integer.  For string values, this will return
    /// the string reference number.
    pub fn to_i32(&self) -> i32 {
        match *self {
            RowValue::Null => 0,
            RowValue::Int(value) => value,
            RowValue::Str(string_ref) => string_ref.number(),
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
    fn from_bitfield(type_bits: i32) -> ColumnType {
        let field_size = (type_bits & COL_FIELD_SIZE_MASK) as usize;
        if (type_bits & COL_STRING_BIT) != 0 {
            ColumnType::Str(field_size)
        } else if field_size == 2 {
            ColumnType::Int16
        } else {
            ColumnType::Int32
        }
    }

    fn read_value<R: Read>(&self, reader: &mut R, long_string_refs: bool)
                           -> io::Result<RowValue> {
        match *self {
            ColumnType::Int16 => {
                match reader.read_i16::<LittleEndian>()? {
                    0 => Ok(RowValue::Null),
                    value => Ok(RowValue::Int((value ^ -0x8000) as i32)),
                }
            }
            ColumnType::Int32 => {
                match reader.read_i32::<LittleEndian>()? {
                    0 => Ok(RowValue::Null),
                    value => Ok(RowValue::Int(value ^ -0x8000_0000)),
                }
            }
            ColumnType::Str(_) => {
                match StringRef::read(reader, long_string_refs)? {
                    Some(string_ref) => Ok(RowValue::Str(string_ref)),
                    None => Ok(RowValue::Null),
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
    pub fn from_bitfield(name: String, type_bits: i32) -> Column {
        Column {
            name: name,
            coltype: ColumnType::from_bitfield(type_bits),
            is_primary_key: (type_bits & COL_PRIMARY_KEY_BIT) != 0,
            is_nullable: (type_bits & COL_NULLABLE_BIT) != 0,
        }
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

    /// Parses row data from the given data source and returns an interator
    /// over the rows.
    pub(crate) fn read_rows<R: Read + Seek>(&self, mut reader: R)
                                            -> io::Result<Rows<R>> {
        let data_length = reader.seek(SeekFrom::End(0))?;
        let row_size = self.columns
            .iter()
            .map(|col| col.coltype.width(self.long_string_refs))
            .sum::<u64>();
        let num_rows = if row_size > 0 {
            data_length / row_size
        } else {
            0
        };
        Ok(Rows {
            table: self,
            reader: reader,
            num_rows: num_rows,
            next_row: 0,
        })
    }
}

// ========================================================================= //

/// An iterator over the rows in a table.
pub struct Rows<'a, R> {
    table: &'a Table,
    reader: R,
    num_rows: u64,
    next_row: u64,
}

impl<'a, R: Read + Seek> Rows<'a, R> {
    fn read_next_row(&mut self) -> io::Result<Vec<RowValue>> {
        let mut row = Vec::<RowValue>::with_capacity(self.table.columns.len());
        let mut offset: u64 = 0;
        for column in self.table.columns.iter() {
            let width = column.coltype.width(self.table.long_string_refs);
            self.reader
                .seek(SeekFrom::Start(offset * self.num_rows +
                                      width * self.next_row))?;
            row.push(column.coltype
                .read_value(&mut self.reader, self.table.long_string_refs)?);
            offset += width;
        }
        self.next_row += 1;
        Ok(row)
    }
}

impl<'a, R: Read + Seek> Iterator for Rows<'a, R> {
    type Item = io::Result<Vec<RowValue>>;

    fn next(&mut self) -> Option<io::Result<Vec<RowValue>>> {
        if self.next_row >= self.num_rows {
            return None;
        }
        Some(self.read_next_row())
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining_rows: u64 = self.num_rows - self.next_row;
        if remaining_rows > (usize::MAX as u64) {
            (usize::MAX, None)
        } else {
            let remaining_rows = remaining_rows as usize;
            (remaining_rows, Some(remaining_rows))
        }
    }
}

// ========================================================================= //
