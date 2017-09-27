use cfb;
use internal::streamname;
use internal::stringpool::{StringPool, StringPoolBuilder};
use internal::summary::SummaryInfo;
use internal::table::{Column, ColumnType, Table};
use std::io::{self, Read, Seek, Write};

// ========================================================================= //

const COLUMNS_TABLE_NAME: &str = "_Columns";
const TABLES_TABLE_NAME: &str = "_Tables";
const STRING_DATA_TABLE_NAME: &str = "_StringData";
const STRING_POOL_TABLE_NAME: &str = "_StringPool";
const SUMMARY_INFO_STREAM_NAME: &str = "\u{5}SummaryInformation";

// ========================================================================= //

fn columns_table(long_string_refs: bool) -> Table {
    Table::new(COLUMNS_TABLE_NAME.to_string(),
               vec![Column::new("Table".to_string(), ColumnType::Str(64)),
                    Column::new("Number".to_string(), ColumnType::Uint16),
                    Column::new("Name".to_string(), ColumnType::Str(64)),
                    Column::new("Type".to_string(), ColumnType::Uint16)],
               long_string_refs)
}

fn tables_table(long_string_refs: bool) -> Table {
    Table::new(TABLES_TABLE_NAME.to_string(),
               vec![Column::new("Name".to_string(), ColumnType::Str(64))],
               long_string_refs)
}

// ========================================================================= //

/// An MSI package file, backed by an underlying reader/writer (such as a
/// [`File`](https://doc.rust-lang.org/std/fs/struct.File.html) or
/// [`Cursor`](https://doc.rust-lang.org/std/io/struct.Cursor.html)).
pub struct Package<F> {
    comp: cfb::CompoundFile<F>,
}

impl<F> Package<F> {
    /// Consumes the `Package` object, returning the underlying reader/writer.
    pub fn into_inner(self) -> F { self.comp.into_inner() }
}

impl<F: Read + Seek> Package<F> {
    /// Opens an existing MSI file, using the underlying reader.  If the
    /// underlying reader also supports the `Write` trait, then the `Package`
    /// object will be writable as well.
    pub fn open(inner: F) -> io::Result<Package<F>> {
        let comp = cfb::CompoundFile::open(inner)?;
        Ok(Package { comp: comp })
    }

    /// Parses the summary information from the MSI package.
    pub fn get_summary_info(&mut self) -> io::Result<SummaryInfo> {
        SummaryInfo::read(self.comp.open_stream(SUMMARY_INFO_STREAM_NAME)?)
    }

    /// Parses the string pool from the MSI package.
    pub fn get_string_pool(&mut self) -> io::Result<StringPool> {
        let builder = {
            let name = streamname::encode(STRING_POOL_TABLE_NAME, true);
            let stream = self.comp.open_stream(name)?;
            StringPoolBuilder::read_from_pool(stream)?
        };
        let name = streamname::encode(STRING_DATA_TABLE_NAME, true);
        let stream = self.comp.open_stream(name)?;
        builder.build_from_data(stream)
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

    /// Returns the names of the database tables in this package.
    pub fn table_names(&mut self) -> io::Result<Vec<String>> {
        let string_pool = self.get_string_pool()?;
        let table = tables_table(string_pool.long_string_refs());
        let stream = self.comp.open_stream(table.encoded_name())?;
        let mut names = Vec::new();
        for row in table.read_rows(stream)? {
            names.push(row?[0].to_string(&string_pool));
        }
        Ok(names)
    }

    /// Temporary helper function for testing.
    pub fn print_column_info(&mut self) -> io::Result<()> {
        let string_pool = self.get_string_pool()?;
        let table = columns_table(string_pool.long_string_refs());
        println!("##### {} #####", table.name());
        {
            let columns = table.columns();
            println!("{:24} {:6} {:24} {:4}",
                     columns[0].name(),
                     columns[1].name(),
                     columns[2].name(),
                     columns[3].name());
            println!("------------------------ ------ \
                      ------------------------ ----");
        }
        let stream = self.comp.open_stream(table.encoded_name())?;
        for row in table.read_rows(stream)? {
            let row = row?;
            println!("{:24} {:6} {:24} {:4}",
                     row[0].to_string(&string_pool),
                     row[1].to_string(&string_pool),
                     row[2].to_string(&string_pool),
                     row[3].to_string(&string_pool));
        }
        Ok(())
    }
}

impl<F: Read + Write + Seek> Package<F> {
    /// Overwrites the package's summary information.
    pub fn set_summary_info(&mut self, summary_info: &SummaryInfo)
                            -> io::Result<()> {
        summary_info.write(self.comp.create_stream(SUMMARY_INFO_STREAM_NAME)?)
    }
}

// ========================================================================= //
