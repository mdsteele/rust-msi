#[macro_use]
mod testutil;

use msi::{Package, PackageType};
use std::io::{Cursor, ErrorKind, Read, Write};

// ========================================================================= //

#[test]
fn invalid_stream_name() {
    let cursor = Cursor::new(Vec::new());
    let mut package = Package::create(PackageType::Installer, cursor).unwrap();
    assert_error!(
        package.read_stream("\u{4840}Foo"),
        ErrorKind::InvalidInput,
        "\"\u{4840}Foo\" is not a valid stream name"
    );
    assert_error!(
        package.write_stream("\u{4840}Bar"),
        ErrorKind::InvalidInput,
        "\"\u{4840}Bar\" is not a valid stream name"
    );
    assert_error!(
        package.remove_stream("\u{4840}Baz"),
        ErrorKind::InvalidInput,
        "\"\u{4840}Baz\" is not a valid stream name"
    );
}

#[test]
fn nonexistent_stream_name() {
    let cursor = Cursor::new(Vec::new());
    let mut package = Package::create(PackageType::Installer, cursor).unwrap();
    assert_error!(
        package.read_stream("Foo"),
        ErrorKind::NotFound,
        "Stream \"Foo\" does not exist"
    );
    assert_error!(
        package.remove_stream("Baz"),
        ErrorKind::NotFound,
        "Stream \"Baz\" does not exist"
    );
}

#[test]
fn create_and_remove_stream() {
    let cursor = Cursor::new(Vec::new());
    let mut package = Package::create(PackageType::Installer, cursor).unwrap();
    assert!(!package.has_stream("Hello"));
    assert_eq!(
        package.streams().collect::<Vec<String>>(),
        Vec::<String>::new()
    );

    package.write_stream("Hello").unwrap().write_all(b"Hi there!").unwrap();
    assert!(package.has_stream("Hello"));
    assert_eq!(
        package.streams().collect::<Vec<String>>(),
        vec!["Hello".to_string()]
    );

    let cursor = package.into_inner().unwrap();
    let mut package = Package::open(cursor).unwrap();
    assert!(package.has_stream("Hello"));
    assert_eq!(
        package.streams().collect::<Vec<String>>(),
        vec!["Hello".to_string()]
    );

    let mut data = Vec::<u8>::new();
    package.read_stream("Hello").unwrap().read_to_end(&mut data).unwrap();
    assert_eq!(data.as_slice(), b"Hi there!");

    package.remove_stream("Hello").unwrap();
    assert!(!package.has_stream("Hello"));
    assert_eq!(
        package.streams().collect::<Vec<String>>(),
        Vec::<String>::new()
    );

    let cursor = package.into_inner().unwrap();
    let package = Package::open(cursor).unwrap();
    assert!(!package.has_stream("Hello"));
    assert_eq!(
        package.streams().collect::<Vec<String>>(),
        Vec::<String>::new()
    );
}

// ========================================================================= //
