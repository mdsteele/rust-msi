use crate::internal::category::Category;
use crate::internal::codepage::CodePage;
use crate::internal::column::Column;
use crate::internal::expr::Expr;
use crate::internal::query::{Delete, Insert, Select, Update};
use crate::internal::stream::{StreamReader, StreamWriter, Streams};
use crate::internal::streamname::{
    self, DIGITAL_SIGNATURE_STREAM_NAME, MSI_DIGITAL_SIGNATURE_EX_STREAM_NAME,
    SUMMARY_INFO_STREAM_NAME,
};
use crate::internal::stringpool::{StringPool, StringPoolBuilder};
use crate::internal::summary::SummaryInfo;
use crate::internal::table::{Rows, Table};
use crate::internal::value::{Value, ValueRef};
use cfb;
use std::borrow::Borrow;
use std::collections::{btree_map, BTreeMap, HashMap, HashSet};
use std::io::{self, Read, Seek, Write};
use std::rc::Rc;
use uuid::Uuid;

// ========================================================================= //

const INSTALLER_PACKAGE_CLSID: &str = "000C1084-0000-0000-C000-000000000046";
const PATCH_PACKAGE_CLSID: &str = "000C1086-0000-0000-C000-000000000046";
const TRANSFORM_PACKAGE_CLSID: &str = "000C1082-0000-0000-C000-000000000046";

const COLUMNS_TABLE_NAME: &str = "_Columns";
const TABLES_TABLE_NAME: &str = "_Tables";
const VALIDATION_TABLE_NAME: &str = "_Validation";

const STRING_DATA_TABLE_NAME: &str = "_StringData";
const STRING_POOL_TABLE_NAME: &str = "_StringPool";

const MAX_NUM_TABLE_COLUMNS: usize = 32;

// ========================================================================= //

fn make_columns_table(long_string_refs: bool) -> Rc<Table> {
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

fn make_tables_table(long_string_refs: bool) -> Rc<Table> {
    Table::new(
        TABLES_TABLE_NAME.to_string(),
        vec![Column::build("Name").primary_key().string(64)],
        long_string_refs,
    )
}

fn make_validation_columns() -> Vec<Column> {
    let min = -0x7fff_ffff;
    let max = 0x7fff_ffff;
    let values: Vec<&str> =
        Category::all().iter().map(Category::as_str).collect();
    vec![
        Column::build("Table").primary_key().id_string(32),
        Column::build("Column").primary_key().id_string(32),
        Column::build("Nullable").enum_values(&["Y", "N"]).string(4),
        Column::build("MinValue").nullable().range(min, max).int32(),
        Column::build("MaxValue").nullable().range(min, max).int32(),
        Column::build("KeyTable").nullable().id_string(255),
        Column::build("KeyColumn").nullable().range(1, 32).int16(),
        Column::build("Category").nullable().enum_values(&values).string(32),
        Column::build("Set").nullable().text_string(255),
        Column::build("Description").nullable().text_string(255),
    ]
}

fn make_validation_table(long_string_refs: bool) -> Rc<Table> {
    Table::new(
        VALIDATION_TABLE_NAME.to_string(),
        make_validation_columns(),
        long_string_refs,
    )
}

fn is_reserved_table_name(table_name: &str) -> bool {
    table_name == COLUMNS_TABLE_NAME
        || table_name == TABLES_TABLE_NAME
        || table_name == VALIDATION_TABLE_NAME
}

// ========================================================================= //

/// The type of MSI package (e.g. installer or patch).
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
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
///
/// # Examples
///
/// ```
/// use msi::{Column, Expr, Insert, Package, PackageType, Select, Value};
/// use std::io::Cursor;
///
/// // Create an in-memory package using a Cursor:
/// let cursor = Cursor::new(Vec::new());
/// let mut package = Package::create(PackageType::Installer, cursor).unwrap();
/// // Set some summary information:
/// package.summary_info_mut().set_author("Jane Doe".to_string());
/// // Add a table to the package:
/// let columns = vec![
///     Column::build("Property").primary_key().id_string(72),
///     Column::build("Value").nullable().formatted_string(64),
/// ];
/// package.create_table("CheckBox", columns).unwrap();
/// // Add a row to the new table:
/// let query = Insert::into("CheckBox").row(vec![
///     Value::from("MoreMagic"),
///     Value::from("Whether magic should be maximized"),
/// ]);
/// package.insert_rows(query).unwrap();
/// // Close the package and get the cursor back out.
/// let cursor = package.into_inner().unwrap();
///
/// // Now, re-open the package and make sure our data is still there.
/// let mut package = Package::open(cursor).unwrap();
/// assert_eq!(package.summary_info().author(), Some("Jane Doe"));
/// let query = Select::table("CheckBox")
///     .with(Expr::col("Property").eq(Expr::string("MoreMagic")));
/// let mut rows = package.select_rows(query).unwrap();
/// assert_eq!(rows.len(), 1);
/// let row = rows.next().unwrap();
/// assert_eq!(row["Property"], Value::Str("MoreMagic".to_string()));
/// assert_eq!(row["Value"],
///            Value::Str("Whether magic should be maximized".to_string()));
/// ```
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
    tables: BTreeMap<String, Rc<Table>>,
    finisher: Option<Box<dyn Finish<F>>>,
}

