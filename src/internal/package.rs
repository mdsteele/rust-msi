use cfb;
use internal::streamname;
use internal::stringpool::{StringPool, StringPoolBuilder};
use internal::summary::SummaryInfo;
use internal::table::{Column, ColumnType, RowValue, Table};
use std::collections::{BTreeMap, btree_map};
use std::io::{self, Read, Seek, Write};

// ========================================================================= //

const COLUMNS_TABLE_NAME: &str = "_Columns";
const TABLES_TABLE_NAME: &str = "_Tables";
const STRING_DATA_TABLE_NAME: &str = "_StringData";
const STRING_POOL_TABLE_NAME: &str = "_StringPool";
const SUMMARY_INFO_STREAM_NAME: &str = "\u{5}SummaryInformation";

// ========================================================================= //

fn columns_table(long_string_refs: bool) -> Table {
    Table::new(
        COLUMNS_TABLE_NAME.to_string(),
        vec![
            Column::new("Table", ColumnType::Str(64), true),
            Column::new("Number", ColumnType::Int16, true),
            Column::new("Name", ColumnType::Str(64), false),
            Column::new("Type", ColumnType::Int16, false),
        ],
        long_string_refs,
    )
}

fn tables_table(long_string_refs: bool) -> Table {
    Table::new(TABLES_TABLE_NAME.to_string(),
               vec![Column::new("Name", ColumnType::Str(64), true)],
               long_string_refs)
}

// ========================================================================= //

/// An MSI package file, backed by an underlying reader/writer (such as a
/// [`File`](https://doc.rust-lang.org/std/fs/struct.File.html) or
/// [`Cursor`](https://doc.rust-lang.org/std/io/struct.Cursor.html)).
pub struct Package<F> {
    comp: cfb::CompoundFile<F>,
    summary_info: SummaryInfo,
    is_summary_info_modified: bool,
    string_pool: StringPool,
    is_string_pool_modified: bool,
    tables: BTreeMap<String, Table>,
    finisher: Option<Box<Finish<F>>>,
}

impl<F> Package<F> {
    /// Returns summary information for this package.
    pub fn summary_info(&self) -> &SummaryInfo { &self.summary_info }

    /// Returns the string pool for this package.
    pub fn string_pool(&self) -> &StringPool { &self.string_pool }

    /// Returns the table with the given name (if any).
    pub fn table(&self, table_name: &str) -> Option<&Table> {
        self.tables.get(table_name)
    }

    /// Returns an iterator over the tables in this package.
    pub fn tables(&self) -> Tables { Tables(self.tables.values()) }
}

impl<F: Read + Seek> Package<F> {
    /// Opens an existing MSI file, using the underlying reader.  If the
    /// underlying reader also supports the `Write` trait, then the `Package`
    /// object will be writable as well.
    pub fn open(inner: F) -> io::Result<Package<F>> {
        let mut comp = cfb::CompoundFile::open(inner)?;
        let summary_info =
            SummaryInfo::read(comp.open_stream(SUMMARY_INFO_STREAM_NAME)?)?;
        let string_pool = {
            let builder = {
                let name = streamname::encode(STRING_POOL_TABLE_NAME, true);
                let stream = comp.open_stream(name)?;
                StringPoolBuilder::read_from_pool(stream)?
            };
            let name = streamname::encode(STRING_DATA_TABLE_NAME, true);
            let stream = comp.open_stream(name)?;
            builder.build_from_data(stream)?
        };
        let mut all_tables = BTreeMap::<String, Table>::new();
        let table_names: Vec<String> = {
            let table = tables_table(string_pool.long_string_refs());
            let stream = comp.open_stream(table.stream_name())?;
            let mut names = Vec::<String>::new();
            for row in table.read_rows(stream)? {
                names.push(row?[0].to_string(&string_pool));
            }
            all_tables.insert(table.name().to_string(), table);
            names
        };
        {
            let table = columns_table(string_pool.long_string_refs());
            let stream = comp.open_stream(table.stream_name())?;
            let mut columns_map: BTreeMap<String,
                                          BTreeMap<i32, Column>> =
                table_names
                    .into_iter()
                    .map(|name| (name, BTreeMap::new()))
                    .collect();
            for row in table.read_rows(stream)? {
                let row = row?;
                let table_name = row[0].to_string(&string_pool);
                if let Some(cols) = columns_map.get_mut(&table_name) {
                    let col_index = row[1].to_i32();
                    if cols.contains_key(&col_index) {
                        invalid_data!("Repeat in _Columns: {:?} column {}",
                                      table_name,
                                      col_index);
                    }
                    let col_name = row[2].to_string(&string_pool);
                    let type_bits = row[3].to_i32();
                    let column = Column::from_bitfield(col_name, type_bits);
                    cols.insert(col_index, column);
                } else {
                    invalid_data!("_Columns mentions table {:?}, which \
                                   isn't in _Tables",
                                  table_name);
                }
            }
            all_tables.insert(table.name().to_string(), table);
            for (table_name, columns) in columns_map.into_iter() {
                if columns.is_empty() {
                    invalid_data!("No columns found for table {:?}",
                                  table_name);
                }
                let num_columns = columns.len() as i32;
                if columns.keys().next() != Some(&1) ||
                    columns.keys().next_back() != Some(&num_columns)
                {
                    invalid_data!("Table {:?} does not have a complete set \
                                   of columns",
                                  table_name);
                }
                let columns: Vec<Column> =
                    columns.into_iter().map(|(_, column)| column).collect();
                let table = Table::new(table_name,
                                       columns,
                                       string_pool.long_string_refs());
                all_tables.insert(table.name().to_string(), table);
            }
        }
        Ok(Package {
               comp: comp,
               summary_info: summary_info,
               is_summary_info_modified: false,
               string_pool: string_pool,
               is_string_pool_modified: false,
               tables: all_tables,
               finisher: None,
           })
    }

