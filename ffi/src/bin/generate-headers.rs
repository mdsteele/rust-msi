fn main() -> ::std::io::Result<()> {
    println!("Syntax: generate-headers <language> <filename>");
    println!("Supported languages: c, csharp (cs) (c#), python (py)");
    if let Some(lang) = ::std::env::args_os().nth(1) {
        msi_ffi::generate_headers(
            lang.to_str().unwrap(),
            ::std::env::args_os().nth(2).unwrap_or_default().to_str().unwrap().to_string(),
        )
    } else {
        println!("No language specified.");
        Ok(())
    }
}