impl<F> Package<F> {
    /// Returns what type of package this is.
    pub fn package_type(&self) -> PackageType {
        self.package_type
    }

    /// Returns summary information for this package.
    pub fn summary_info(&self) -> &SummaryInfo {
        &self.summary_info
    }

    /// Returns the code page used for serializing strings in the database.
    pub fn database_codepage(&self) -> CodePage {
        self.string_pool.codepage()
    }

    /// Returns true if the database has a table with the given name.
    pub fn has_table(&self, table_name: &str) -> bool {
        self.tables.contains_key(table_name)
    }

    /// Returns the database table with the given name (if any).
    pub fn get_table(&self, table_name: &str) -> Option<&Table> {
        self.tables.get(table_name).map(Rc::borrow)
    }

    /// Returns an iterator over the database tables in this package.
    pub fn tables(&self) -> Tables {
        Tables { iter: self.tables.values() }
    }

    /// Returns true if the package has an embedded binary stream with the
    /// given name.
    pub fn has_stream(&self, stream_name: &str) -> bool {
        self.comp().is_stream(streamname::encode(stream_name, false))
    }

    /// Returns an iterator over the embedded binary streams in this package.
    pub fn streams(&self) -> Streams<F> {
        Streams::new(self.comp().read_root_storage())
    }

    /// Returns true if the package has been digitally signed.  Note that this
    /// method only checks whether a signature is present; it does *not* verify
    /// that the signature is actually valid.
    pub fn has_digital_signature(&self) -> bool {
        self.comp().is_stream(DIGITAL_SIGNATURE_STREAM_NAME)
    }

    /// Consumes the `Package` object, returning the underlying reader/writer.
    pub fn into_inner(mut self) -> io::Result<F> {
        if let Some(finisher) = self.finisher.take() {
            finisher.finish(&mut self)?;
        }
        Ok(self.comp.take().unwrap().into_inner())
    }

