use byteorder::{LittleEndian, ReadBytesExt};
use internal::codepage::CodePage;
use std::io::{self, Read};
use std::u16;

// ========================================================================= //

pub struct StringPoolBuilder {
    codepage: CodePage,
    long_string_refs: bool,
    lengths_and_refcounts: Vec<(u32, u16)>,
}

impl StringPoolBuilder {
    pub fn read_from_pool<R: Read>(mut reader: R)
                                   -> io::Result<StringPoolBuilder> {
        let codepage_id = reader.read_u32::<LittleEndian>()?;
        let long_string_refs = (codepage_id & 0x80000000) != 0;
        let codepage_id = (codepage_id & 0x7fffffff) as i32;
        let codepage = match CodePage::from_id(codepage_id) {
            Some(codepage) => codepage,
            None => {
                invalid_data!("Unknown codepage for string pool ({})",
                              codepage_id)
            }
        };
        let mut lengths_and_refcounts = Vec::<(u32, u16)>::new();
        loop {
            let mut length = match reader.read_u16::<LittleEndian>() {
                Ok(length) => length as u32,
                Err(_) => {
                    break;
                }
            };
            let mut refcount = reader.read_u16::<LittleEndian>()?;
            if length == 0 && refcount > 0 {
                length = ((refcount as u32) << 16) &
                         (reader.read_u16::<LittleEndian>()? as u32);
                refcount = reader.read_u16::<LittleEndian>()?;
            }
            lengths_and_refcounts.push((length, refcount));
        }
        Ok(StringPoolBuilder {
            codepage: codepage,
            long_string_refs: long_string_refs,
            lengths_and_refcounts: lengths_and_refcounts,
        })
    }

    pub fn build_from_data<R: Read>(self, mut reader: R)
                                    -> io::Result<StringPool> {
        let mut strings = Vec::<(String, u16)>::new();
        for (length, refcount) in self.lengths_and_refcounts.into_iter() {
            let mut buffer = vec![0u8; length as usize];
            reader.read_exact(&mut buffer)?;
            strings.push((self.codepage.decode(&buffer), refcount));
        }
        Ok(StringPool {
            codepage: self.codepage,
            strings: strings,
            long_string_refs: self.long_string_refs,
        })
    }
}

// ========================================================================= //

/// The string pool for an MSI package.
pub struct StringPool {
    codepage: CodePage,
    strings: Vec<(String, u16)>,
    long_string_refs: bool,
}

impl StringPool {
    /// Creates a new, empty string pool.
    pub fn new() -> StringPool {
        StringPool {
            codepage: CodePage::default(),
            strings: Vec::new(),
            long_string_refs: false,
        }
    }

    /// Gets the code page used for serializing the string data.
    pub fn codepage(&self) -> CodePage { self.codepage }

    /// Returns the number of strings in the string pool (including empty
    /// entries).
    pub fn num_strings(&self) -> u32 { self.strings.len() as u32 }

    /// Returns the number of bytes that should be used to encode a reference
    /// to a string in the string pool (either 2 or 3).  References are encoded
    /// little-endian.
    pub fn bytes_per_string_ref(&self) -> usize {
        if self.long_string_refs { 3 } else { 2 }
    }

    /// Reads a string reference from the given reader and returns the
    /// corresponding string from the pool.
    pub fn read_string_ref<R: Read>(&self, reader: &mut R)
                                    -> io::Result<&str> {
        let mut string_ref = reader.read_u16::<LittleEndian>()? as u32;
        if self.long_string_refs {
            string_ref &= (reader.read_u8()? as u32) << 16;
        }
        Ok(self.get(string_ref))
    }

    /// Returns the string in the pool for the given reference.
    pub fn get(&self, string_ref: u32) -> &str {
        if string_ref > 0 {
            let index = (string_ref - 1) as usize;
            if index < self.strings.len() {
                return self.strings[index].0.as_str();
            }
        }
        ""
    }

    /// Returns the pool's refcount for the given string reference.
    pub fn refcount(&self, string_ref: u32) -> u16 {
        if string_ref > 0 {
            let index = (string_ref - 1) as usize;
            if index < self.strings.len() {
                return self.strings[index].1;
            }
        }
        0
    }

    /// Inserts a string into the pool, or increments its refcount if it's
    /// already in the pool, and returns the index of the string in the pool.
    pub fn incref(&mut self, string: String) -> u32 {
        // TODO: change the internal representation of StringPool to make this
        // more efficient.
        for (index, &mut (ref mut st, ref mut refcount)) in
            self.strings.iter_mut().enumerate() {
            if *refcount == 0 {
                debug_assert_eq!(st, "");
                *st = string;
                *refcount = 1;
                return (index + 1) as u32;
            }
            if *st == string && *refcount < u16::MAX {
                *refcount += 1;
                return (index + 1) as u32;
            }
        }
        self.strings.push((string, 1));
        self.strings.len() as u32
    }

    /// Decrements the refcount of a string in the pool.
    pub fn decref(&mut self, string_ref: u32) {
        if string_ref < 1 {
            panic!("decref: zero is not a valid string_ref");
        }
        let index = (string_ref - 1) as usize;
        if index >= self.strings.len() {
            panic!("decref: string_ref {} invalid, pool has only {} entries",
                   string_ref,
                   self.strings.len());
        }
        let (ref mut string, ref mut refcount) = self.strings[index];
        if *refcount < 1 {
            panic!("decref: string refcount is already zero");
        }
        *refcount -= 1;
        if *refcount == 0 {
            string.clear();
        }
    }
}

