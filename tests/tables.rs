#[macro_use]
mod testutil;

use msi::{
    Column, ColumnType, Expr, Insert, Package, PackageType, Select, Value,
};
use std::io::{Cursor, ErrorKind};

// ========================================================================= //

#[test]
fn create_table_with_invalid_name() {
    let cursor = Cursor::new(Vec::new());
    let mut package = Package::create(PackageType::Installer, cursor).unwrap();
    let columns = vec![Column::build("Foo").primary_key().int32()];
    assert_error!(
        package.create_table("Foo & Bar", columns),
        ErrorKind::InvalidInput,
        "\"Foo & Bar\" is not a valid table name"
    );
    assert!(!package.has_table("Foo & Bar"));
}

#[test]
fn create_table_with_no_columns() {
    let cursor = Cursor::new(Vec::new());
    let mut package = Package::create(PackageType::Installer, cursor).unwrap();
    assert_error!(
        package.create_table("FooBar", vec![]),
        ErrorKind::InvalidInput,
        "Cannot create a table with no columns"
    );
    assert!(!package.has_table("FooBar"));
}

#[test]
fn create_table_with_no_primary_key() {
    let cursor = Cursor::new(Vec::new());
    let mut package = Package::create(PackageType::Installer, cursor).unwrap();
    let columns = vec![Column::build("Foo").int32()];
    assert_error!(
        package.create_table("FooBar", columns),
        ErrorKind::InvalidInput,
        "Cannot create a table without at least one primary key \
                   column"
    );
    assert!(!package.has_table("FooBar"));
}

#[test]
fn create_duplicate_table() {
    let cursor = Cursor::new(Vec::new());
    let mut package = Package::create(PackageType::Installer, cursor).unwrap();
    let columns = vec![Column::build("Foo").primary_key().int32()];
    package.create_table("FooBar", columns).unwrap();
    assert!(package.has_table("FooBar"));
    let columns = vec![Column::build("Bar").primary_key().int32()];
    assert_error!(
        package.create_table("FooBar", columns),
        ErrorKind::AlreadyExists,
        "Table \"FooBar\" already exists"
    );
    assert!(package.has_table("FooBar"));
}

#[test]
fn create_table_with_invalid_column_name() {
    let cursor = Cursor::new(Vec::new());
    let mut package = Package::create(PackageType::Installer, cursor).unwrap();
    let columns = vec![Column::build("Foo & Bar").primary_key().int32()];
    assert_error!(
        package.create_table("FooBar", columns),
        ErrorKind::InvalidInput,
        "\"Foo & Bar\" is not a valid column name"
    );
    assert!(!package.has_table("FooBar"));
}

#[test]
fn create_table_with_duplicate_column_names() {
    let cursor = Cursor::new(Vec::new());
    let mut package = Package::create(PackageType::Installer, cursor).unwrap();
    let columns = vec![
        Column::build("Foo").primary_key().int32(),
        Column::build("Bar").int16(),
        Column::build("Foo").string(6),
    ];
    assert_error!(
        package.create_table("FooBar", columns),
        ErrorKind::InvalidInput,
        "Cannot create a table with multiple columns with the \
                   same name (\"Foo\")"
    );
    assert!(!package.has_table("FooBar"));
}

#[test]
fn create_valid_table() {
    let cursor = Cursor::new(Vec::new());
    let mut package = Package::create(PackageType::Installer, cursor).unwrap();
    let columns = vec![
        Column::build("Number").primary_key().range(0, 100).int16(),
        Column::build("Word").nullable().string(50),
    ];
    package.create_table("Numbers", columns).unwrap();
    assert!(package.has_table("Numbers"));

    let cursor = package.into_inner().unwrap();
    let package = Package::open(cursor).unwrap();
    assert!(package.has_table("Numbers"));
    let table = package.get_table("Numbers").unwrap();
    assert_eq!(table.name(), "Numbers");

    assert!(table.has_column("Number"));
    let column = table.get_column("Number").unwrap();
    assert_eq!(column.name(), "Number");
    assert_eq!(column.coltype(), ColumnType::Int16);
    assert!(column.is_primary_key());
    assert!(!column.is_nullable());
    assert_eq!(column.value_range(), Some((0, 100)));

    assert!(table.has_column("Word"));
    let column = table.get_column("Word").unwrap();
    assert_eq!(column.name(), "Word");
    assert_eq!(column.coltype(), ColumnType::Str(50));
    assert!(!column.is_primary_key());
    assert!(column.is_nullable());
}

#[test]
fn drop_table_with_invalid_name() {
    let cursor = Cursor::new(Vec::new());
    let mut package = Package::create(PackageType::Installer, cursor).unwrap();
    assert_error!(
        package.drop_table("Foo & Bar"),
        ErrorKind::InvalidInput,
        "\"Foo & Bar\" is not a valid table name"
    );
}

