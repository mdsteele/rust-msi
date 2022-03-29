#[macro_use]
mod testutil;

use msi::{Category, Column, Insert, Package, PackageType, Value};
use std::io::{Cursor, ErrorKind};

// ========================================================================= //

#[test]
fn int_column_value_range() {
    let cursor = Cursor::new(Vec::new());
    let mut package = Package::create(PackageType::Installer, cursor).unwrap();
    let columns =
        vec![Column::build("Number").primary_key().range(0, 100).int16()];
    package.create_table("Numbers", columns).unwrap();

    let cursor = package.into_inner().unwrap();
    let mut package = Package::open(cursor).unwrap();
    {
        let table = package.get_table("Numbers").unwrap();
        let column = table.get_column("Number").unwrap();
        assert_eq!(column.value_range(), Some((0, 100)));
    }
    let query = Insert::into("Numbers").row(vec![Value::Int(-7)]);
    assert_error!(
        package.insert_rows(query),
        ErrorKind::InvalidInput,
        "-7 is not a valid value for column \"Number\""
    );
    let query = Insert::into("Numbers").row(vec![Value::Int(101)]);
    assert_error!(
        package.insert_rows(query),
        ErrorKind::InvalidInput,
        "101 is not a valid value for column \"Number\""
    );
    let query = Insert::into("Numbers").row(vec![Value::Int(100)]);
    package.insert_rows(query).unwrap();
}

#[test]
fn string_column_category() {
    let cursor = Cursor::new(Vec::new());
    let mut package = Package::create(PackageType::Installer, cursor).unwrap();
    let columns = vec![Column::build("Property")
        .primary_key()
        .category(Category::Property)
        .string(32)];
    package.create_table("Properties", columns).unwrap();

    let cursor = package.into_inner().unwrap();
    let mut package = Package::open(cursor).unwrap();
    {
        let table = package.get_table("Properties").unwrap();
        let column = table.get_column("Property").unwrap();
        assert_eq!(column.category(), Some(Category::Property));
    }
    let query = Insert::into("Properties").row(vec![Value::from("$99")]);
    assert_error!(
        package.insert_rows(query),
        ErrorKind::InvalidInput,
        "\"$99\" is not a valid value for column \"Property\""
    );
    let query = Insert::into("Properties").row(vec![Value::from("%Foo")]);
    package.insert_rows(query).unwrap();
}

#[test]
fn string_column_enum_values() {
    let cursor = Cursor::new(Vec::new());
    let mut package = Package::create(PackageType::Installer, cursor).unwrap();
    let columns = vec![Column::build("Day")
        .primary_key()
        .enum_values(&["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"])
        .string(3)];
    package.create_table("Days", columns).unwrap();

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
                ]
                .as_ref(),
            )
        );
    }
    let query = Insert::into("Days").row(vec![Value::from("Sit")]);
    assert_error!(
        package.insert_rows(query),
        ErrorKind::InvalidInput,
        "\"Sit\" is not a valid value for column \"Day\""
    );
    let query = Insert::into("Days").row(vec![Value::from("Sat")]);
    package.insert_rows(query).unwrap();
}

// ========================================================================= //
