use crate::internal::streamname::{self, SUMMARY_INFO_STREAM_NAME};
use cfb;
use std::io::{self, Read, Seek, SeekFrom, Write};

// ========================================================================= //

/// An IO reader for an embedded binary stream in a package.
pub struct StreamReader<'a, F: 'a> {
    stream: cfb::Stream<'a, F>,
}

impl<'a, F> StreamReader<'a, F> {
    pub(crate) fn new(stream: cfb::Stream<'a, F>) -> StreamReader<'a, F> {
        StreamReader { stream }
    }
}

impl<'a, F: Read + Seek> Read for StreamReader<'a, F> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.stream.read(buf)
    }
}

impl<'a, F: Read + Seek> Seek for StreamReader<'a, F> {
    fn seek(&mut self, from: SeekFrom) -> io::Result<u64> {
        self.stream.seek(from)
    }
}

// ========================================================================= //

/// An IO writer for an embedded binary stream in a package.
pub struct StreamWriter<'a, F: 'a> {
    stream: cfb::Stream<'a, F>,
}

impl<'a, F> StreamWriter<'a, F> {
    pub(crate) fn new(stream: cfb::Stream<'a, F>) -> StreamWriter<'a, F> {
        StreamWriter { stream }
    }
}

impl<'a, F: Read + Seek + Write> Write for StreamWriter<'a, F> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.stream.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.stream.flush()
    }
}

impl<'a, F: Read + Seek + Write> Seek for StreamWriter<'a, F> {
    fn seek(&mut self, from: SeekFrom) -> io::Result<u64> {
        self.stream.seek(from)
    }
}

// ========================================================================= //

/// An iterator over the names of the binary streams in a package.
///
/// No guarantees are made about the order in which items are returned.
pub struct Streams<'a> {
    entries: cfb::Entries<'a>,
}

impl<'a> Streams<'a> {
    pub(crate) fn new(entries: cfb::Entries<'a>) -> Streams<'a> {
        Streams { entries }
    }
}

impl<'a> Iterator for Streams<'a> {
    type Item = String;

    fn next(&mut self) -> Option<String> {
        loop {
            let entry = match self.entries.next() {
                Some(entry) => entry,
                None => return None,
            };
            if !entry.is_stream() || entry.name() == SUMMARY_INFO_STREAM_NAME {
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
