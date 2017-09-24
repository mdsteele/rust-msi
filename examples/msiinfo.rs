extern crate msi;

use std::env;

fn main() {
    if env::args().count() != 2 {
        println!("Usage: msiinfo <path>");
        return;
    }
    let path = env::args().nth(1).unwrap();
    let mut package = msi::open(path).unwrap();
    package.print_entries().unwrap();

    let summary_info = package.get_summary_info().unwrap();
    if let Some(title) = summary_info.title() {
        println!("Title: {}", title);
    }
    if let Some(author) = summary_info.author() {
        println!("Author: {}", author);
    }
}
