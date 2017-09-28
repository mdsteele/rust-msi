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
    summary_info: SummaryInfo,
    is_summary_info_modified: bool,
    string_pool: StringPool,
    is_string_pool_modified: bool,
    finisher: Option<Box<Finish<F>>>,
}

impl<F> Package<F> {
    /// Returns summary information for this package.
    pub fn summary_info(&self) -> &SummaryInfo { &self.summary_info }

    /// Returns the string pool for this package.
    pub fn string_pool(&self) -> &StringPool { &self.string_pool }
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
        Ok(Package {
            comp: comp,
            summary_info: summary_info,
            is_summary_info_modified: false,
            string_pool: string_pool,
            is_string_pool_modified: false,
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

    /// Returns the names of the database tables in this package.
    pub fn table_names(&mut self) -> io::Result<Vec<String>> {
        let table = tables_table(self.string_pool.long_string_refs());
        let stream = self.comp.open_stream(table.encoded_name())?;
        let mut names = Vec::new();
        for row in table.read_rows(stream)? {
            names.push(row?[0].to_string(&self.string_pool));
        }
        Ok(names)
    }

    /// Temporary helper function for testing.
    pub fn print_column_info(&mut self) -> io::Result<()> {
        let table = columns_table(self.string_pool.long_string_refs());
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
                     row[0].to_string(&self.string_pool),
                     row[1].to_string(&self.string_pool),
                     row[2].to_string(&self.string_pool),
                     row[3].to_string(&self.string_pool));
        }
        Ok(())
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
                let stream = package.comp.open_stream(name)?;
                package.string_pool.write_pool(stream)?;
            }
            {
                let name = streamname::encode(STRING_DATA_TABLE_NAME, true);
                let stream = package.comp.open_stream(name)?;
                package.string_pool.write_data(stream)?;
            }
            package.is_string_pool_modified = false;
        }
        Ok(())
    }
}

// ========================================================================= //
