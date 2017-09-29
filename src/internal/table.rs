use byteorder::{LittleEndian, ReadBytesExt};
use internal::streamname;
use internal::stringpool::{StringPool, StringRef};
use std::io::{self, Read, Seek, SeekFrom};
use std::usize;

// ========================================================================= //

/// A value from one cell in a table row.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RowValue {
    Uint(u32),
    Str(StringRef),
}

impl RowValue {
    /// Formats the value as a string, using the given string pool to look up
    /// string references.  Integer values will be formatted in hexadecimal.
    pub fn to_string(&self, string_pool: &StringPool) -> String {
        match *self {
            RowValue::Uint(value) => format!("{:x}", value),
            RowValue::Str(string_ref) => {
                string_pool.get(string_ref).to_string()
            }
        }
    }

    /// Returns the value as an integer.  For string values, this will return
    /// the string reference number.
    pub fn to_u32(&self) -> u32 {
        match *self {
            RowValue::Uint(value) => value,
            RowValue::Str(string_ref) => string_ref.number(),
        }
    }
}

// ========================================================================= //

/// A column data type.
#[allow(dead_code)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ColumnType {
    /// A 16-bit integer.
    Uint16,
    /// A 32-bit integer.
    Uint32,
    /// A string, with the specified maximum length.
    Str(usize),
}

impl ColumnType {
    fn from_bitfield(type_bits: u32) -> ColumnType {
        let field_size = (type_bits & 0xff) as usize;
        if (type_bits & 0x800) != 0 {
            ColumnType::Str(field_size)
        } else if field_size == 2 {
            ColumnType::Uint16
        } else {
            ColumnType::Uint32
        }
    }

    fn read_value<R: Read>(&self, reader: &mut R, long_string_refs: bool)
                           -> io::Result<RowValue> {
        match *self {
            ColumnType::Uint16 => {
                Ok(RowValue::Uint(reader.read_u16::<LittleEndian>()? as u32))
            }
            ColumnType::Uint32 => {
                Ok(RowValue::Uint(reader.read_u32::<LittleEndian>()?))
            }
            ColumnType::Str(_) => {
                Ok(RowValue::Str(StringRef::read(reader, long_string_refs)?))
            }
        }
    }

    fn width(&self, long_string_refs: bool) -> u64 {
        match *self {
            ColumnType::Uint16 => 2,
            ColumnType::Uint32 => 4,
            ColumnType::Str(_) => if long_string_refs { 3 } else { 2 },
        }
    }
}

// ========================================================================= //

/// A database column.
pub struct Column {
    name: String,
    coltype: ColumnType,
    is_key: bool,
}

impl Column {
    /// Creates a new column object with the given name, type, and primary key
    /// status.
    pub fn new(name: &str, coltype: ColumnType, is_key: bool) -> Column {
        Column {
            name: name.to_string(),
            coltype: coltype,
            is_key: is_key,
        }
    }

    /// Creates a new column object with the given name, and with other
    /// attributes determened from the given bitfield (taken from the
    /// `_Columns` table).
    pub fn from_bitfield(name: String, type_bits: u32) -> Column {
        Column {
            name: name,
            coltype: ColumnType::from_bitfield(type_bits & 0x8ff),
            is_key: (type_bits & 0x2000) != 0,
        }
    }

    /// Returns the name of the column.
    pub fn name(&self) -> &str { &self.name }

    /// Returns the type of data stored in the column.
    pub fn coltype(&self) -> ColumnType { self.coltype }

    /// Returns true if this is primary key column, false otherwise.
    pub fn is_key(&self) -> bool { self.is_key }
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