    fn comp(&self) -> &cfb::CompoundFile<F> {
        self.comp.as_ref().unwrap()
    }

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
                None => invalid_data!(
                    "Unrecognized package CLSID ({})",
                    clsid.hyphenated()
                ),
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
        let mut all_tables = BTreeMap::<String, Rc<Table>>::new();
        // Read in _Tables table:
        let table_names: HashSet<String> = {
            let table = make_tables_table(string_pool.long_string_refs());
            let stream_name = table.stream_name();
            let mut names = HashSet::<String>::new();
            if comp.exists(&stream_name) {
                let stream = comp.open_stream(&stream_name)?;
                let rows = Rows::new(
                    &string_pool,
                    table.clone(),
                    table.read_rows(stream)?,
                );
                for row in rows {
                    let table_name = row[0].as_str().unwrap().to_string();
                    if names.contains(&table_name) {
                        invalid_data!(
                            "Repeated key in {:?} table: {:?}",
                            TABLES_TABLE_NAME,
                            table_name
                        );
                    }
                    names.insert(table_name);
                }
            }
            all_tables.insert(table.name().to_string(), table);
            names
        };
        // Read in _Columns table:
        let mut columns_map: HashMap<String, BTreeMap<i32, (String, i32)>> =
            table_names
                .into_iter()
                .map(|name| (name, BTreeMap::new()))
                .collect();
        {
            let table = make_columns_table(string_pool.long_string_refs());
            let stream_name = table.stream_name();
            if comp.exists(&stream_name) {
                let stream = comp.open_stream(&stream_name)?;
                let rows = Rows::new(
                    &string_pool,
                    table.clone(),
                    table.read_rows(stream)?,
                );
                for row in rows {
                    let table_name = row[0].as_str().unwrap();
                    if let Some(cols) = columns_map.get_mut(table_name) {
                        let col_index = row[1].as_int().unwrap();
                        if cols.contains_key(&col_index) {
                            invalid_data!(
                                "Repeated key in {:?} table: {:?}",
                                COLUMNS_TABLE_NAME,
                                (table_name, col_index)
                            );
                        }
                        let col_name = row[2].as_str().unwrap().to_string();
                        let type_bits = row[3].as_int().unwrap();
                        cols.insert(col_index, (col_name, type_bits));
                    } else {
                        invalid_data!(
                            "_Columns mentions table {:?}, which isn't in \
                             _Tables",
                            table_name
                        );
                    }
                }
            }
            all_tables.insert(table.name().to_string(), table);
        }
        // Read in _Validation table:
        let mut validation_map =
            HashMap::<(String, String), Vec<ValueRef>>::new();
        {
            let table = make_validation_table(string_pool.long_string_refs());
            // TODO: Ensure that columns_map["_Validation"].columns() matches
            // the hard-coded validation table definition.
            let stream_name = table.stream_name();
            if comp.exists(&stream_name) {
                let stream = comp.open_stream(&stream_name)?;
                for value_refs in table.read_rows(stream)?.into_iter() {
                    let table_name = value_refs[0]
                        .to_value(&string_pool)
                        .as_str()
                        .unwrap()
                        .to_string();
                    let column_name = value_refs[1]
                        .to_value(&string_pool)
                        .as_str()
                        .unwrap()
                        .to_string();
                    let key = (table_name, column_name);
                    if validation_map.contains_key(&key) {
                        invalid_data!(
                            "Repeated key in {:?} table: {:?}",
                            VALIDATION_TABLE_NAME,
                            key
                        );
                    }
                    validation_map.insert(key, value_refs);
                }
            }
        }
        // Construct Table objects from column/validation data:
        for (table_name, column_specs) in columns_map.into_iter() {
            if column_specs.is_empty() {
                invalid_data!("No columns found for table {:?}", table_name);
            }
            let num_columns = column_specs.len() as i32;
            if column_specs.keys().next() != Some(&1)
                || column_specs.keys().next_back() != Some(&num_columns)
            {
                invalid_data!(
                    "Table {:?} does not have a complete set of columns",
                    table_name
                );
            }
            let mut columns = Vec::<Column>::with_capacity(column_specs.len());
            for (_, (column_name, bitfield)) in column_specs.into_iter() {
                let mut builder = Column::build(column_name.as_str());
                let key = (table_name.clone(), column_name);
                if let Some(value_refs) = validation_map.get(&key) {
                    let is_nullable = value_refs[2].to_value(&string_pool);
                    if is_nullable.as_str().unwrap() == "Y" {
                        builder = builder.nullable();
                    }
                    let min_value = value_refs[3].to_value(&string_pool);
                    let max_value = value_refs[4].to_value(&string_pool);
                    if !min_value.is_null() && !max_value.is_null() {
                        let min = min_value.as_int().unwrap();
                        let max = max_value.as_int().unwrap();
                        builder = builder.range(min, max);
                    }
                    let key_table = value_refs[5].to_value(&string_pool);
                    let key_column = value_refs[6].to_value(&string_pool);
                    if !key_table.is_null() && !key_column.is_null() {
                        builder = builder.foreign_key(
                            key_table.as_str().unwrap(),
                            key_column.as_int().unwrap(),
                        );
                    }
                    let category_value = value_refs[7].to_value(&string_pool);
                    if !category_value.is_null() {
                        let category = category_value
                            .as_str()
                            .unwrap()
                            .parse::<Category>()
                            .ok();
                        if let Some(category) = category {
                            builder = builder.category(category);
                        }
                    }
                    let enum_values = value_refs[8].to_value(&string_pool);
                    if !enum_values.is_null() {
                        let enum_values: Vec<&str> =
                            enum_values.as_str().unwrap().split(';').collect();
                        builder = builder.enum_values(&enum_values);
                    }
                }
                columns.push(builder.with_bitfield(bitfield)?);
            }
            let table = Table::new(
                table_name,
                columns,
                string_pool.long_string_refs(),
            );
            all_tables.insert(table.name().to_string(), table);
        }
        Ok(Package {
            comp: Some(comp),
            package_type,
            summary_info,
            is_summary_info_modified: false,
            string_pool,
            tables: all_tables,
            finisher: None,
        })
    }

    /// Attempts to execute a select query.  Returns an error if the query
    /// fails (e.g. due to the column names being incorrect or the table(s) not
    /// existing).
    pub fn select_rows(&mut self, query: Select) -> io::Result<Rows> {
        query.exec(
            self.comp.as_mut().unwrap(),
            &self.string_pool,
            &self.tables,
        )
    }

    /// Opens an existing binary stream in the package for reading.
    pub fn read_stream(
        &mut self,
        stream_name: &str,
    ) -> io::Result<StreamReader<F>> {
        if !streamname::is_valid(stream_name, false) {
            invalid_input!("{:?} is not a valid stream name", stream_name);
        }
        let encoded_name = streamname::encode(stream_name, false);
        if !self.comp().is_stream(&encoded_name) {
            not_found!("Stream {:?} does not exist", stream_name);
        }
        Ok(StreamReader::new(self.comp_mut().open_stream(&encoded_name)?))
    }

    // TODO: pub fn has_valid_digital_signature(&mut self) -> io::Result<bool>
}

