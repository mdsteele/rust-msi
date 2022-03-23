#[macro_use]
mod testutil;

use msi::{Column, Expr, Insert, Package, PackageType, Select, Value};
use std::io::{Cursor, ErrorKind};

//===========================================================================//

#[test]
fn nonexistent_table() {
    let cursor = Cursor::new(Vec::new());
    let mut package = Package::create(PackageType::Installer, cursor).unwrap();
    let query =
        Select::table("Foobar").with(Expr::col("Foo").eq(Expr::integer(1)));
    assert_error!(
        package.select_rows(query),
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
    let query = Select::table("Foobar").with(
        (Expr::col("Bar").ne(Expr::null()))
            .and(Expr::col("Baz").eq(Expr::integer(1))),
    );
    assert_error!(
        package.select_rows(query),
        ErrorKind::InvalidInput,
        "Table \"Foobar\" has no column named \"Baz\""
    );
}

#[test]
fn select_rows() {
    let cursor = Cursor::new(Vec::new());
    let mut package = Package::create(PackageType::Installer, cursor).unwrap();

    let columns = vec![
        Column::build("Foo").primary_key().int16(),
        Column::build("Bar").string(16),
        Column::build("Baz").nullable().int32(),
    ];
    package.create_table("Quux", columns).unwrap();
    let query = Insert::into("Quux")
        .row(vec![
            Value::Int(1),
            Value::Str("spam".to_string()),
            Value::Int(0),
        ])
        .row(vec![Value::Int(2), Value::Str("eggs".to_string()), Value::Null])
        .row(vec![
            Value::Int(3),
            Value::Str("bacon".to_string()),
            Value::Int(0),
        ])
        .row(vec![
            Value::Int(4),
            Value::Str("spam".to_string()),
            Value::Int(17),
        ]);
    package.insert_rows(query).unwrap();

    let query = Select::table("Quux")
        .columns(&["Bar", "Foo"])
        .with(Expr::col("Baz").eq(Expr::integer(0)));
    let rows = package.select_rows(query).unwrap();
    let values: Vec<(String, i32)> = rows
        .map(|row| {
            (row[0].as_str().unwrap().to_string(), row[1].as_int().unwrap())
        })
        .collect();
    assert_eq!(
        values,
        vec![("spam".to_string(), 1), ("bacon".to_string(), 3)]
    );
}

#[test]
fn join_tables() {
    let cursor = Cursor::new(Vec::new());
    let mut package = Package::create(PackageType::Installer, cursor).unwrap();

    // Create a table called "Foobar":
    let columns = vec![
        Column::build("Foo").primary_key().int16(),
        Column::build("Bar").int16(),
    ];
    package.create_table("Foobar", columns).unwrap();
    let query = Insert::into("Foobar")
        .row(vec![Value::Int(1), Value::Int(17)])
        .row(vec![Value::Int(2), Value::Int(42)])
        .row(vec![Value::Int(3), Value::Int(17)]);
    package.insert_rows(query).unwrap();

    // Create a table called "Bazfoo":
    let columns = vec![
        Column::build("Baz").primary_key().int16(),
        Column::build("Foo").int16(),
    ];
    package.create_table("Bazfoo", columns).unwrap();
    let query = Insert::into("Bazfoo")
        .row(vec![Value::Int(4), Value::Int(42)])
        .row(vec![Value::Int(5), Value::Int(13)])
        .row(vec![Value::Int(6), Value::Int(17)]);
    package.insert_rows(query).unwrap();

    // Perform an inner join:
    let query = Select::table("Foobar")
        .inner_join(
            Select::table("Bazfoo"),
            Expr::col("Foobar.Bar").eq(Expr::col("Bazfoo.Foo")),
        )
        .columns(&["Foobar.Foo", "Bazfoo.Baz"]);
    let rows = package.select_rows(query).unwrap();
    let values: Vec<(i32, i32)> = rows
        .map(|row| (row[0].as_int().unwrap(), row[1].as_int().unwrap()))
        .collect();
    assert_eq!(values, vec![(1, 6), (2, 4), (3, 6)]);

    // Perform a left join:
    let query = Select::table("Bazfoo")
        .left_join(
            Select::table("Foobar"),
            Expr::col("Foobar.Bar").eq(Expr::col("Bazfoo.Foo")),
        )
        .columns(&["Bazfoo.Baz", "Foobar.Foo"]);
    let rows = package.select_rows(query).unwrap();
    let values: Vec<(i32, Option<i32>)> =
        rows.map(|row| (row[0].as_int().unwrap(), row[1].as_int())).collect();
    assert_eq!(
        values,
        vec![(4, Some(2)), (5, None), (6, Some(1)), (6, Some(3))]
    );
}

// Regression test for https://github.com/mdsteele/rust-msi/issues/10
#[test]
fn nested_inner_join() {
    let cursor = Cursor::new(Vec::new());
    let mut package = Package::create(PackageType::Installer, cursor).unwrap();

    // Create a table called "Directory":
    let columns = vec![Column::build("Directory").primary_key().int16()];
    package.create_table("Directory", columns).unwrap();
    let query = Insert::into("Directory")
        .row(vec![Value::Int(1)])
        .row(vec![Value::Int(2)]);
    package.insert_rows(query).unwrap();

    // Create a table called "Component":
    let columns = vec![
        Column::build("Component").primary_key().int16(),
        Column::build("Directory_").foreign_key("Directory", 1).int16(),
    ];
    package.create_table("Component", columns).unwrap();
    let query = Insert::into("Component")
        .row(vec![Value::Int(3), Value::Int(2)])
        .row(vec![Value::Int(4), Value::Int(1)])
        .row(vec![Value::Int(5), Value::Int(2)]);
    package.insert_rows(query).unwrap();

    // Create a table called "File":
    let columns = vec![
        Column::build("File").primary_key().int16(),
        Column::build("Component_").foreign_key("Component", 1).int16(),
    ];
    package.create_table("File", columns).unwrap();
    let query = Insert::into("File")
        .row(vec![Value::Int(6), Value::Int(3)])
        .row(vec![Value::Int(7), Value::Int(4)]);
    package.insert_rows(query).unwrap();

    // Perform a nested inner join:
    let query = Select::table("File")
        .inner_join(
            Select::table("Component"),
            Expr::col("Component.Component").eq(Expr::col("File.Component_")),
        )
        .inner_join(
            Select::table("Directory"),
            Expr::col("Directory.Directory")
                .eq(Expr::col("Component.Directory_")),
        );
    let rows = package.select_rows(query).unwrap();
    let values: Vec<(i32, i32, i32)> = rows
        .map(|row| {
            (
                row["File.File"].as_int().unwrap(),
                row["Component.Component"].as_int().unwrap(),
                row["Directory.Directory"].as_int().unwrap(),
            )
        })
        .collect();
    assert_eq!(values, vec![(6, 3, 2), (7, 4, 1)]);
}

//===========================================================================//
