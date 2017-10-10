use cfb;
use internal::codepage::CodePage;
use internal::column::Column;
use internal::streamname;
use internal::stringpool::{StringPool, StringPoolBuilder};
use internal::summary::SummaryInfo;
use internal::table::{Rows, Table};
use internal::value::{Value, ValueRef};
use std::collections::{BTreeMap, btree_map};
use std::io::{self, Read, Seek, Write};
use uuid::Uuid;

// ========================================================================= //

const INSTALLER_PACKAGE_CLSID: &str = "000C1084-0000-0000-C000-000000000046";
const PATCH_PACKAGE_CLSID: &str = "000C1086-0000-0000-C000-000000000046";
const TRANSFORM_PACKAGE_CLSID: &str = "000C1082-0000-0000-C000-000000000046";

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
            Column::build("Table").primary_key().string(64),
            Column::build("Number").primary_key().int16(),
            Column::build("Name").string(64),
            Column::build("Type").int16(),
        ],
        long_string_refs,
    )
}

fn tables_table(long_string_refs: bool) -> Table {
    Table::new(TABLES_TABLE_NAME.to_string(),
               vec![Column::build("Name").primary_key().string(64)],
               long_string_refs)
}

// ========================================================================= //

/// The type of MSI package (e.g. installer or patch).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PackageType {
    /// An installer package, which installs a new application.
    Installer,
    /// A patch package, which provides an update to an application.
    Patch,
    /// A transform, which is a collection of changes applied to an
    /// installation.
    Transform,
}

impl PackageType {
    fn from_clsid(clsid: &Uuid) -> Option<PackageType> {
        if *clsid == PackageType::Installer.clsid() {
            Some(PackageType::Installer)
        } else if *clsid == PackageType::Patch.clsid() {
            Some(PackageType::Patch)
        } else if *clsid == PackageType::Transform.clsid() {
            Some(PackageType::Transform)
        } else {
            None
        }
    }

    fn clsid(&self) -> Uuid {
        match *self {
            PackageType::Installer => {
                Uuid::parse_str(INSTALLER_PACKAGE_CLSID).unwrap()
            }
            PackageType::Patch => {
                Uuid::parse_str(PATCH_PACKAGE_CLSID).unwrap()
            }
            PackageType::Transform => {
                Uuid::parse_str(TRANSFORM_PACKAGE_CLSID).unwrap()
            }
        }
    }

    fn default_title(&self) -> &str {
        match *self {
            PackageType::Installer => "Installation Database",
            PackageType::Patch => "Patch",
            PackageType::Transform => "Transform",
        }
    }
}

// ========================================================================= //

/// An MSI package file, backed by an underlying reader/writer (such as a
/// [`File`](https://doc.rust-lang.org/std/fs/struct.File.html) or
/// [`Cursor`](https://doc.rust-lang.org/std/io/struct.Cursor.html)).
pub struct Package<F> {
    // The comp field is always `Some`, unless we are about to destroy the
    // `Package` object.  The only reason for it to be an `Option` is to make
    // it possible for the `into_inner()` method to move the `CompoundFile` out
    // of the `Package` object, even though `Package` implements `Drop`
    // (normally you can't move fields out an object that implements `Drop`).
    comp: Option<cfb::CompoundFile<F>>,
    package_type: PackageType,
    summary_info: SummaryInfo,
    is_summary_info_modified: bool,
    string_pool: StringPool,
    is_string_pool_modified: bool,
    tables: BTreeMap<String, Table>,
    finisher: Option<Box<Finish<F>>>,
}

impl<F> Package<F> {
    /// Returns what type of package this is.
    pub fn package_type(&self) -> PackageType { self.package_type }

    /// Returns summary information for this package.
    pub fn summary_info(&self) -> &SummaryInfo { &self.summary_info }

    /// Returns the code page used for serializing strings in the database.
    pub fn database_codepage(&self) -> CodePage { self.string_pool.codepage() }

    // TODO: pub fn set_database_codepage

    /// Returns the database table with the given name (if any).
    pub fn table(&self, table_name: &str) -> Option<&Table> {
        self.tables.get(table_name)
    }

    /// Returns an iterator over the database tables in this package.
    pub fn tables(&self) -> Tables { Tables(self.tables.values()) }

    /// Consumes the `Package` object, returning the underlying reader/writer.
    pub fn into_inner(mut self) -> io::Result<F> {
        if let Some(finisher) = self.finisher.take() {
            finisher.finish(&mut self)?;
        }
        Ok(self.comp.take().unwrap().into_inner())
    }

    fn comp(&self) -> &cfb::CompoundFile<F> { self.comp.as_ref().unwrap() }

