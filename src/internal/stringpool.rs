use crate::internal::codepage::CodePage;
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use std::io::{self, Read, Write};

// ========================================================================= //

const LONG_STRING_REFS_BIT: u32 = 0x8000_0000;

const MAX_STRING_REF: i32 = 0xff_ffff;

// ========================================================================= //

/// A reference to a string in the string pool.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StringRef(i32);

impl StringRef {
    /// Reads a serialized StringRef and returns it, or `None` for a null
    /// reference.  The `long_string_refs` argument specifies whether to read
    /// three bytes (if true) or two (if false).
    pub fn read<R: Read>(
        reader: &mut R,
        long_string_refs: bool,
    ) -> io::Result<Option<StringRef>> {
        let mut number = reader.read_u16::<LittleEndian>()? as i32;
        if long_string_refs {
            number |= (reader.read_u8()? as i32) << 16;
        }
        Ok(if number == 0 { None } else { Some(StringRef(number)) })
    }

    /// Serializes a nullable StringRef.  The `long_string_refs` argument
    /// specifies whether to write three bytes (if true) or two (if false).
    pub fn write<W: Write>(
        writer: &mut W,
        string_ref: Option<StringRef>,
        long_string_refs: bool,
    ) -> io::Result<()> {
        let number = if let Some(StringRef(number)) = string_ref {
            debug_assert!(number > 0);
            debug_assert!(number <= MAX_STRING_REF);
            number
        } else {
            0
        };
        if long_string_refs {
            writer.write_u16::<LittleEndian>((number & 0xffff) as u16)?;
            writer.write_u8(((number >> 16) & 0xff) as u8)?;
        } else if number <= (u16::MAX as i32) {
            writer.write_u16::<LittleEndian>(number as u16)?;
        } else {
            invalid_input!(
                "Cannot write {:?} with long_string_refs=false",
                StringRef(number)
            );
        }
        Ok(())
    }

    /// Returns the reference number, that is, the 1-based index into the
    /// string pool for this reference.
    pub fn number(self) -> i32 {
        let StringRef(number) = self;
        debug_assert!(number > 0);
        debug_assert!(number <= MAX_STRING_REF);
        number
    }

    fn index(self) -> usize {
        let number = self.number();
        debug_assert!(number > 0);
        debug_assert!(number <= MAX_STRING_REF);
        (number - 1) as usize
    }
}

// ========================================================================= //

pub struct StringPoolBuilder {
    codepage: CodePage,
    long_string_refs: bool,
    lengths_and_refcounts: Vec<(u32, u16)>,
}

impl StringPoolBuilder {
    pub fn read_from_pool<R: Read>(
        mut reader: R,
    ) -> io::Result<StringPoolBuilder> {
        let codepage_id = reader.read_u32::<LittleEndian>()?;
        let long_string_refs = (codepage_id & LONG_STRING_REFS_BIT) != 0;
        let codepage_id = (codepage_id & !LONG_STRING_REFS_BIT) as i32;
        let codepage = match CodePage::from_id(codepage_id) {
            Some(codepage) => codepage,
            None => invalid_data!(
                "Unknown codepage for string pool ({})",
                codepage_id
            ),
        };
        let mut lengths_and_refcounts = Vec::<(u32, u16)>::new();
        while let Ok(length) = reader.read_u16::<LittleEndian>() {
            let mut length = length as u32;
            let mut refcount = reader.read_u16::<LittleEndian>()?;
            if length == 0 && refcount > 0 {
                length = ((refcount as u32) << 16)
                    | (reader.read_u16::<LittleEndian>()? as u32);
                refcount = reader.read_u16::<LittleEndian>()?;
            }
            lengths_and_refcounts.push((length, refcount));
        }
        Ok(StringPoolBuilder {
            codepage,
            long_string_refs,
            lengths_and_refcounts,
        })
    }

    pub fn build_from_data<R: Read>(
        self,
        mut reader: R,
    ) -> io::Result<StringPool> {
        let mut strings = Vec::<(String, u16)>::new();
        for (length, refcount) in self.lengths_and_refcounts.into_iter() {
            let mut buffer = vec![0u8; length as usize];
            reader.read_exact(&mut buffer)?;
            strings.push((self.codepage.decode(&buffer), refcount));
        }
        Ok(StringPool {
            codepage: self.codepage,
            strings,
            long_string_refs: self.long_string_refs,
            is_modified: false,
        })
    }
}

// ========================================================================= //

/// The string pool for an MSI package.
pub struct StringPool {
    codepage: CodePage,
    strings: Vec<(String, u16)>,
    long_string_refs: bool,
    is_modified: bool,
}