// ========================================================================= //

#[cfg(test)]
mod tests {
    use super::{StringPool, StringPoolBuilder};
    use internal::codepage::CodePage;

    #[test]
    fn new_string_pool() {
        let mut string_pool = StringPool::new();
        assert_eq!(string_pool.bytes_per_string_ref(), 2);
        assert_eq!(string_pool.num_strings(), 0);
        assert_eq!(string_pool.incref("Foo".to_string()), 1);
        assert_eq!(string_pool.num_strings(), 1);
        assert_eq!(string_pool.incref("Quux".to_string()), 2);
        assert_eq!(string_pool.num_strings(), 2);
        assert_eq!(string_pool.incref("Foo".to_string()), 1);
        assert_eq!(string_pool.num_strings(), 2);
        assert_eq!(string_pool.get(1), "Foo");
        assert_eq!(string_pool.refcount(1), 2);
        assert_eq!(string_pool.get(2), "Quux");
        assert_eq!(string_pool.refcount(2), 1);
    }

    #[test]
    fn build_string_pool() {
        let pool: &[u8] = b"\xe9\xfd\x00\x00\x03\x00\x02\x00\x04\x00\x07\x00";
        let data: &[u8] = b"FooQuux";
        let builder = StringPoolBuilder::read_from_pool(pool).expect("pool");
        let string_pool = builder.build_from_data(data).expect("data");
        assert_eq!(string_pool.codepage(), CodePage::Utf8);
        assert_eq!(string_pool.bytes_per_string_ref(), 2);
        assert_eq!(string_pool.num_strings(), 2);
        assert_eq!(string_pool.get(1), "Foo");
        assert_eq!(string_pool.refcount(1), 2);
        assert_eq!(string_pool.get(2), "Quux");
        assert_eq!(string_pool.refcount(2), 7);
    }

    #[test]
    fn long_string_refs() {
        let pool: &[u8] = b"\xe4\x04\x00\x80\x03\x00\x02\x00\x04\x00\x07\x00";
        let data: &[u8] = b"FooQuux";
        let builder = StringPoolBuilder::read_from_pool(pool).expect("pool");
        let string_pool = builder.build_from_data(data).expect("data");
        assert_eq!(string_pool.codepage(), CodePage::Windows1252);
        assert_eq!(string_pool.bytes_per_string_ref(), 3);
        assert_eq!(string_pool.num_strings(), 2);
    }

    #[test]
    fn repeated_string() {
        let pool: &[u8] = b"\xe9\xfd\x00\x00\x03\x00\x02\x00\x03\x00\x07\x00";
        let data: &[u8] = b"FooFoo";
        let builder = StringPoolBuilder::read_from_pool(pool).expect("pool");
        let string_pool = builder.build_from_data(data).expect("data");
        assert_eq!(string_pool.num_strings(), 2);
        assert_eq!(string_pool.get(1), "Foo");
        assert_eq!(string_pool.refcount(1), 2);
        assert_eq!(string_pool.get(2), "Foo");
        assert_eq!(string_pool.refcount(2), 7);
    }

    #[test]
    fn max_refcount() {
        let pool: &[u8] = b"\xe9\xfd\x00\x00\x06\x00\xfe\xff";
        let data: &[u8] = b"Foobar";
        let builder = StringPoolBuilder::read_from_pool(pool).expect("pool");
        let mut string_pool = builder.build_from_data(data).expect("data");
        assert_eq!(string_pool.num_strings(), 1);
        assert_eq!(string_pool.get(1), "Foobar");
        assert_eq!(string_pool.refcount(1), 0xfffe);
        assert_eq!(string_pool.incref("Foobar".to_string()), 1);
        assert_eq!(string_pool.num_strings(), 1);
        assert_eq!(string_pool.incref("Foobar".to_string()), 2);
        assert_eq!(string_pool.num_strings(), 2);
        assert_eq!(string_pool.refcount(1), 0xffff);
        assert_eq!(string_pool.refcount(2), 1);
    }

    #[test]
    fn reuse_entries() {
        let mut string_pool = StringPool::new();
        assert_eq!(string_pool.incref("Foo".to_string()), 1);
        assert_eq!(string_pool.incref("Bar".to_string()), 2);
        assert_eq!(string_pool.num_strings(), 2);
        string_pool.decref(1);
        assert_eq!(string_pool.refcount(1), 0);
        assert_eq!(string_pool.get(1), "");
        assert_eq!(string_pool.num_strings(), 2);
        assert_eq!(string_pool.incref("Quux".to_string()), 1);
        assert_eq!(string_pool.num_strings(), 2);
        assert_eq!(string_pool.refcount(1), 1);
        assert_eq!(string_pool.get(1), "Quux");
    }

    #[test]
    #[should_panic(expected = "Unknown codepage for string pool (123456)")]
    fn invalid_codepage() {
        let pool: &[u8] = b"\x40\xe2\x01\x00\x06\x00\x01\x00";
        StringPoolBuilder::read_from_pool(pool).unwrap();
    }
}

// ========================================================================= //
