use clap::{App, Arg, SubCommand};
use std::cmp;
use std::io::{self, Read, Seek};
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
            languages.iter().map(msi::Language::tag).collect();
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
        for &width in col_widths.iter() {
            let string = pad(String::new(), '-', width);
            line.push_str(&string);
            line.push_str("  ");
        }
        println!("{line}");
    }
    for row in rows.into_iter() {
        let mut line = String::new();
        for (index, value) in row.into_iter().enumerate() {
            let string = pad(value, ' ', col_widths[index]);
            line.push_str(&string);
            line.push_str("  ");
        }
        println!("{line}");
    }
}

fn main() {
    let matches = App::new("msiinfo")
        .version("0.1")
        .author("Matthew D. Steele <mdsteele@alum.mit.edu>")
        .about("Inspects MSI files")
        .subcommand(
            SubCommand::with_name("describe")
                .about("Prints schema for a table in an MSI file")
                .arg(Arg::with_name("path").required(true))
                .arg(Arg::with_name("table").required(true)),
        )
        .subcommand(
            SubCommand::with_name("export")
                .about("Prints all rows for a table in an MSI file")
                .arg(Arg::with_name("path").required(true))
                .arg(Arg::with_name("table").required(true)),
        )
        .subcommand(
            SubCommand::with_name("extract")
                .about("Extract a binary stream from an MSI file")
                .arg(Arg::with_name("path").required(true))
                .arg(Arg::with_name("stream").required(true)),
        )
        .subcommand(
            SubCommand::with_name("streams")
                .about("Lists binary streams in an MSI file")
                .arg(Arg::with_name("path").required(true)),
        )
        .subcommand(
            SubCommand::with_name("summary")
                .about("Prints summary information for an MSI file")
                .arg(Arg::with_name("path").required(true)),
        )
        .subcommand(
            SubCommand::with_name("tables")
                .about("Lists database tables in an MSI file")
                .arg(Arg::with_name("path").required(true)),
        )
        .get_matches();
    if let Some(submatches) = matches.subcommand_matches("describe") {
        let path = submatches.value_of("path").unwrap();
        let table_name = submatches.value_of("table").unwrap();
        let package = msi::open(path).expect("open package");
        if let Some(table) = package.get_table(table_name) {
            print_table_description(table);
        } else {
            println!("No table {table_name:?} exists in the database.");
        }
    } else if let Some(submatches) = matches.subcommand_matches("export") {
        let path = submatches.value_of("path").unwrap();
        let table_name = submatches.value_of("table").unwrap();
        let mut package = msi::open(path).expect("open package");
        print_table_contents(&mut package, table_name);
    } else if let Some(submatches) = matches.subcommand_matches("extract") {
        let path = submatches.value_of("path").unwrap();
        let stream_name = submatches.value_of("stream").unwrap();
        let mut package = msi::open(path).expect("open package");
        let mut stream = package.read_stream(stream_name).expect("read");
        io::copy(&mut stream, &mut io::stdout()).expect("extract");
    } else if let Some(submatches) = matches.subcommand_matches("streams") {
        let path = submatches.value_of("path").unwrap();
        let package = msi::open(path).expect("open package");
        for stream_name in package.streams() {
            println!("{stream_name}");
        }
    } else if let Some(submatches) = matches.subcommand_matches("summary") {
        let path = submatches.value_of("path").unwrap();
        let package = msi::open(path).expect("open package");
        print_summary_info(&package);
    } else if let Some(submatches) = matches.subcommand_matches("tables") {
        let path = submatches.value_of("path").unwrap();
        let package = msi::open(path).expect("open package");
        for table in package.tables() {
            println!("{}", table.name());
        }
    }
}