#[test]
fn drop_nonexistent_table() {
    let cursor = Cursor::new(Vec::new());
    let mut package = Package::create(PackageType::Installer, cursor).unwrap();
    assert!(!package.has_table("FooBar"));
    assert_error!(
        package.drop_table("FooBar"),
        ErrorKind::NotFound,
        "Table \"FooBar\" does not exist"
    );
}

#[test]
fn drop_special_tables() {
    let cursor = Cursor::new(Vec::new());
    let mut package = Package::create(PackageType::Installer, cursor).unwrap();
    assert_error!(
        package.drop_table("_Columns"),
        ErrorKind::InvalidInput,
        "Cannot drop special \"_Columns\" table"
    );
    assert!(package.has_table("_Columns"));
    assert_error!(
        package.drop_table("_Tables"),
        ErrorKind::InvalidInput,
        "Cannot drop special \"_Tables\" table"
    );
    assert!(package.has_table("_Tables"));
    assert_error!(
        package.drop_table("_Validation"),
        ErrorKind::InvalidInput,
        "Cannot drop special \"_Validation\" table"
    );
    assert!(package.has_table("_Validation"));
}

#[test]
fn drop_valid_table() {
    // Create a package with a table, and verify that that the special tables
    // have entries for that new table.
    let table_name = "Numbers";
    let cursor = Cursor::new(Vec::new());
    let mut package = Package::create(PackageType::Installer, cursor).unwrap();
    let columns = vec![
        Column::build("Number").primary_key().range(0, 100).int16(),
        Column::build("Word").nullable().string(50),
    ];
    package.create_table(table_name, columns).unwrap();
    assert!(package.has_table(table_name));
    let query = Select::table("_Tables")
        .with(Expr::col("Name").eq(Expr::string(table_name)));
    assert_eq!(package.select_rows(query).unwrap().len(), 1);
    let query = Select::table("_Columns")
        .with(Expr::col("Table").eq(Expr::string(table_name)));
    assert_eq!(package.select_rows(query).unwrap().len(), 2);
    let query = Select::table("_Validation")
        .with(Expr::col("Table").eq(Expr::string(table_name)));
    assert_eq!(package.select_rows(query).unwrap().len(), 2);

    // Reopen the package, and drop the table.  Confirm that the special tables
    // no longer have entries for the deleted table.
    let cursor = package.into_inner().unwrap();
    let mut package = Package::open(cursor).unwrap();
    assert!(package.has_table(table_name));
    package.drop_table(table_name).unwrap();
    assert!(!package.has_table(table_name));
    let query = Select::table("_Tables")
        .with(Expr::col("Name").eq(Expr::string(table_name)));
    assert_eq!(package.select_rows(query).unwrap().len(), 0);
    let query = Select::table("_Columns")
        .with(Expr::col("Table").eq(Expr::string(table_name)));
    assert_eq!(package.select_rows(query).unwrap().len(), 0);
    let query = Select::table("_Validation")
        .with(Expr::col("Table").eq(Expr::string(table_name)));
    assert_eq!(package.select_rows(query).unwrap().len(), 0);

    // Reopen the package again, and make sure the table still isn't there.
    let cursor = package.into_inner().unwrap();
    let package = Package::open(cursor).unwrap();
    assert!(!package.has_table(table_name));
}

#[test]
fn drop_table_with_rows() {
    // Create a package with a table, and add a few rows to that table.
    let table_name = "Numbers";
    let cursor = Cursor::new(Vec::new());
    let mut package = Package::create(PackageType::Installer, cursor).unwrap();
    let columns = vec![
        Column::build("Number").primary_key().range(0, 100).int16(),
        Column::build("Word").nullable().string(50),
    ];
    package.create_table(table_name, columns.clone()).unwrap();
    assert!(package.has_table(table_name));
    let query = Insert::into(table_name)
        .row(vec![Value::Int(4), Value::from("Four")])
        .row(vec![Value::Int(7), Value::from("Seven")])
        .row(vec![Value::Int(10), Value::from("Ten")]);
    package.insert_rows(query).unwrap();
    let query = Select::table(table_name);
    assert_eq!(package.select_rows(query).unwrap().len(), 3);

    // Reopen the package, and drop the table.
    let cursor = package.into_inner().unwrap();
    let mut package = Package::open(cursor).unwrap();
    assert!(package.has_table(table_name));
    package.drop_table(table_name).unwrap();
    assert!(!package.has_table(table_name));

    // Reopen the package again, and recreate the same table.  The rows should
    // not reappear.
    let cursor = package.into_inner().unwrap();
    let mut package = Package::open(cursor).unwrap();
    assert!(!package.has_table(table_name));
    package.create_table(table_name, columns).unwrap();
    let query = Select::table(table_name);
    assert_eq!(package.select_rows(query).unwrap().len(), 0);
}

// ========================================================================= //
