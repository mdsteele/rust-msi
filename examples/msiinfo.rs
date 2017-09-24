extern crate msi;

use std::env;

fn main() {
    if env::args().count() != 2 {
        println!("Usage: msiinfo <path>");
        return;
    }
    let path = env::args().nth(1).unwrap();
    let package = msi::open(path).unwrap();
    package.print_entries().unwrap();
}
