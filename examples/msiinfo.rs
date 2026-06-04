use clap::{Parser, Subcommand};
use std::cmp;
use std::io::{self, Read, Seek};
use std::path::PathBuf;
use time::OffsetDateTime;

fn pad(mut string: String, fill: char, width: usize) -> String {
    while string.len() < width {
        string.push(fill);
    }
    string
}

fn print_summary_info<F>(package: &msi::Package<F>) {
    println!("Package type: {:?}", package.package_type());
    let is_signed = package.has_digital_signature();
    let summary_info = package.summary_info();
    let codepage = summary_info.codepage();
    println!("   Code page: {} ({})", codepage.id(), codepage.name());
    if let Some(title) = summary_info.title() {
        println!("       Title: {title}");
    }
    if let Some(subject) = summary_info.subject() {
        println!("     Subject: {subject}");
    }
    if let Some(author) = summary_info.author() {
        println!("      Author: {author}");
    }
    if let Some(uuid) = summary_info.uuid() {
        println!("        UUID: {}", uuid.hyphenated());
    }
    if let Some(arch) = summary_info.arch() {
        println!("        Arch: {arch}");
    }
    let languages = summary_info.languages();
    if !languages.is_empty() {
        let tags: Vec<&str> =
            languages.iter().map(msi::LanguageId::tag).collect();
        println!("    Language: {}", tags.join(", "));
    }
    if let Some(timestamp) = summary_info.creation_time() {
        println!("  Created at: {}", OffsetDateTime::from(timestamp));
    }
    if let Some(app_name) = summary_info.creating_application() {
        println!("Created with: {app_name}");
    }
    println!("      Signed: {}", if is_signed { "yes" } else { "no" });
    if let Some(comments) = summary_info.comments() {
        println!("Comments:");
        for line in comments.lines() {
            println!("  {line}");
        }
    }
}

fn print_table_description(table: &msi::Table) {
    println!("{}", table.name());
    for column in table.columns() {
        println!(
            "  {:<16} {}{}{}",
            column.name(),
            if column.is_primary_key() { '*' } else { ' ' },
            column.coltype(),
            if column.is_nullable() { "?" } else { "" }
        );
    }
}

fn print_table_contents<F: Read + Seek>(
    package: &mut msi::Package<F>,
    table_name: &str,
) {
    let mut col_widths: Vec<usize> = package
        .get_table(table_name)
        .unwrap()
        .columns()
        .iter()
        .map(|column| column.name().len())
        .collect();
    let rows: Vec<Vec<String>> = package
        .select_rows(msi::Select::table(table_name))
        .expect("select")
        .map(|row| {
            let mut strings = Vec::with_capacity(row.len());
            for index in 0..row.len() {
                let string = row[index].to_string();
                col_widths[index] = cmp::max(col_widths[index], string.len());
                strings.push(string);
            }
            strings
        })
        .collect();
    {
        let mut line = String::new();
        for (index, column) in
            package.get_table(table_name).unwrap().columns().iter().enumerate()
        {
            let string =
                pad(column.name().to_string(), ' ', col_widths[index]);
            line.push_str(&string);
            line.push_str("  ");
        }
        println!("{line}");
    }
    {
        let mut line = String::new();
        for &width in &col_widths {
            let string = pad(String::new(), '-', width);
            line.push_str(&string);
            line.push_str("  ");
        }
        println!("{line}");
    }
    for row in rows {
        let mut line = String::new();
        for (index, value) in row.into_iter().enumerate() {
            let string = pad(value, ' ', col_widths[index]);
            line.push_str(&string);
            line.push_str("  ");
        }
        println!("{line}");
    }
}

#[derive(Parser)]
#[command(
    name = "msiinfo",
    version = "0.1",
    author = "Matthew D. Steele <mdsteele@alum.mit.edu>",
    about = "Inspects MSI files"
)]
struct MsiInfo {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Prints schema for a table in an MSI file
    Describe { path: PathBuf, table: String },

    /// Prints all rows for a table in an MSI file
    Export { path: PathBuf, table: String },

    /// Extract a binary stream from an MSI file
    Extract { path: PathBuf, stream: String },

    /// Lists binary streams in an MSI file
    Streams { path: PathBuf },

    /// Prints summary information for an MSI file
    Summary { path: PathBuf },

    /// Lists database tables in an MSI file
    Tables { path: PathBuf },
}

fn main() -> io::Result<()> {
    let cli = MsiInfo::parse();

    match cli.command {
        Commands::Describe { path, table } => {
            let package = msi::open(&path)?;

            if let Some(table_def) = package.get_table(&table) {
                print_table_description(table_def);
            } else {
                println!("No table {table:?} exists in the database.");
            }
        }
        Commands::Export { path, table } => {
            let mut package = msi::open(&path)?;
            print_table_contents(&mut package, &table);
        }
        Commands::Extract { path, stream } => {
            let mut package = msi::open(&path)?;
            let mut input = package.read_stream(&stream)?;
            io::copy(&mut input, &mut io::stdout())?;
        }
        Commands::Streams { path } => {
            let package = msi::open(&path)?;

            for stream_name in package.streams() {
                println!("{stream_name}");
            }
        }
        Commands::Summary { path } => {
            let package = msi::open(&path)?;
            print_summary_info(&package);
        }
        Commands::Tables { path } => {
            let package = msi::open(&path)?;

            for table in package.tables() {
                println!("{}", table.name());
            }
        }
    }

    Ok(())
}