impl<F: Read + Write + Seek> Package<F> {
    /// Creates a new, empty package of the given type, using the underlying
    /// reader/writer.  The reader/writer should be initially empty.
    pub fn create(
        package_type: PackageType,
        inner: F,
    ) -> io::Result<Package<F>> {
        let mut comp = cfb::CompoundFile::create(inner)?;
        comp.set_storage_clsid("/", package_type.clsid())?;
        let mut summary_info = SummaryInfo::new();
        summary_info.set_title(package_type.default_title().to_string());
        let string_pool = StringPool::new(summary_info.codepage());
        let tables = {
            let mut tables = BTreeMap::<String, Rc<Table>>::new();
            let table = make_tables_table(string_pool.long_string_refs());
            tables.insert(table.name().to_string(), table);
            let table = make_columns_table(string_pool.long_string_refs());
            tables.insert(table.name().to_string(), table);
            tables
        };
        let mut package = Package {
            comp: Some(comp),
            package_type,
            summary_info,
            is_summary_info_modified: true,
            string_pool,
            tables,
            finisher: None,
        };
        package
            .create_table(VALIDATION_TABLE_NAME, make_validation_columns())?;
        package.flush()?;
        debug_assert!(!package.is_summary_info_modified);
        debug_assert!(!package.string_pool.is_modified());
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

    /// Sets the code page used for serializing strings in the database.
    pub fn set_database_codepage(&mut self, codepage: CodePage) {
        self.set_finisher();
        self.string_pool.set_codepage(codepage)
    }

    /// Creates a new database table.  Returns an error without modifying the
    /// database if the table name or columns are invalid, or if a table with
    /// that name already exists.
    pub fn create_table<S: Into<String>>(
        &mut self,
        table_name: S,
        columns: Vec<Column>,
    ) -> io::Result<()> {
        self.create_table_with_name(table_name.into(), columns)
    }

    fn create_table_with_name(
        &mut self,
        table_name: String,
        columns: Vec<Column>,
    ) -> io::Result<()> {
        if !Table::is_valid_name(&table_name) {
            invalid_input!("{:?} is not a valid table name", table_name);
        }
        if columns.is_empty() {
            invalid_input!("Cannot create a table with no columns");
        }
        if columns.len() > MAX_NUM_TABLE_COLUMNS {
            invalid_input!(
                "Cannot create a table with more than {} columns",
                MAX_NUM_TABLE_COLUMNS
            );
        }
        if !columns.iter().any(Column::is_primary_key) {
            invalid_input!(
                "Cannot create a table without at least one primary key column"
            );
        }
        {
            let mut column_names = HashSet::<&str>::new();
            for column in columns.iter() {
                let name = column.name();
                if !Column::is_valid_name(name) {
                    invalid_input!("{:?} is not a valid column name", name);
                }
                if column_names.contains(name) {
                    invalid_input!(
                        "Cannot create a table with multiple columns with the \
                         same name ({:?})",
                        name
                    );
                }
                column_names.insert(name);
            }
        }
        if self.tables.contains_key(&table_name) {
            already_exists!("Table {:?} already exists", table_name);
        }
        self.insert_rows(
            Insert::into(COLUMNS_TABLE_NAME).rows(
                columns
                    .iter()
                    .enumerate()
                    .map(|(index, column)| {
                        vec![
                            Value::Str(table_name.clone()),
                            Value::Int(1 + index as i32),
                            Value::Str(column.name().to_string()),
                            Value::Int(column.bitfield()),
                        ]
                    })
                    .collect(),
            ),
        )?;
        self.insert_rows(
            Insert::into(TABLES_TABLE_NAME)
                .row(vec![Value::Str(table_name.clone())]),
        )?;
        let validation_rows: Vec<Vec<Value>> = columns
            .iter()
            .map(|column| {
                let (min_value, max_value) =
                    if let Some((min, max)) = column.value_range() {
                        (Value::Int(min), Value::Int(max))
                    } else {
                        (Value::Null, Value::Null)
                    };
                let (key_table, key_column) =
                    if let Some((table, column)) = column.foreign_key() {
                        (Value::Str(table.to_string()), Value::Int(column))
                    } else {
                        (Value::Null, Value::Null)
                    };
                vec![
                    Value::Str(table_name.clone()),
                    Value::Str(column.name().to_string()),
                    Value::Str(if column.is_nullable() {
                        "Y".to_string()
                    } else {
                        "N".to_string()
                    }),
                    min_value,
                    max_value,
                    key_table,
                    key_column,
                    if let Some(category) = column.category() {
                        Value::Str(category.to_string())
                    } else {
                        Value::Null
                    },
                    if let Some(values) = column.enum_values() {
                        Value::Str(values.join(";"))
                    } else {
                        Value::Null
                    },
                    Value::Null,
                ]
            })
            .collect();
        let long_string_refs = self.string_pool.long_string_refs();
        let table = Table::new(table_name.clone(), columns, long_string_refs);
        self.tables.insert(table_name, table);
        self.insert_rows(
            Insert::into(VALIDATION_TABLE_NAME).rows(validation_rows),
        )?;
        Ok(())
    }

    /// Removes an existing database table.  Returns an error without modifying
    /// the database if the table name is invalid, or if no such table exists.
    pub fn drop_table(&mut self, table_name: &str) -> io::Result<()> {
        if is_reserved_table_name(table_name) {
            invalid_input!("Cannot drop special {:?} table", table_name);
        }
        if !Table::is_valid_name(table_name) {
            invalid_input!("{:?} is not a valid table name", table_name);
        }
        if !self.tables.contains_key(table_name) {
            not_found!("Table {:?} does not exist", table_name);
        }
        let stream_name = self.tables.get(table_name).unwrap().stream_name();
        if self.comp().exists(&stream_name) {
            self.comp_mut().remove_stream(&stream_name)?;
        }
        self.delete_rows(
            Delete::from(VALIDATION_TABLE_NAME)
                .with(Expr::col("Table").eq(Expr::string(table_name))),
        )?;
        self.delete_rows(
            Delete::from(COLUMNS_TABLE_NAME)
                .with(Expr::col("Table").eq(Expr::string(table_name))),
        )?;
        self.delete_rows(
            Delete::from(TABLES_TABLE_NAME)
                .with(Expr::col("Name").eq(Expr::string(table_name))),
        )?;
        self.tables.remove(table_name);
        Ok(())
    }

    /// Attempts to execute a delete query.  Returns an error without modifying
    /// the database if the query fails (e.g. due to the table not existing).
    pub fn delete_rows(&mut self, query: Delete) -> io::Result<()> {
        self.set_finisher();
        query.exec(
            self.comp.as_mut().unwrap(),
            &mut self.string_pool,
            &self.tables,
        )
    }

    /// Attempts to execute an insert query.  Returns an error without
    /// modifying the database if the query fails (e.g. due to values being
    /// invalid, or keys not being unique, or the table not existing).
    pub fn insert_rows(&mut self, query: Insert) -> io::Result<()> {
        self.set_finisher();
        query.exec(
            self.comp.as_mut().unwrap(),
            &mut self.string_pool,
            &self.tables,
        )
    }

    /// Attempts to execute an update query.  Returns an error without
    /// modifying the database if the query fails (e.g. due to values being
    /// invalid, or column names being incorrect, or the table not existing).
    pub fn update_rows(&mut self, query: Update) -> io::Result<()> {
        self.set_finisher();
        query.exec(
            self.comp.as_mut().unwrap(),
            &mut self.string_pool,
            &self.tables,
        )
    }

    /// Creates (or overwrites) a binary stream in the package.
    pub fn write_stream(
        &mut self,
        stream_name: &str,
    ) -> io::Result<StreamWriter<F>> {
        if !streamname::is_valid(stream_name, false) {
            invalid_input!("{:?} is not a valid stream name", stream_name);
        }
        let encoded_name = streamname::encode(stream_name, false);
        Ok(StreamWriter::new(self.comp_mut().create_stream(&encoded_name)?))
    }

    /// Removes an existing binary stream from the package.
    pub fn remove_stream(&mut self, stream_name: &str) -> io::Result<()> {
        if !streamname::is_valid(stream_name, false) {
            invalid_input!("{:?} is not a valid stream name", stream_name);
        }
        let encoded_name = streamname::encode(stream_name, false);
        if !self.comp().is_stream(&encoded_name) {
            not_found!("Stream {:?} does not exist", stream_name);
        }
        self.comp_mut().remove_stream(&encoded_name)
    }

    // TODO: pub fn add_digital_signature(&mut self, ...) -> io::Result<()>

    /// Removes any existing digital signature from the package.  This can be
    /// useful if you need to modify a signed package (which will invalidate
    /// the signature).
    pub fn remove_digital_signature(&mut self) -> io::Result<()> {
        if self.comp().is_stream(DIGITAL_SIGNATURE_STREAM_NAME) {
            self.comp_mut().remove_stream(DIGITAL_SIGNATURE_STREAM_NAME)?;
        }
        if self.comp().is_stream(MSI_DIGITAL_SIGNATURE_EX_STREAM_NAME) {
            self.comp_mut()
                .remove_stream(MSI_DIGITAL_SIGNATURE_EX_STREAM_NAME)?;
        }
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
            let finisher: Box<dyn Finish<F>> = Box::new(FinishImpl {});
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
///
/// No guarantees are made about the order in which items are returned.
#[derive(Clone)]
pub struct Tables<'a> {
    iter: btree_map::Values<'a, String, Rc<Table>>,
}

impl<'a> Iterator for Tables<'a> {
    type Item = &'a Table;

    fn next(&mut self) -> Option<&'a Table> {
        self.iter.next().map(Rc::borrow)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
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
        if package.string_pool.is_modified() {
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
            package.string_pool.mark_unmodified();
        }
        Ok(())
    }
}

// ========================================================================= //

#[cfg(test)]
mod tests {
    use super::{Package, PackageType};
    use crate::internal::codepage::CodePage;
    use crate::internal::column::Column;
    use crate::internal::expr::Expr;
    use crate::internal::query::{Insert, Select, Update};
    use crate::internal::value::Value;
    use std::io::Cursor;

