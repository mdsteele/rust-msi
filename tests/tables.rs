extern crate msi;

#[macro_use]
mod testutil;

use msi::{Column, ColumnType, Package, PackageType};
use std::error::Error;
use std::io::{Cursor, ErrorKind};

// ========================================================================= //

#[test]
fn invalid_table_name() {
    let cursor = Cursor::new(Vec::new());
    let mut package = Package::create(PackageType::Installer, cursor).unwrap();
    let columns = vec![Column::build("Foo").primary_key().int32()];
    assert_error!(package.create_table("Foo & Bar".to_string(), columns),
                  ErrorKind::InvalidInput,
                  "\"Foo & Bar\" is not a valid table name");
}

#[test]
fn table_with_no_columns() {
    let cursor = Cursor::new(Vec::new());
    let mut package = Package::create(PackageType::Installer, cursor).unwrap();
    assert_error!(package.create_table("FooBar".to_string(), vec![]),
                  ErrorKind::InvalidInput,
                  "Cannot create a table with no columns");
}

#[test]
fn table_with_no_primary_key() {
    let cursor = Cursor::new(Vec::new());
    let mut package = Package::create(PackageType::Installer, cursor).unwrap();
    let columns = vec![Column::build("Foo").int32()];
    assert_error!(package.create_table("FooBar".to_string(), columns),
                  ErrorKind::InvalidInput,
                  "Cannot create a table without at least one primary key \
                   column");
}

#[test]
fn duplicate_table() {
    let cursor = Cursor::new(Vec::new());
    let mut package = Package::create(PackageType::Installer, cursor).unwrap();
    let columns = vec![Column::build("Foo").primary_key().int32()];
    package.create_table("FooBar".to_string(), columns).unwrap();
    let columns = vec![Column::build("Bar").primary_key().int32()];
    assert_error!(package.create_table("FooBar".to_string(), columns),
                  ErrorKind::AlreadyExists,
                  "Table \"FooBar\" already exists");
}

#[test]
fn table_with_invalid_column_name() {
    let cursor = Cursor::new(Vec::new());
    let mut package = Package::create(PackageType::Installer, cursor).unwrap();
    let columns = vec![Column::build("Foo & Bar").primary_key().int32()];
    assert_error!(package.create_table("FooBar".to_string(), columns),
                  ErrorKind::InvalidInput,
                  "\"Foo & Bar\" is not a valid column name");
}

#[test]
fn table_with_duplicate_column_names() {
    let cursor = Cursor::new(Vec::new());
    let mut package = Package::create(PackageType::Installer, cursor).unwrap();
    let columns = vec![
        Column::build("Foo").primary_key().int32(),
        Column::build("Bar").int16(),
        Column::build("Foo").string(6),
    ];
    assert_error!(package.create_table("FooBar".to_string(), columns),
                  ErrorKind::InvalidInput,
                  "Cannot create a table with multiple columns with the \
                   same name (\"Foo\")");
}

#[test]
fn create_valid_table() {
    let cursor = Cursor::new(Vec::new());
    let mut package = Package::create(PackageType::Installer, cursor).unwrap();
    let columns =
        vec![
            Column::build("Number").primary_key().range(0, 100).int16(),
            Column::build("Word").nullable().string(50),
        ];
    package.create_table("Numbers".to_string(), columns).unwrap();
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

// ========================================================================= //