impl StringPool {
    /// Creates a new, empty string pool.
    pub fn new(codepage: CodePage) -> StringPool {
        StringPool {
            codepage,
            strings: Vec::new(),
            long_string_refs: false,
            is_modified: true,
        }
    }

    /// Gets the code page used for serializing the string data.
    pub fn codepage(&self) -> CodePage {
        self.codepage
    }

    /// Sets the code page used for serializing the string data.
    pub fn set_codepage(&mut self, codepage: CodePage) {
        self.is_modified = true;
        self.codepage = codepage;
    }

    /// Returns the number of strings in the string pool (including empty
    /// entries).
    #[allow(dead_code)]
    pub fn num_strings(&self) -> u32 {
        self.strings.len() as u32
    }

    /// Returns true if string references should be serialized with three bytes
    /// instead of two.
    pub fn long_string_refs(&self) -> bool {
        self.long_string_refs
    }

    pub(crate) fn is_modified(&self) -> bool {
        self.is_modified
    }

    pub(crate) fn mark_unmodified(&mut self) {
        self.is_modified = false;
    }

    /// Returns the string in the pool for the given reference.
    pub fn get(&self, string_ref: StringRef) -> &str {
        let index = string_ref.index();
        if index < self.strings.len() {
            self.strings[index].0.as_str()
        } else {
            ""
        }
    }

    /// Returns the pool's refcount for the given string reference.
    #[allow(dead_code)]
    pub fn refcount(&self, string_ref: StringRef) -> u16 {
        let index = string_ref.index();
        if index < self.strings.len() {
            self.strings[index].1
        } else {
            0
        }
    }

    /// Inserts a string into the pool, or increments its refcount if it's
    /// already in the pool, and returns the index of the string in the pool.
    pub fn incref(&mut self, string: String) -> StringRef {
        self.is_modified = true;
        // TODO: change the internal representation of StringPool to make this
        // more efficient.
        for (index, &mut (ref mut st, ref mut refcount)) in
            self.strings.iter_mut().enumerate()
        {
            if *refcount == 0 {
                debug_assert_eq!(st, "");
                *st = string;
                *refcount = 1;
                return StringRef((index + 1) as i32);
            }
            if *st == string && *refcount < u16::MAX {
                *refcount += 1;
                return StringRef((index + 1) as i32);
            }
        }
        if self.strings.len() >= u16::MAX as usize && !self.long_string_refs {
            // TODO: If this happens, we need to rewrite all database tables
            // from short to long string refs.
            panic!(
                "Too many strings; rewriting to long string refs is not \
                    yet supported"
            );
        }
        if self.strings.len() >= MAX_STRING_REF as usize {
            panic!("Too many distinct strings in string pool");
        }
        self.strings.push((string, 1));
        StringRef(self.strings.len() as i32)
    }

    /// Decrements the refcount of a string in the pool.
    pub fn decref(&mut self, string_ref: StringRef) {
        let index = string_ref.index();
        if index >= self.strings.len() {
            panic!(
                "decref: string_ref {} invalid, pool has only {} entries",
                string_ref.number(),
                self.strings.len()
            );
        }
        let (ref mut string, ref mut refcount) = self.strings[index];
        if *refcount < 1 {
            panic!("decref: string refcount is already zero");
        }
        self.is_modified = true;
        *refcount -= 1;
        if *refcount == 0 {
            string.clear();
        }
    }

    /// Writes to the `_StringPool` table.
    pub fn write_pool<W: Write>(&self, mut writer: W) -> io::Result<()> {
        let mut codepage_id = self.codepage.id() as u32;
        if self.long_string_refs {
            codepage_id |= LONG_STRING_REFS_BIT;
        }
        writer.write_u32::<LittleEndian>(codepage_id)?;
        for &(ref string, refcount) in self.strings.iter() {
            let length = self.codepage.encode(string.as_str()).len() as u32;
            if length > (u16::MAX as u32) {
                writer.write_u16::<LittleEndian>(0)?;
                writer.write_u16::<LittleEndian>((length >> 16) as u16)?;
            }
            writer.write_u16::<LittleEndian>((length & 0xffff) as u16)?;
            writer.write_u16::<LittleEndian>(refcount)?;
        }
        Ok(())
    }

    /// Writes to the `_StringData` table.
    pub fn write_data<W: Write>(&self, mut writer: W) -> io::Result<()> {
        for (string, _) in self.strings.iter() {
            writer.write_all(&self.codepage.encode(string.as_str()))?;
        }
        Ok(())
    }
}

// ========================================================================= //