    /// Temporary helper function for testing.
    pub fn print_entries(&self) -> io::Result<()> {
        for entry in self.comp.read_storage("/")? {
            let (name, is_table) = streamname::decode(entry.name());
            let prefix = if is_table { "T" } else { " " };
            println!("{} {:?}", prefix, name);
        }
        Ok(())
    }

    /// Read and return all rows from a table.
    pub fn read_table_rows(&mut self, table_name: &str)
                           -> io::Result<Vec<Vec<RowValue>>> {
        if let Some(table) = self.tables.get(table_name) {
            let stream = self.comp.open_stream(table.stream_name())?;
            let mut rows = Vec::<Vec<RowValue>>::new();
            for row in table.read_rows(stream)? {
                rows.push(row?);
            }
            Ok(rows)
        } else {
            not_found!("Table {:?} does not exist", table_name);
        }
    }
}

impl<F: Read + Write + Seek> Package<F> {
    /// Returns a mutable reference to the summary information for this
    /// package.  Call `flush()` or drop the `Package` object to persist any
    /// changes made to the underlying writer.
    pub fn summary_info_mut(&mut self) -> &mut SummaryInfo {
        self.is_summary_info_modified = true;
        self.set_finisher();
        &mut self.summary_info
    }

    /// Flushes any buffered changes to the underlying writer.
    pub fn flush(&mut self) -> io::Result<()> {
        if let Some(finisher) = self.finisher.take() {
            finisher.finish(self)?;
        }
        self.comp.flush()
    }

    fn set_finisher(&mut self) {
        if self.finisher.is_none() {
            let finisher: Box<Finish<F>> = Box::new(FinishImpl {});
            self.finisher = Some(finisher);
        }
    }
}

impl<F> Drop for Package<F> {
    fn drop(&mut self) {
        if let Some(finisher) = self.finisher.take() {
            let _ = finisher.finish(self);
        }
    }
}

// ========================================================================= //

/// An iterator over the database tables in a package.
#[derive(Clone)]
pub struct Tables<'a>(btree_map::Values<'a, String, Table>);

impl<'a> Iterator for Tables<'a> {
    type Item = <btree_map::Values<'a, String, Table> as Iterator>::Item;

    fn next(&mut self) -> Option<Self::Item> {
        let Tables(ref mut iter) = *self;
        iter.next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let Tables(ref iter) = *self;
        iter.size_hint()
    }
}

impl<'a> ExactSizeIterator for Tables<'a> {}

// ========================================================================= //

trait Finish<F> {
    fn finish(&self, package: &mut Package<F>) -> io::Result<()>;
}

struct FinishImpl {}

impl<F: Read + Write + Seek> Finish<F> for FinishImpl {
    fn finish(&self, package: &mut Package<F>) -> io::Result<()> {
        if package.is_summary_info_modified {
            let stream = package.comp.create_stream(SUMMARY_INFO_STREAM_NAME)?;
            package.summary_info.write(stream)?;
            package.is_summary_info_modified = false;
        }
        if package.is_string_pool_modified {
            {
                let name = streamname::encode(STRING_POOL_TABLE_NAME, true);
                let stream = package.comp.create_stream(name)?;
                package.string_pool.write_pool(stream)?;
            }
            {
                let name = streamname::encode(STRING_DATA_TABLE_NAME, true);
                let stream = package.comp.create_stream(name)?;
                package.string_pool.write_data(stream)?;
            }
            package.is_string_pool_modified = false;
        }
        Ok(())
    }
}

// ========================================================================= //
