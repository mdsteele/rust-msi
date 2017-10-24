extern crate msi;

use msi::{Column, ColumnCategory, Insert, Package, PackageType, Value};
use std::error::Error;
use std::io::{Cursor, ErrorKind};

// ========================================================================= //

macro_rules! assert_error {
    ($e:expr, $k:expr, $d:expr) => {
        let kind = $k;
        let description = $d;
        match $e {
            Ok(_) => panic!("Expected {:?} error, but result was Ok", kind),
            Err(error) => {
                if error.kind() != kind {
                    panic!("Expected {:?} error, but result was {:?} error \
                            with description {:?}",
                           kind, error.kind(), error.description());
                }
                if error.description() != description {
                    panic!("Expected {:?} error with description {:?}, but \
                            result had description {:?}",
                           kind, description, error.description());
                }
            }
        }
    };
}

// ========================================================================= //

#[test]
fn int_column_value_range() {
    let cursor = Cursor::new(Vec::new());
    let mut package = Package::create(PackageType::Installer, cursor).unwrap();
    let columns =
        vec![Column::build("Number").primary_key().range(0, 100).int16()];
    package.create_table("Numbers".to_string(), columns).unwrap();

    let cursor = package.into_inner().unwrap();
    let mut package = Package::open(cursor).unwrap();
    {
        let table = package.get_table("Numbers").unwrap();
        let column = table.get_column("Number").unwrap();
        assert_eq!(column.value_range(), Some((0, 100)));
    }
    let query = Insert::into("Numbers").row(vec![Value::Int(-7)]);
    assert_error!(package.insert_rows(query),
                  ErrorKind::InvalidInput,
                  "-7 is not a valid value for column \"Number\"");
    let query = Insert::into("Numbers").row(vec![Value::Int(101)]);
    assert_error!(package.insert_rows(query),
                  ErrorKind::InvalidInput,
                  "101 is not a valid value for column \"Number\"");
    let query = Insert::into("Numbers").row(vec![Value::Int(100)]);
    package.insert_rows(query).unwrap();
}

#[test]
fn string_column_category() {
    let cursor = Cursor::new(Vec::new());
    let mut package = Package::create(PackageType::Installer, cursor).unwrap();
    let columns = vec![
        Column::build("Property")
            .primary_key()
            .category(ColumnCategory::Property)
            .string(32),
    ];
    package.create_table("Properties".to_string(), columns).unwrap();

    let cursor = package.into_inner().unwrap();
    let mut package = Package::open(cursor).unwrap();
    {
        let table = package.get_table("Properties").unwrap();
        let column = table.get_column("Property").unwrap();
        assert_eq!(column.category(), Some(ColumnCategory::Property));
    }
    let query = Insert::into("Properties")
        .row(vec![Value::Str("$99".to_string())]);
    assert_error!(package.insert_rows(query),
                  ErrorKind::InvalidInput,
                  "\"$99\" is not a valid value for column \"Property\"");
    let query = Insert::into("Properties")
        .row(vec![Value::Str("%Foo".to_string())]);
    package.insert_rows(query).unwrap();
}

#[test]
fn string_column_enum_values() {
    let cursor = Cursor::new(Vec::new());
    let mut package = Package::create(PackageType::Installer, cursor).unwrap();
    let columns = vec![
        Column::build("Day")
            .primary_key()
            .enum_values(&["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"])
            .string(3),
    ];
    package.create_table("Days".to_string(), columns).unwrap();

    let cursor = package.into_inner().unwrap();
    let mut package = Package::open(cursor).unwrap();
    {
        let table = package.get_table("Days").unwrap();
        let column = table.get_column("Day").unwrap();
        assert_eq!(
            column.enum_values(),
            Some(
                [
                    "Sun".to_string(),
                    "Mon".to_string(),
                    "Tue".to_string(),
                    "Wed".to_string(),
                    "Thu".to_string(),
                    "Fri".to_string(),
                    "Sat".to_string(),
                ].as_ref(),
            )
        );
    }
    let query = Insert::into("Days").row(vec![Value::Str("Sit".to_string())]);
    assert_error!(package.insert_rows(query),
                  ErrorKind::InvalidInput,
                  "\"Sit\" is not a valid value for column \"Day\"");
    let query = Insert::into("Days").row(vec![Value::Str("Sat".to_string())]);
    package.insert_rows(query).unwrap();
}

// ========================================================================= //
