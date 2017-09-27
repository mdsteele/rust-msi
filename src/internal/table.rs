use byteorder::{LittleEndian, ReadBytesExt};
use internal::streamname;
use internal::stringpool::{StringPool, StringRef};
use std::io::{self, Read, Seek, SeekFrom};
use std::usize;

// ========================================================================= //

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RowValue {
    Uint(u32),
    Str(StringRef),
}

impl RowValue {
    pub fn to_string(&self, string_pool: &StringPool) -> String {
        match *self {
            RowValue::Uint(value) => format!("{:x}", value),
            RowValue::Str(string_ref) => {
                string_pool.get(string_ref).to_string()
            }
        }
    }
}

// ========================================================================= //

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ColumnType {
    Uint16,
    Uint32,
    Str(usize),
}

impl ColumnType {
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

pub struct Column {
    name: String,
    coltype: ColumnType,
}

impl Column {
    pub fn new(name: String, coltype: ColumnType) -> Column {
        Column {
            name: name,
            coltype: coltype,
        }
    }

    pub fn name(&self) -> &str { &self.name }
}

// ========================================================================= //

pub struct Table {
    name: String,
    columns: Vec<Column>,
    long_string_refs: bool,
}

impl Table {
    pub fn new(name: String, columns: Vec<Column>, long_string_refs: bool)
               -> Table {
        Table {
            name: name,
            columns: columns,
            long_string_refs: long_string_refs,
        }
    }

    pub fn name(&self) -> &str { &self.name }

    pub fn encoded_name(&self) -> String {
        streamname::encode(&self.name, true)
    }

    pub fn columns(&self) -> &[Column] { &self.columns }

    pub fn read_rows<R: Read + Seek>(&self, mut reader: R)
                                     -> io::Result<Rows<R>> {
        // TODO: We have to SeekFrom::End(0) twice due to a bug in the cfb
        // crate.
        reader.seek(SeekFrom::End(0))?;
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