#[cfg(test)]
mod tests {
    use super::{StringPool, StringPoolBuilder, StringRef};
    use crate::internal::codepage::CodePage;

    #[test]
    fn read_string_ref() {
        let mut input: &[u8] = b"\x00\x00";
        assert_eq!(StringRef::read(&mut input, false).unwrap(), None);

        let mut input: &[u8] = b"\x34\x12";
        assert_eq!(
            StringRef::read(&mut input, false).unwrap(),
            Some(StringRef(0x1234))
        );

        let mut input: &[u8] = b"\x00\x00\x00";
        assert_eq!(StringRef::read(&mut input, true).unwrap(), None);

        let mut input: &[u8] = b"\x00\x00\x02";
        assert_eq!(
            StringRef::read(&mut input, true).unwrap(),
            Some(StringRef(0x20000))
        );

        let mut input: &[u8] = b"\x56\x34\x12";
        assert_eq!(
            StringRef::read(&mut input, true).unwrap(),
            Some(StringRef(0x123456))
        );
    }

    #[test]
    fn write_string_ref() {
        let mut output = Vec::<u8>::new();
        StringRef::write(&mut output, None, false).unwrap();
        assert_eq!(&output as &[u8], b"\x00\x00");

        let mut output = Vec::<u8>::new();
        StringRef::write(&mut output, Some(StringRef(0x1234)), false).unwrap();
        assert_eq!(&output as &[u8], b"\x34\x12");

        let mut output = Vec::<u8>::new();
        StringRef::write(&mut output, None, true).unwrap();
        assert_eq!(&output as &[u8], b"\x00\x00\x00");

        let mut output = Vec::<u8>::new();
        StringRef::write(&mut output, Some(StringRef(0x123456)), true)
            .unwrap();
        assert_eq!(&output as &[u8], b"\x56\x34\x12");
    }

    #[test]
    fn new_string_pool() {
        let mut string_pool = StringPool::new(CodePage::default());
        assert!(!string_pool.long_string_refs());
        assert_eq!(string_pool.num_strings(), 0);
        assert_eq!(string_pool.incref("Foo".to_string()), StringRef(1));
        assert_eq!(string_pool.num_strings(), 1);
        assert_eq!(string_pool.incref("Quux".to_string()), StringRef(2));
        assert_eq!(string_pool.num_strings(), 2);
        assert_eq!(string_pool.incref("Foo".to_string()), StringRef(1));
        assert_eq!(string_pool.num_strings(), 2);
        assert_eq!(string_pool.get(StringRef(1)), "Foo");
        assert_eq!(string_pool.refcount(StringRef(1)), 2);
        assert_eq!(string_pool.get(StringRef(2)), "Quux");
        assert_eq!(string_pool.refcount(StringRef(2)), 1);
    }

    #[test]
    fn build_string_pool() {
        let pool: &[u8] = b"\xe9\xfd\x00\x00\x03\x00\x02\x00\x04\x00\x07\x00";
        let data: &[u8] = b"FooQuux";
        let builder = StringPoolBuilder::read_from_pool(pool).expect("pool");
        let string_pool = builder.build_from_data(data).expect("data");
        assert_eq!(string_pool.codepage(), CodePage::Utf8);
        assert!(!string_pool.long_string_refs());
        assert_eq!(string_pool.num_strings(), 2);
        assert_eq!(string_pool.get(StringRef(1)), "Foo");
        assert_eq!(string_pool.refcount(StringRef(1)), 2);
        assert_eq!(string_pool.get(StringRef(2)), "Quux");
        assert_eq!(string_pool.refcount(StringRef(2)), 7);
    }

    #[test]
    fn write_string_pool() {
        let mut string_pool = StringPool::new(CodePage::Windows1252);
        assert_eq!(string_pool.incref("Foo".to_string()), StringRef(1));
        assert_eq!(string_pool.incref("Quux".to_string()), StringRef(2));
        assert_eq!(string_pool.incref("Foo".to_string()), StringRef(1));
        let mut pool_output = Vec::<u8>::new();
        string_pool.write_pool(&mut pool_output).expect("pool");
        assert_eq!(
            &pool_output as &[u8],
            b"\xe4\x04\x00\x00\x03\x00\x02\x00\x04\x00\x01\x00"
        );
        let mut data_output = Vec::<u8>::new();
        string_pool.write_data(&mut data_output).expect("data");
        assert_eq!(&data_output as &[u8], b"FooQuux");
    }