    fn comp_mut(&mut self) -> &mut cfb::CompoundFile<F> {
        self.comp.as_mut().unwrap()
    }
}

impl<F: Read + Seek> Package<F> {
    /// Opens an existing MSI file, using the underlying reader.  If the
    /// underlying reader also supports the `Write` trait, then the `Package`
    /// object will be writable as well.
    pub fn open(inner: F) -> io::Result<Package<F>> {
        let mut comp = cfb::CompoundFile::open(inner)?;
        let package_type = {
            let root_entry = comp.root_entry();
            let clsid = root_entry.clsid();
            match PackageType::from_clsid(clsid) {
                Some(ptype) => ptype,
                None => {
                    invalid_data!("Unrecognized package CLSID ({})",
                                  clsid.hyphenated())
                }
            }
        };
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
            let stream_name = table.stream_name();
            let mut names = Vec::<String>::new();
            if comp.exists(&stream_name) {
                let stream = comp.open_stream(&stream_name)?;
                let rows = table.read_rows(stream)?;
                for row in Rows::new(&string_pool, &table, rows) {
                    names.push(row[0].to_string());
                }
            }
            all_tables.insert(table.name().to_string(), table);
            names
        };
        {
            let table = columns_table(string_pool.long_string_refs());
            let stream_name = table.stream_name();
            let mut columns_map: BTreeMap<String,
                                          BTreeMap<i32, Column>> =
                table_names
                    .into_iter()
                    .map(|name| (name, BTreeMap::new()))
                    .collect();
            if comp.exists(&stream_name) {
                let stream = comp.open_stream(&stream_name)?;
                let rows = table.read_rows(stream)?;
                for row in Rows::new(&string_pool, &table, rows) {
                    let table_name = row[0].as_str().unwrap();
                    if let Some(cols) = columns_map.get_mut(table_name) {
                        let col_index = row[1].as_int().unwrap();
                        if cols.contains_key(&col_index) {
                            invalid_data!("Repeat in _Columns: {:?} column {}",
                                          table_name,
                                          col_index);
                        }
                        let col_name = row[2].to_string();
                        let type_bits = row[3].as_int().unwrap();
                        let column = Column::from_bitfield(col_name,
                                                           type_bits)?;
                        cols.insert(col_index, column);
                    } else {
                        invalid_data!("_Columns mentions table {:?}, which \
                                       isn't in _Tables",
                                      table_name);
                    }
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
               comp: Some(comp),
               package_type: package_type,
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
        for entry in self.comp().read_storage("/")? {
            let (name, is_table) = streamname::decode(entry.name());
            let prefix = if is_table { "T" } else { " " };
            println!("{} {:?}", prefix, name);
        }
        Ok(())
    }

    /// Read and return all rows from a table.
    pub fn read_table_rows(&mut self, table_name: &str) -> io::Result<Rows> {
        if let Some(table) = self.tables.get(table_name) {
            let stream_name = table.stream_name();
            let rows = {
                let comp = self.comp.as_mut().unwrap();
                if comp.exists(&stream_name) {
                    let stream = comp.open_stream(&stream_name)?;
                    table.read_rows(stream)?
                } else {
                    Vec::new()
                }
            };
            Ok(Rows::new(&self.string_pool, table, rows))
        } else {
            not_found!("Table {:?} does not exist", table_name);
        }
    }
}

impl<F: Read + Write + Seek> Package<F> {
    /// Creates a new, empty package of the given type, using the underlying
    /// reader/writer.  The reader/writer should be initially empty.
    pub fn create(package_type: PackageType, inner: F)
                  -> io::Result<Package<F>> {
        let mut comp = cfb::CompoundFile::create(inner)?;
        comp.set_storage_clsid("/", package_type.clsid())?;
        let mut summary_info = SummaryInfo::new();
        summary_info.set_title(package_type.default_title().to_string());
        let string_pool = StringPool::new(summary_info.codepage());
        let tables = {
            let mut tables = BTreeMap::<String, Table>::new();
            let table = tables_table(string_pool.long_string_refs());
            tables.insert(table.name().to_string(), table);
            let table = columns_table(string_pool.long_string_refs());
            tables.insert(table.name().to_string(), table);
            tables
        };
        let mut package = Package {
            comp: Some(comp),
            package_type: package_type,
            summary_info: summary_info,
            is_summary_info_modified: true,
            string_pool: string_pool,
            is_string_pool_modified: true,
            tables: tables,
            finisher: None,
        };
        package.set_finisher();
        // TODO: create _Validation table
        package.flush()?;
        debug_assert!(!package.is_summary_info_modified);
        debug_assert!(!package.is_string_pool_modified);
        Ok(package)
    }

    /// Returns a mutable reference to the summary information for this
    /// package.  Call `flush()` or drop the `Package` object to persist any
    /// changes made to the underlying writer.
    pub fn summary_info_mut(&mut self) -> &mut SummaryInfo {
        self.is_summary_info_modified = true;
        self.set_finisher();
        &mut self.summary_info
    }

    /// Inserts a new row into a table.  Returns an error without modifying the
    /// table if the values are of the wrong types (or otherwise invalid) for
    /// the table, or if the primary key values are not unique within the
    /// table, or if the table doesn't exist.
    pub fn insert_row(&mut self, table_name: &str, values: Vec<Value>)
                      -> io::Result<()> {
        if let Some(table) = self.tables.get(table_name) {
            // Validate the new row.
            if values.len() != table.columns().len() {
                invalid_input!("Table {:?} has {} columns, but {} values \
                                were provided",
                               table_name,
                               table.columns().len(),
                               values.len());
            }
            for (column, value) in table.columns().iter().zip(values.iter()) {
                if !column.is_valid_value(value) {
                    invalid_input!("{:?} is not a valid value for column {:?}",
                                   value,
                                   column.name());
                }
            }
            // Read in the rows from the table.
            let stream_name = table.stream_name();
            let key_indices = table.primary_key_indices();
            let mut rows = BTreeMap::<Vec<Value>, Vec<ValueRef>>::new();
            let comp = self.comp.as_mut().unwrap();
            if comp.exists(&stream_name) {
                let stream = comp.open_stream(&stream_name)?;
                for row in table.read_rows(stream)?.into_iter() {
                    let mut keys = Vec::with_capacity(key_indices.len());
                    for &index in key_indices.iter() {
                        keys.push(row[index].to_value(&self.string_pool));
                    }
                    if rows.contains_key(&keys) {
                        invalid_data!("Malformed table {:?} contains \
                                       multiple rows with key {:?}",
                                      table_name,
                                      keys);
                    }
                    rows.insert(keys, row);
                }
            }
            // Check if this row already exists in the table.
            let mut keys = Vec::with_capacity(key_indices.len());
            for &index in key_indices.iter() {
                keys.push(values[index].clone());
            }
            if rows.contains_key(&keys) {
                already_exists!("Table {:?} already contains a row with \
                                 key {:?}",
                                table_name,
                                keys);
            }
            // Insert the new row into the table.
            let mut row = Vec::<ValueRef>::with_capacity(values.len());
            for value in values.into_iter() {
                row.push(ValueRef::create(value, &mut self.string_pool));
            }
            rows.insert(keys, row);
            // Write table back out to the file.
            let rows: Vec<Vec<ValueRef>> =
                rows.into_iter().map(|(_, row)| row).collect();
            let stream = comp.create_stream(&stream_name)?;
            table.write_rows(stream, rows)?;
        } else {
            not_found!("Table {:?} does not exist", table_name);
        }
        self.set_finisher();
        Ok(())
    }

    /// Flushes any buffered changes to the underlying writer.
    pub fn flush(&mut self) -> io::Result<()> {
        if let Some(finisher) = self.finisher.take() {
            finisher.finish(self)?;
        }
        self.comp_mut().flush()
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
            let stream = package
                .comp
                .as_mut()
                .unwrap()
                .create_stream(SUMMARY_INFO_STREAM_NAME)?;
            package.summary_info.write(stream)?;
            package.is_summary_info_modified = false;
        }
        if package.is_string_pool_modified {
            {
                let name = streamname::encode(STRING_POOL_TABLE_NAME, true);
                let stream =
                    package.comp.as_mut().unwrap().create_stream(name)?;
                package.string_pool.write_pool(stream)?;
            }
            {
                let name = streamname::encode(STRING_DATA_TABLE_NAME, true);
                let stream =
                    package.comp.as_mut().unwrap().create_stream(name)?;
                package.string_pool.write_data(stream)?;
            }
            package.is_string_pool_modified = false;
        }
        Ok(())
    }
}

// ========================================================================= //

#[cfg(test)]
mod tests {
    use super::{Package, PackageType};
    use std::io::Cursor;

    #[test]
    fn set_summary_information() {
        let cursor = Cursor::new(Vec::new());
        let mut package = Package::create(PackageType::Installer, cursor)
            .expect("create");
        package.summary_info_mut().set_author("Jane Doe".to_string());

        let cursor = package.into_inner().unwrap();
        let package = Package::open(cursor).expect("open");
        assert_eq!(package.package_type(), PackageType::Installer);
        assert_eq!(package.summary_info().author(), Some("Jane Doe"));
    }
}

// ========================================================================= //
