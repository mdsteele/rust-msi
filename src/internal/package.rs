use cfb;
use internal::streamname;
use internal::stringpool::{StringPool, StringPoolBuilder};
use internal::summary::SummaryInfo;
use std::io::{self, Read, Seek, Write};

// ========================================================================= //

const STRING_DATA_STREAM_NAME: &str = "\u{4840}_StringData";
const STRING_POOL_STREAM_NAME: &str = "\u{4840}_StringPool";
const SUMMARY_INFO_STREAM_NAME: &str = "\u{5}SummaryInformation";

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

    /// Parses the summary information from the MSI package.
    pub fn get_summary_info(&mut self) -> io::Result<SummaryInfo> {
        SummaryInfo::read(self.comp.open_stream(SUMMARY_INFO_STREAM_NAME)?)
    }

    /// Parses the string pool from the MSI package.
    pub fn get_string_pool(&mut self) -> io::Result<StringPool> {
        let builder = {
            let name = streamname::encode(STRING_POOL_STREAM_NAME);
            let stream = self.comp.open_stream(name)?;
            StringPoolBuilder::read_from_pool(stream)?
        };
        let name = streamname::encode(STRING_DATA_STREAM_NAME);
        let stream = self.comp.open_stream(name)?;
        builder.build_from_data(stream)
    }

    /// Temporary helper function for testing.
    pub fn print_entries(&self) -> io::Result<()> {
        for entry in self.comp.read_storage("/")? {
            println!("{:?}", streamname::decode(entry.name()));
        }
        Ok(())
    }
}

impl<F: Read + Write + Seek> Package<F> {
    /// Overwrites the package's summary information.
    pub fn set_summary_info(&mut self, summary_info: &SummaryInfo)
                            -> io::Result<()> {
        summary_info.write(self.comp.create_stream(SUMMARY_INFO_STREAM_NAME)?)
    }
}

// ========================================================================= //
