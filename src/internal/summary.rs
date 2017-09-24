use internal::propset::{OperatingSystem, PropertySet, PropertyValue};
use std::io::{self, Read, Seek, Write};

// ========================================================================= //

// This constant is this UUID:
//     F29F85E0-4FF9-1068-AB91-08002B27B3D9
// Which comes from this page:
//     https://msdn.microsoft.com/en-us/library/windows/desktop/
//     aa380052(v=vs.85).aspx
// The first three fields are in little-endian, and the last two in big-endian,
// because that's how Windows encodes UUIDs.  For details, see:
//     https://en.wikipedia.org/wiki/Universally_unique_identifier#Encoding
const FMTID: [u8; 16] = [0xe0, 0x85, 0x9f, 0xf2, 0xf9, 0x4f, 0x68, 0x10,
                         0xab, 0x91, 0x08, 0x00, 0x2b, 0x27, 0xb3, 0xd9];

const PROPERTY_TITLE: u32 = 2;
const PROPERTY_AUTHOR: u32 = 4;
// TODO: Support other properties.

// ========================================================================= //

/// Summary information (e.g. title, author) about an MSI package.
pub struct SummaryInfo {
    properties: PropertySet,
}

impl SummaryInfo {
    /// Creates an empty `SummaryInfo` with no properties set.
    pub fn new() -> SummaryInfo {
        let properties = PropertySet::new(OperatingSystem::Win32, 10, FMTID);
        SummaryInfo { properties: properties }
    }

    pub(crate) fn read<R: Read + Seek>(reader: R) -> io::Result<SummaryInfo> {
        let properties = PropertySet::read(reader)?;
        if properties.format_identifier() != &FMTID {
            invalid_data!("Property set has wrong format identifier");
        }
        Ok(SummaryInfo { properties: properties })
    }

    pub(crate) fn write<W: Write>(&self, writer: W) -> io::Result<()> {
        self.properties.write(writer)
    }

    /// Gets the "author" property, if one is set.
    pub fn author(&self) -> Option<&str> {
        match self.properties.get(PROPERTY_AUTHOR) {
            Some(&PropertyValue::LpStr(ref author)) => Some(author.as_str()),
            _ => None,
        }
    }

    /// Sets the "author" property.
    pub fn set_author(&mut self, author: String) {
        self.properties.set(PROPERTY_AUTHOR, PropertyValue::LpStr(author));
    }

    /// Gets the "title" property, if one is set.
    pub fn title(&self) -> Option<&str> {
        match self.properties.get(PROPERTY_TITLE) {
            Some(&PropertyValue::LpStr(ref title)) => Some(title.as_str()),
            _ => None,
        }
    }

    /// Sets the "title" property.
    pub fn set_title(&mut self, title: String) {
        self.properties.set(PROPERTY_TITLE, PropertyValue::LpStr(title));
    }
}

// ========================================================================= //
