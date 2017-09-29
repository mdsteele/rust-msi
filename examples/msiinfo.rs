extern crate chrono;
extern crate msi;

use chrono::{DateTime, NaiveDateTime, Utc};
use std::env;
use std::time::{SystemTime, UNIX_EPOCH};

fn to_datetime(timestamp: SystemTime) -> DateTime<Utc> {
    let delta = timestamp.duration_since(UNIX_EPOCH).expect("duration_since");
    let naive = NaiveDateTime::from_timestamp(delta.as_secs() as i64,
                                              delta.subsec_nanos());
    DateTime::<Utc>::from_utc(naive, Utc)
}

fn print_summary_info(summary_info: &msi::SummaryInfo) {
    let codepage = summary_info.codepage();
    println!("   Code page: {} ({})", codepage.id(), codepage.name());
    if let Some(title) = summary_info.title() {
        println!("       Title: {}", title);
    }
    if let Some(subject) = summary_info.subject() {
        println!("     Subject: {}", subject);
    }
    if let Some(author) = summary_info.author() {
        println!("      Author: {}", author);
    }
    if let Some(uuid) = summary_info.uuid() {
        println!("        UUID: {}", uuid.hyphenated());
    }
    if let Some(timestamp) = summary_info.creation_time() {
        println!("  Created at: {}", to_datetime(timestamp));
    }
    if let Some(app_name) = summary_info.creating_application() {
        println!("Created with: {}", app_name);
    }
    if let Some(comments) = summary_info.comments() {
        println!("Comments:");
        for line in comments.lines() {
            println!("  {}", line);
        }
    }
}

fn print_table_description(table: &msi::Table) {
    println!("{}", table.name());
    for column in table.columns() {
        println!("  {:<16} {}{:?}",
                 column.name(),
                 if column.is_key() { '*' } else { ' ' },
                 column.coltype());
    }
}

fn main() {
    if env::args().count() != 2 {
        println!("Usage: msiinfo <path>");
        return;
    }
    let path = env::args().nth(1).expect("path");
    let package = msi::open(path).expect("package");

    package.print_entries().expect("print_entries");

    print_summary_info(package.summary_info());

    for table in package.tables().values() {
        println!();
        print_table_description(table);
    }
}
