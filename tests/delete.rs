#[macro_use]
mod testutil;

use msi::{Column, Delete, Expr, Insert, Package, PackageType, Select, Value};
use std::io::{Cursor, ErrorKind};

// ========================================================================= //

#[test]
fn nonexistent_table() {
    let cursor = Cursor::new(Vec::new());
    let mut package = Package::create(PackageType::Installer, cursor).unwrap();
    let query =
        Delete::from("Foobar").with(Expr::col("Foo").eq(Expr::integer(1)));
    assert_error!(
        package.delete_rows(query),
        ErrorKind::NotFound,
        "Table \"Foobar\" does not exist"
    );
}

#[test]
fn nonexistent_column() {
    let cursor = Cursor::new(Vec::new());
    let mut package = Package::create(PackageType::Installer, cursor).unwrap();
    let columns = vec![
        Column::build("Foo").primary_key().int32(),
        Column::build("Bar").nullable().string(6),
    ];
    package.create_table("Foobar", columns).unwrap();
    let query = Delete::from("Foobar").with(
        (Expr::col("Bar").ne(Expr::null()))
            .and(Expr::col("Baz").eq(Expr::integer(1))),
    );
    assert_error!(
        package.delete_rows(query),
        ErrorKind::InvalidInput,
        "Table \"Foobar\" has no column named \"Baz\""
    );
}

#[test]
fn delete_one_row() {
    let cursor = Cursor::new(Vec::new());
    let mut package = Package::create(PackageType::Installer, cursor).unwrap();
    let columns = vec![
        Column::build("Foo").primary_key().int32(),
        Column::build("Bar").nullable().string(6),
    ];
    package.create_table("Foobar", columns).unwrap();

    let query = Insert::into("Foobar")
        .row(vec![Value::Int(1), Value::from("One")])
        .row(vec![Value::Int(2), Value::from("Two")])
        .row(vec![Value::Int(3), Value::from("Three")]);
    package.insert_rows(query).unwrap();

    let query =
        Delete::from("Foobar").with(Expr::col("Foo").eq(Expr::integer(2)));
    package.delete_rows(query).unwrap();

    let cursor = package.into_inner().unwrap();
    let mut package = Package::open(cursor).unwrap();
    let rows = package.select_rows(Select::table("Foobar")).unwrap();
    assert_eq!(rows.len(), 2);
    let keys = rows.map(|row| row[0].as_int().unwrap()).collect::<Vec<i32>>();
    assert_eq!(keys, vec![1, 3]);
}

#[test]
fn delete_multiple_rows() {
    let cursor = Cursor::new(Vec::new());
    let mut package = Package::create(PackageType::Installer, cursor).unwrap();
    let columns = vec![
        Column::build("Key").primary_key().int16(),
        Column::build("Value").nullable().int32(),
    ];
    package.create_table("Mapping", columns).unwrap();

    let query = Insert::into("Mapping")
        .row(vec![Value::Int(1), Value::Int(17)])
        .row(vec![Value::Int(2), Value::Int(42)])
        .row(vec![Value::Int(3), Value::Int(17)]);
    package.insert_rows(query).unwrap();

    let query =
        Delete::from("Mapping").with(Expr::col("Value").eq(Expr::integer(17)));
    package.delete_rows(query).unwrap();

    let cursor = package.into_inner().unwrap();
    let mut package = Package::open(cursor).unwrap();
    let rows = package.select_rows(Select::table("Mapping")).unwrap();
    let values: Vec<(i32, i32)> = rows
        .map(|row| (row[0].as_int().unwrap(), row[1].as_int().unwrap()))
        .collect();
    assert_eq!(values, vec![(2, 42)]);
}

#[test]
fn delete_all_rows() {
    let cursor = Cursor::new(Vec::new());
    let mut package = Package::create(PackageType::Installer, cursor).unwrap();
    let columns = vec![
        Column::build("Foo").primary_key().int32(),
        Column::build("Bar").nullable().string(6),
    ];
    package.create_table("Foobar", columns).unwrap();

    let query = Insert::into("Foobar")
        .row(vec![Value::Int(1), Value::from("One")])
        .row(vec![Value::Int(2), Value::from("Two")])
        .row(vec![Value::Int(3), Value::from("Three")]);
    package.insert_rows(query).unwrap();

    package.delete_rows(Delete::from("Foobar")).unwrap();

    let cursor = package.into_inner().unwrap();
    let mut package = Package::open(cursor).unwrap();
    let rows = package.select_rows(Select::table("Foobar")).unwrap();
    assert_eq!(rows.len(), 0);
    let keys = rows.map(|row| row[0].as_int().unwrap()).collect::<Vec<i32>>();
    assert!(keys.is_empty());
}

// ========================================================================= //
