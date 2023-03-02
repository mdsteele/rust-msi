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
extern crate encoding_rs;
extern crate uuid;

mod internal;

pub use crate::internal::category::Category;
pub use crate::internal::codepage::CodePage;
pub use crate::internal::column::{Column, ColumnBuilder, ColumnType};
pub use crate::internal::expr::Expr;
pub use crate::internal::language::Language;
pub use crate::internal::package::{Package, PackageType, Tables};
pub use crate::internal::query::{Delete, Insert, Select, Update};
pub use crate::internal::stream::{StreamReader, StreamWriter, Streams};
pub use crate::internal::summary::SummaryInfo;
pub use crate::internal::table::{Row, Rows, Table};
pub use crate::internal::value::Value;
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