    #[test]
    fn long_string_refs() {
        let pool: &[u8] = b"\xe4\x04\x00\x80\x03\x00\x02\x00\x04\x00\x07\x00";
        let data: &[u8] = b"FooQuux";
        let builder = StringPoolBuilder::read_from_pool(pool).expect("pool");
        let string_pool = builder.build_from_data(data).expect("data");
        assert_eq!(string_pool.codepage(), CodePage::Windows1252);
        assert!(string_pool.long_string_refs());
        assert_eq!(string_pool.num_strings(), 2);
    }

    #[test]
    fn repeated_string() {
        let pool: &[u8] = b"\xe9\xfd\x00\x00\x03\x00\x02\x00\x03\x00\x07\x00";
        let data: &[u8] = b"FooFoo";
        let builder = StringPoolBuilder::read_from_pool(pool).expect("pool");
        let string_pool = builder.build_from_data(data).expect("data");
        assert_eq!(string_pool.num_strings(), 2);
        assert_eq!(string_pool.get(StringRef(1)), "Foo");
        assert_eq!(string_pool.refcount(StringRef(1)), 2);
        assert_eq!(string_pool.get(StringRef(2)), "Foo");
        assert_eq!(string_pool.refcount(StringRef(2)), 7);
    }

    #[test]
    fn string_over_64k() {
        let pool: &[u8] = b"\xe9\xfd\x00\x00\x00\x00\x01\x00\x70\x11\x01\x00";

        let mut rustmsi_x10000: [u8; 70000] = [0; 70000];
        for i in 0..10000 {
            rustmsi_x10000[7 * i] = b'r';
            rustmsi_x10000[7 * i + 1] = b'u';
            rustmsi_x10000[7 * i + 2] = b's';
            rustmsi_x10000[7 * i + 3] = b't';
            rustmsi_x10000[7 * i + 4] = b'm';
            rustmsi_x10000[7 * i + 5] = b's';
            rustmsi_x10000[7 * i + 6] = b'i';
        }
        let data: &[u8] = &rustmsi_x10000;
        assert_eq!(data.len(), 70000);
        let builder = StringPoolBuilder::read_from_pool(pool).expect("pool");
        let string_pool = builder.build_from_data(data).expect("data");
        assert_eq!(string_pool.codepage(), CodePage::Utf8);
        assert!(!string_pool.long_string_refs());
        assert_eq!(string_pool.num_strings(), 1);
        assert_eq!(string_pool.get(StringRef(1)), "rustmsi".repeat(10000));
        assert_eq!(string_pool.refcount(StringRef(1)), 1);
    }

    #[test]
    fn max_refcount() {
        let pool: &[u8] = b"\xe9\xfd\x00\x00\x06\x00\xfe\xff";
        let data: &[u8] = b"Foobar";
        let builder = StringPoolBuilder::read_from_pool(pool).expect("pool");
        let mut string_pool = builder.build_from_data(data).expect("data");
        assert_eq!(string_pool.num_strings(), 1);
        assert_eq!(string_pool.get(StringRef(1)), "Foobar");
        assert_eq!(string_pool.refcount(StringRef(1)), 0xfffe);
        assert_eq!(string_pool.incref("Foobar".to_string()), StringRef(1));
        assert_eq!(string_pool.num_strings(), 1);
        assert_eq!(string_pool.incref("Foobar".to_string()), StringRef(2));
        assert_eq!(string_pool.num_strings(), 2);
        assert_eq!(string_pool.refcount(StringRef(1)), 0xffff);
        assert_eq!(string_pool.refcount(StringRef(2)), 1);
    }

    #[test]
    fn reuse_entries() {
        let mut string_pool = StringPool::new(CodePage::default());
        assert_eq!(string_pool.incref("Foo".to_string()), StringRef(1));
        assert_eq!(string_pool.incref("Bar".to_string()), StringRef(2));
        assert_eq!(string_pool.num_strings(), 2);
        string_pool.decref(StringRef(1));
        assert_eq!(string_pool.refcount(StringRef(1)), 0);
        assert_eq!(string_pool.get(StringRef(1)), "");
        assert_eq!(string_pool.num_strings(), 2);
        assert_eq!(string_pool.incref("Quux".to_string()), StringRef(1));
        assert_eq!(string_pool.num_strings(), 2);
        assert_eq!(string_pool.refcount(StringRef(1)), 1);
        assert_eq!(string_pool.get(StringRef(1)), "Quux");
    }

    #[test]
    #[should_panic(expected = "Unknown codepage for string pool (123456)")]
    fn invalid_codepage() {
        let pool: &[u8] = b"\x40\xe2\x01\x00\x06\x00\x01\x00";
        StringPoolBuilder::read_from_pool(pool).unwrap();
    }
}

// ========================================================================= //