    #[test]
    fn set_database_codepage() {
        let cursor = Cursor::new(Vec::new());
        let mut package =
            Package::create(PackageType::Installer, cursor).expect("create");
        assert_eq!(package.database_codepage(), CodePage::Utf8);
        package.set_database_codepage(CodePage::MacintoshRoman);
        assert_eq!(package.database_codepage(), CodePage::MacintoshRoman);

        let cursor = package.into_inner().expect("into_inner");
        let package = Package::open(cursor).expect("open");
        assert_eq!(package.database_codepage(), CodePage::MacintoshRoman);
    }

    #[test]
    fn insert_rows() {
        let cursor = Cursor::new(Vec::new());
        let mut package =
            Package::create(PackageType::Installer, cursor).expect("create");
        let columns = vec![
            Column::build("Number").primary_key().int16(),
            Column::build("Word").nullable().string(50),
        ];
        package.create_table("Numbers", columns).expect("create_table");
        package
            .insert_rows(
                Insert::into("Numbers")
                    .row(vec![Value::Int(2), Value::Str("Two".to_string())])
                    .row(vec![Value::Int(4), Value::Str("Four".to_string())])
                    .row(vec![Value::Int(1), Value::Str("One".to_string())]),
            )
            .expect("insert_rows");
        assert_eq!(
            package
                .select_rows(Select::table("Numbers"))
                .expect("select")
                .len(),
            3
        );

        let cursor = package.into_inner().expect("into_inner");
        let mut package = Package::open(cursor).expect("open");
        let rows = package.select_rows(Select::table("Numbers")).unwrap();
        assert_eq!(rows.len(), 3);
        let values: Vec<(i32, String)> = rows
            .map(|row| {
                (
                    row[0].as_int().unwrap(),
                    row[1].as_str().unwrap().to_string(),
                )
            })
            .collect();
        assert_eq!(
            values,
            vec![
                (1, "One".to_string()),
                (2, "Two".to_string()),
                (4, "Four".to_string()),
            ]
        );
    }

