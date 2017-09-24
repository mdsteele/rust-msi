use cfb;
use internal::streamname;
use std::io::{self, Read, Seek};

// ========================================================================= //

/// An MSI package file, backed by an underlying reader/writer (such as a
/// [`File`](https://doc.rust-lang.org/std/fs/struct.File.html) or
/// [`Cursor`](https://doc.rust-lang.org/std/io/struct.Cursor.html)).
pub struct Package<F> {
    comp: cfb::CompoundFile<F>,
}

impl<F> Package<F> {
    /// Consumes the `Package` object, returning the underlying reader/writer.
    pub fn into_inner(self) -> F { self.comp.into_inner() }
}

impl<F: Read + Seek> Package<F> {
    /// Opens an existing MSI file, using the underlying reader.  If the
    /// underlying reader also supports the `Write` trait, then the `Package`
    /// object will be writable as well.
    pub fn open(inner: F) -> io::Result<Package<F>> {
        let comp = cfb::CompoundFile::open(inner)?;
        Ok(Package { comp: comp })
    }

    /// Temporary helper function for testing.
    pub fn print_entries(&self) -> io::Result<()> {
        for entry in self.comp.read_storage("/")? {
            println!("{:?}", streamname::decode(entry.name()));
        }
        Ok(())
    }
}

// ========================================================================= //
