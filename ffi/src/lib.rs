//! A basic foreign function interface (FFI) for "only" reading information
//! of [Windows Installer](https://en.wikipedia.org/wiki/Windows_Installer)
//! (MSI) files, built on top of the [msi](https://crates.io/crates/msi) crate.

#![warn(missing_docs)]

use chrono::prelude::{DateTime, Utc};
use msi::{Package, Select};
use safer_ffi::prelude::*;
use std::io;
use std::path::Path;

// ========================================================================= //

/// Information about an MSI file.
#[derive_ReprC]
#[repr(C)]
pub struct MsiInformation {
    /// Architecture of the MSI installer.
    arch: repr_c::String,
    /// Author of the MSI file.
    author: repr_c::String,
    /// Comments about the MSI file.
    comments: repr_c::String,
    /// The application that created the MSI file.
    creating_application: repr_c::String,
    /// The creation time in RFC2822 format.
    creation_time: repr_c::String,
    /// The BCP-47 codes of the languages in the MSI file.
    languages: repr_c::Vec<repr_c::String>,
    /// The subject of the MSI file.
    subject: repr_c::String,
    /// The title of the MSI file.
    title: repr_c::String,
    /// The UUID of the MSI file.
    uuid: repr_c::String,
    /// Word count
    word_count: i32,

    /// Indicates whether the MSI file has a digital signature.
    has_digital_signature: bool,

    /// The names of the tables in the MSI database.
    table_names: repr_c::Vec<repr_c::String>,
}

// ========================================================================= //

/// Gets the information of an MSI file at the given path.
#[ffi_export]
fn get_information(path: char_p::Ref<'_>) -> MsiInformation {
    let file_handle = std::fs::File::open(Path::new(path.to_str())).unwrap();
    let package = Package::open(file_handle).unwrap();

    MsiInformation {
        arch: package
            .summary_info()
            .arch()
            .unwrap_or_default()
            .to_string()
            .into(),
        author: package
            .summary_info()
            .author()
            .unwrap_or_default()
            .to_string()
            .into(),
        comments: package
            .summary_info()
            .comments()
            .unwrap_or_default()
            .to_string()
            .into(),
        creating_application: package
            .summary_info()
            .creating_application()
            .unwrap_or_default()
            .to_string()
            .into(),
        creation_time: {
            if let Some(time) = package.summary_info().creation_time() {
                let datetime: DateTime<Utc> = time.into();
                datetime.to_rfc2822().into()
            } else {
                "".into()
            }
        },
        languages: {
            let mut langs: Vec<repr_c::String> = Vec::new();
            for language in package.summary_info().languages() {
                langs.push(language.code().to_string().into());
            }
            langs.into()
        },
        subject: package
            .summary_info()
            .subject()
            .unwrap_or_default()
            .to_string()
            .into(),
        title: package
            .summary_info()
            .title()
            .unwrap_or_default()
            .to_string()
            .into(),
        uuid: package
            .summary_info()
            .uuid()
            .unwrap_or_default()
            .to_string()
            .into(),
        word_count: package.summary_info().word_count().unwrap_or_default(),

        has_digital_signature: package.has_digital_signature(),

        table_names: {
            let mut names: Vec<repr_c::String> = Vec::new();
            for table in package.tables() {
                names.push(table.name().to_string().into());
            }
            names.into()
        },
    }
}

/// Frees the memory of the given MsiInformation.
#[ffi_export]
fn free_information(info: MsiInformation) {
    drop(info);
}

/// Get the specified table from the MSI file.
#[ffi_export]
fn get_table(
    path: char_p::Ref<'_>,
    table_name: char_p::Ref<'_>,
) -> repr_c::Vec<repr_c::Vec<repr_c::String>> {
    let file_handle = std::fs::File::open(Path::new(path.to_str())).unwrap();
    let mut package = Package::open(file_handle).unwrap();

    let mut result: Vec<repr_c::Vec<repr_c::String>> = Vec::new();

    if !package.has_table(table_name.to_str()) {
        return Vec::new().into();
    }

    let table = package.get_table(table_name.to_str()).unwrap();

    // first, we add column names
    let mut columns: Vec<repr_c::String> = Vec::new();
    for column in table.columns() {
        columns.push(column.name().to_string().into());
    }
    result.push(columns.into());

    // then, we add the rows
    package
        .select_rows(Select::table(table_name.to_str()))
        .expect("select")
        .for_each(|row| {
            let mut row_data: Vec<repr_c::String> =
                Vec::with_capacity(row.len());
            for index in 0..row.len() {
                row_data.push(row[index].to_string().into());
            }
            result.push(row_data.into());
        });

    result.into()
}

/// Frees the memory of the get_table result.
#[ffi_export]
fn free_table(table: repr_c::Vec<repr_c::Vec<repr_c::String>>) {
    drop(table);
}

/// Generate headers/bindings for get_information() function.
pub fn generate_headers() -> io::Result<()> {
    ::safer_ffi::headers::builder()
        .with_language(::safer_ffi::headers::Language::CSharp)
        .to_file("generated.cs")?
        .generate()
}

// ========================================================================= //