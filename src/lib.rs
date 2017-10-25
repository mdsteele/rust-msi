//! A library for reading/writing [Windows
//! Installer](https://en.wikipedia.org/wiki/Windows_Installer) (MSI) files.
//!
//! A Windows Installer file, or *MSI file*, represents a Windows software
//! package and a declarative description of how it should be installed.
//! An MSI file consists of a relational database stored within a [Compound
//! File Binary](https://en.wikipedia.org/wiki/Compound_File_Binary_Format)
//! file.

#![warn(missing_docs)]

extern crate byteorder;
extern crate cfb;
extern crate encoding;
extern crate ordermap;
extern crate uuid;

mod internal;

pub use internal::codepage::CodePage;
pub use internal::column::{Column, ColumnBuilder, ColumnCategory, ColumnType};
pub use internal::expr::Expr;
pub use internal::package::{Package, PackageType, Tables};
pub use internal::query::{Delete, Insert, Select, Update};
pub use internal::stream::{StreamReader, StreamWriter, Streams};
pub use internal::summary::SummaryInfo;
pub use internal::table::{Row, Rows, Table};
pub use internal::value::Value;
use std::fs;
use std::io;
use std::path::Path;

// ========================================================================= //

/// Opens an existing MSI file at the given path in read-only mode.
pub fn open<P: AsRef<Path>>(path: P) -> io::Result<Package<fs::File>> {
    Package::open(fs::File::open(path)?)
}

/// Opens an existing MSI file at the given path in read-write mode.
pub fn open_rw<P: AsRef<Path>>(path: P) -> io::Result<Package<fs::File>> {
    Package::open(fs::OpenOptions::new().read(true).write(true).open(path)?)
}

// ========================================================================= //