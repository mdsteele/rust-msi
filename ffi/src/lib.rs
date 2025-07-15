//! A basic foreign function interface (FFI) for "only" reading information
//! of [Windows Installer](https://en.wikipedia.org/wiki/Windows_Installer)
//! (MSI) files, built on top of the [msi](https://crates.io/crates/msi) crate.

#![warn(missing_docs)]

use chrono::prelude::{DateTime, Utc};
use msi::{Package, Select};
use safer_ffi::prelude::*;
use std::{io, path::Path};

// ========================================================================= //

trait MsiInformationDefault {
    fn default() -> Self;
}

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

impl MsiInformationDefault for MsiInformation {
    fn default() -> Self {
        Self {
            arch: "".into(),
            author: "".into(),
            comments: "".into(),
            creating_application: "".into(),
            creation_time: "".into(),
            languages: repr_c::Vec::EMPTY,
            subject: "".into(),
            title: "".into(),
            uuid: "".into(),
            word_count: 0,

            has_digital_signature: false,

            table_names: repr_c::Vec::EMPTY,
        }
    }
}

// ========================================================================= //

/// Gets the information of an MSI file at the given path.
#[ffi_export]
fn get_information(path: char_p::Ref<'_>) -> MsiInformation {
    match std::fs::File::open(Path::new(path.to_str())) {
        Ok(file_handle) => match Package::open(file_handle) {
            Ok(package) => MsiInformation {
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
                    if let Some(time) = package.summary_info().creation_time()
                    {
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
                word_count: package
                    .summary_info()
                    .word_count()
                    .unwrap_or_default(),

                has_digital_signature: package.has_digital_signature(),

                table_names: {
                    let mut names: Vec<repr_c::String> = Vec::new();
                    for table in package.tables() {
                        names.push(table.name().to_string().into());
                    }
                    names.into()
                },
            },
            _ => MsiInformation::default(),
        },
        _ => MsiInformation::default(),
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
    match std::fs::File::open(Path::new(path.to_str())) {
        Ok(file_handle) => match Package::open(file_handle) {
            Ok(mut package) => match package.get_table(table_name.to_str()) {
                Some(table) => {
                    let mut result: Vec<repr_c::Vec<repr_c::String>> =
                        Vec::new();

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
                None => repr_c::Vec::EMPTY,
            },
            _ => repr_c::Vec::EMPTY,
        },
        _ => repr_c::Vec::EMPTY,
    }
}

/// Frees the memory of the get_table result.
#[ffi_export]
fn free_table(table: repr_c::Vec<repr_c::Vec<repr_c::String>>) {
    drop(table);
}

/// Generate headers/bindings for get_information() function.
pub fn generate_headers(lang: &str, filename: String) -> io::Result<()> {
    let file_extension;
    let language = match lang.to_lowercase().as_str() {
        "c" => {
            println!("Selected language: C");
            file_extension = "h";
            safer_ffi::headers::Language::C
        }
        "cs" | "c#" | "csharp" => {
            println!("Selected language: CSharp (C#)");
            file_extension = "cs";
            safer_ffi::headers::Language::CSharp
        }
        "py" | "python" => {
            println!("Selected language: Python (py)");
            file_extension = "cffi";
            safer_ffi::headers::Language::Python
        }
        _ => {
            println!("Unsupported language: {lang}");
            println!("Defaulting to C language.");
            file_extension = "h";
            safer_ffi::headers::Language::C
        }
    };
    safer_ffi::headers::builder()
        .with_language(language)
        .to_file({
            if filename.is_empty() {
                println!(
                    "No filename specified. Defaulting to msi_ffi.{file_extension}"
                );
                format!("msi_ffi.{file_extension}")
            } else if filename
                .ends_with(format!(".{file_extension}").as_str())
            {
                filename
            } else {
                format!("{filename}.{file_extension}")
            }
        })?
        .generate()
}

// ========================================================================= //
