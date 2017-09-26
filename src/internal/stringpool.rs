use byteorder::{LittleEndian, ReadBytesExt};
use internal::codepage::CodePage;
use ordermap::OrderMap;
use std::io::{self, Read};
use std::u16;

// ========================================================================= //

pub struct StringPoolBuilder {
    codepage: CodePage,
    lengths_and_refcounts: Vec<(u32, u16)>,
}

impl StringPoolBuilder {
    pub fn read_from_pool<R: Read>(mut reader: R)
                                   -> io::Result<StringPoolBuilder> {
        let codepage_id = reader.read_u32::<LittleEndian>()?;
        let _more_than_64k_strings = (codepage_id & 0x8000000) != 0;
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
                Err(_) => break,
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
            lengths_and_refcounts: lengths_and_refcounts,
        })
    }

    pub fn build_from_data<R: Read>(self, mut reader: R)
                                    -> io::Result<StringPool> {
        let mut strings = OrderMap::<String, (u32, u16)>::new();
        for (index, (length, refcount)) in
            self.lengths_and_refcounts.into_iter().enumerate() {
            let mut buffer = vec![0u8; length as usize];
            reader.read_exact(&mut buffer)?;
            let string = self.codepage.decode(&buffer);
            if strings.contains_key(&string) {
                invalid_data!("Repeated string value in string pool");
            }
            strings.insert(string, (index as u32, refcount));
        }
        Ok(StringPool { strings: strings })
    }
}

// ========================================================================= //

/// The string pool for an MSI package.
pub struct StringPool {
    strings: OrderMap<String, (u32, u16)>,
}

impl StringPool {
    /// Returns the number of strings in the string pool (including empty
    /// entries).
    pub fn num_strings(&self) -> u32 { self.strings.len() as u32 }

    /// Returns the string in the pool with the given index.
    pub fn get(&self, index: u32) -> &str {
        let entry = self.strings.iter().nth(index as usize);
        if let Some((ref string, &(stored_index, _))) = entry {
            debug_assert_eq!(stored_index, index);
            string.as_str()
        } else {
            ""
        }
    }

    /// Returns the refcount of the string in the pool with the given index.
    pub fn refcount(&self, index: u32) -> u16 {
        let entry = self.strings.iter().nth(index as usize);
        if let Some((_, &(stored_index, refcount))) = entry {
            debug_assert_eq!(stored_index, index);
            refcount
        } else {
            0
        }
    }

    /// Inserts a string into the pool, or increments its refcount if it's
    /// already in the pool, and returns the index of the string in the pool.
    pub fn incref(&mut self, string: String) -> u32 {
        if let Some(&mut (index, ref mut refcount)) =
            self.strings.get_mut(&string) {
            if *refcount == u16::MAX {
                panic!("incref: too many references to string");
            }
            *refcount += 1;
            return index;
        }
        let index = self.strings.len() as u32;
        self.strings.insert(string, (index, 1));
        index
    }

    /// Decrements the refcount of a string in the pool.
    pub fn decref(&mut self, string: &str) {
        if let Some(&mut (_, ref mut refcount)) =
            self.strings.get_mut(string) {
            if *refcount == 0 {
                panic!("decref: string refcount is already zero");
            }
            *refcount -= 1;
        } else {
            panic!("decref: string not present in StringPool");
        }
    }
}

// ========================================================================= //
