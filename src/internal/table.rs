use crate::internal::category::Category;
use crate::internal::column::Column;
use crate::internal::streamname;
use crate::internal::stringpool::StringPool;
use crate::internal::value::{Value, ValueRef};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::ops::Index;
use std::rc::Rc;

// ========================================================================= //

/// A database table.
#[derive(Clone)]
pub struct Table {
    name: String,
    columns: Vec<Column>,
    long_string_refs: bool,
}

impl Table {
    /// Creates a new table object with the given name and columns.  The
    /// `long_string_refs` argument indicates the size of any encoded string
    /// refs.
    pub(crate) fn new(
        name: String,
        columns: Vec<Column>,
        long_string_refs: bool,
    ) -> Rc<Table> {
        Rc::new(Table { name, columns, long_string_refs })
    }

    /// Returns the name of the table.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the name of the CFB stream that holds this table's data.
    pub(crate) fn stream_name(&self) -> String {
        streamname::encode(&self.name, true)
    }

    /// Returns true if the given string is a valid table name.
    pub(crate) fn is_valid_name(name: &str) -> bool {
        Category::Identifier.validate(name) && streamname::is_valid(name, true)
    }

    pub(crate) fn long_string_refs(&self) -> bool {
        self.long_string_refs
    }

    /// Returns the list of columns in this table.
    pub fn columns(&self) -> &[Column] {
        &self.columns
    }

    /// Returns true if this table has a column with the given name.
    pub fn has_column(&self, column_name: &str) -> bool {
        self.index_for_column_name(column_name).is_some()
    }

    /// Returns the column with the given name, if any.
    pub fn get_column(&self, column_name: &str) -> Option<&Column> {
        match self.index_for_column_name(column_name) {
            Some(index) => Some(&self.columns[index]),
            None => None,
        }
    }

    /// Returns the indices of table's primary key columns.
    pub fn primary_key_indices(&self) -> Vec<usize> {
        self.columns
            .iter()
            .enumerate()
            .filter_map(|(index, column)| {
                if column.is_primary_key() {
                    Some(index)
                } else {
                    None
                }
            })
            .collect()
    }

    pub(crate) fn index_for_column_name(
        &self,
        column_name: &str,
    ) -> Option<usize> {
        for (index, column) in self.columns.iter().enumerate() {
            if column.name() == column_name {
                return Some(index);
            }
        }
        None
    }

    /// Parses row data from the given data source and returns an interator
    /// over the rows.
    pub(crate) fn read_rows<R: Read + Seek>(
        &self,
        mut reader: R,
    ) -> io::Result<Vec<Vec<ValueRef>>> {
        let data_length = reader.seek(SeekFrom::End(0))?;
        reader.rewind()?;
        let row_size = self
            .columns
            .iter()
            .map(|col| col.coltype().width(self.long_string_refs))
            .sum::<u64>();
        let num_columns = self.columns.len();
        let num_rows =
            if row_size > 0 { (data_length / row_size) as usize } else { 0 };
        let mut rows =
            vec![Vec::<ValueRef>::with_capacity(num_columns); num_rows];
        for column in self.columns.iter() {
            let coltype = column.coltype();
            for row in rows.iter_mut() {
                row.push(
                    coltype.read_value(&mut reader, self.long_string_refs)?,
                );
            }
        }
        Ok(rows)
    }

    pub(crate) fn write_rows<W: Write>(
        &self,
        mut writer: W,
        rows: Vec<Vec<ValueRef>>,
    ) -> io::Result<()> {
        for (index, column) in self.columns.iter().enumerate() {
            let coltype = column.coltype();
            for row in rows.iter() {
                coltype.write_value(
                    &mut writer,
                    row[index],
                    self.long_string_refs,
                )?;
            }
        }
        Ok(())
    }
}

// ========================================================================= //

/// One row from a database table.
#[derive(Clone)]
pub struct Row {
    table: Rc<Table>,
    values: Vec<Value>,
}

impl Row {
    pub(crate) fn new(table: Rc<Table>, values: Vec<Value>) -> Row {
        debug_assert_eq!(values.len(), table.columns().len());
        Row { table, values }
    }

    /// Returns the number of values in the row.
    pub fn len(&self) -> usize {
        self.values.len()
    }

    /// Returns values in the row is empty
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns the list of columns in this row.
    pub fn columns(&self) -> &[Column] {
        self.table.columns()
    }

    /// Returns true if this row has a column with the given name.
    pub fn has_column(&self, column_name: &str) -> bool {
        self.table.has_column(column_name)
    }
}

/// Gets the value of the column with the given index.  Panics if `index >=
/// self.len()`.
impl Index<usize> for Row {
    type Output = Value;

    fn index(&self, index: usize) -> &Value {
        debug_assert_eq!(self.values.len(), self.table.columns().len());
        if index < self.values.len() {
            &self.values[index]
        } else if self.table.name.is_empty() {
            panic!(
                "Anonymous table has only {} columns (index was {index})",
                self.values.len()
            );
        } else {
            panic!(
                "Table {:?} has only {} columns (index was {index})",
                self.table.name,
                self.values.len()
            );
        }
    }
}

/// Gets the value of the column with the given name.  Panics if
/// `!self.has_column(column_name)`.
impl<'a> Index<&'a str> for Row {
    type Output = Value;

    fn index(&self, column_name: &str) -> &Value {
        match self.table.index_for_column_name(column_name) {
            Some(index) => &self.values[index],
            None => {
                if self.table.name.is_empty() {
                    panic!(
                        "Anonymous table has no column named {column_name:?}"
                    );
                } else {
                    panic!(
                        "Table {:?} has no column named {column_name:?}",
                        self.table.name
                    );
                }
            }
        }
    }
}

// ========================================================================= //

/// An iterator over the rows in a database table.
pub struct Rows<'a> {
    string_pool: &'a StringPool,
    table: Rc<Table>,
    rows: Vec<Vec<ValueRef>>,
    next_row_index: usize,
}

impl<'a> Rows<'a> {
    pub(crate) fn new(
        string_pool: &'a StringPool,
        table: Rc<Table>,
        rows: Vec<Vec<ValueRef>>,
    ) -> Rows<'a> {
        Rows { table, string_pool, rows, next_row_index: 0 }
    }

    /// Returns the list of columns for these rows.
    pub fn columns(&self) -> &[Column] {
        self.table.columns()
    }

    pub(crate) fn into_table_and_values(
        self,
    ) -> (Rc<Table>, Vec<Vec<ValueRef>>) {
        (self.table, self.rows)
    }
}

impl<'a> Iterator for Rows<'a> {
    type Item = Row;

    fn next(&mut self) -> Option<Row> {
        if self.next_row_index < self.rows.len() {
            let values: Vec<Value> = self.rows[self.next_row_index]
                .iter()
                .map(|value_ref| value_ref.to_value(self.string_pool))
                .collect();
            self.next_row_index += 1;
            Some(Row::new(self.table.clone(), values))
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
    use super::Table;

    #[test]
    fn valid_table_name() {
        assert!(Table::is_valid_name("fooBar"));
        assert!(Table::is_valid_name("_Validation"));
        assert!(Table::is_valid_name("Catch22"));
        assert!(Table::is_valid_name("Foo.Bar"));

        assert!(!Table::is_valid_name(""));
        assert!(!Table::is_valid_name("99Bottles"));
        assert!(!Table::is_valid_name(
            "ThisStringIsWayTooLongToBeATableNameIMeanSeriouslyWhoWouldTryTo\
             UseANameThatIsThisLongItWouldBePrettySilly"
        ));
    }
}

// ========================================================================= //
