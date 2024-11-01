use crate::internal::streamname::{
    self, DIGITAL_SIGNATURE_STREAM_NAME, DOCUMENT_SUMMARY_INFO_STREAM_NAME,
    MSI_DIGITAL_SIGNATURE_EX_STREAM_NAME, SUMMARY_INFO_STREAM_NAME,
};
use cfb;
use std::io::{self, Read, Seek, SeekFrom, Write};

// ========================================================================= //

/// An IO reader for an embedded binary stream in a package.
pub struct StreamReader<F> {
    stream: cfb::Stream<F>,
}

impl<F> StreamReader<F> {
    pub(crate) fn new(stream: cfb::Stream<F>) -> StreamReader<F> {
        StreamReader { stream }
    }
}

impl<F: Read + Seek> Read for StreamReader<F> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.stream.read(buf)
    }
}

impl<F: Read + Seek> Seek for StreamReader<F> {
    fn seek(&mut self, from: SeekFrom) -> io::Result<u64> {
        self.stream.seek(from)
    }
}

// ========================================================================= //

/// An IO writer for an embedded binary stream in a package.
pub struct StreamWriter<F> {
    stream: cfb::Stream<F>,
}

impl<F> StreamWriter<F> {
    pub(crate) fn new(stream: cfb::Stream<F>) -> StreamWriter<F> {
        StreamWriter { stream }
    }
}

impl<F: Read + Seek + Write> Write for StreamWriter<F> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.stream.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.stream.flush()
    }
}

impl<F: Read + Seek + Write> Seek for StreamWriter<F> {
    fn seek(&mut self, from: SeekFrom) -> io::Result<u64> {
        self.stream.seek(from)
    }
}

// ========================================================================= //

/// An iterator over the names of the binary streams in a package.
///
/// No guarantees are made about the order in which items are returned.
pub struct Streams<'a, F: 'a> {
    entries: cfb::Entries<'a, F>,
}

impl<'a, F: 'a> Streams<'a, F> {
    pub(crate) fn new(entries: cfb::Entries<'a, F>) -> Streams<'a, F> {
        Streams { entries }
    }
}

impl<'a, F: 'a> Iterator for Streams<'a, F> {
    type Item = String;

    fn next(&mut self) -> Option<String> {
        loop {
            let entry = match self.entries.next() {
                Some(entry) => entry,
                None => return None,
            };
            if !entry.is_stream()
                || entry.name() == DIGITAL_SIGNATURE_STREAM_NAME
                || entry.name() == MSI_DIGITAL_SIGNATURE_EX_STREAM_NAME
                || entry.name() == SUMMARY_INFO_STREAM_NAME
                || entry.name() == DOCUMENT_SUMMARY_INFO_STREAM_NAME
            {
                continue;
            }
            let (name, is_table) = streamname::decode(entry.name());
            if !is_table {
                return Some(name);
            }
        }
    }
}

// ========================================================================= //