    #[test]
    fn update_rows() {
        let cursor = Cursor::new(Vec::new());
        let mut package =
            Package::create(PackageType::Installer, cursor).expect("create");
        let columns = vec![
            Column::build("Key").primary_key().int16(),
            Column::build("Value").nullable().int32(),
        ];
        package.create_table("Mapping", columns).expect("create_table");
        package
            .insert_rows(
                Insert::into("Mapping")
                    .row(vec![Value::Int(1), Value::Int(17)])
                    .row(vec![Value::Int(2), Value::Int(42)])
                    .row(vec![Value::Int(3), Value::Int(17)]),
            )
            .expect("insert_rows");
        package
            .update_rows(
                Update::table("Mapping")
                    .set("Value", Value::Int(-5))
                    .with(Expr::col("Value").eq(Expr::integer(17))),
            )
            .unwrap();

        let cursor = package.into_inner().expect("into_inner");
        let mut package = Package::open(cursor).expect("open");
        let rows = package.select_rows(Select::table("Mapping")).unwrap();
        let values: Vec<(i32, i32)> = rows
            .map(|row| (row[0].as_int().unwrap(), row[1].as_int().unwrap()))
            .collect();
        assert_eq!(values, vec![(1, -5), (2, 42), (3, -5)]);
    }
}

// ========================================================================= //
