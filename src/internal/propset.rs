// This module gives a (partial) implementation of Windows property sets,
// whose serialization format is described at:
//     https://msdn.microsoft.com/en-us/library/windows/desktop/
//     aa380367(v=vs.85).aspx

use crate::internal;
use crate::internal::codepage::CodePage;
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use std::cmp;
use std::collections::BTreeMap;
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::time::SystemTime;

// ========================================================================= //

const BYTE_ORDER_MARK: u16 = 0xfffe;

const PROPERTY_CODEPAGE: u32 = 1;

// ========================================================================= //

#[derive(Clone, Copy, Eq, Ord, PartialEq, PartialOrd)]
enum PropertyFormatVersion {
    V0,
    V1,
}

impl PropertyFormatVersion {
    fn version_number(self) -> u16 {
        match self {
            PropertyFormatVersion::V0 => 0,
            PropertyFormatVersion::V1 => 1,
        }
    }
}

// ========================================================================= //

pub enum OperatingSystem {
    Win16,
    Macintosh,
    Win32,
}

// ========================================================================= //

#[derive(Debug, Eq, PartialEq)]
pub enum PropertyValue {
    Empty,
    Null,
    I1(i8),
    I2(i16),
    I4(i32),
    LpStr(String),
    FileTime(SystemTime),
}

impl PropertyValue {
    fn read<R: Read>(
        mut reader: R,
        codepage: CodePage,
    ) -> io::Result<PropertyValue> {
        let type_number = reader.read_u32::<LittleEndian>()?;
        match type_number {
            0 => Ok(PropertyValue::Empty),
            1 => Ok(PropertyValue::Null),
            2 => Ok(PropertyValue::I2(reader.read_i16::<LittleEndian>()?)),
            3 => Ok(PropertyValue::I4(reader.read_i32::<LittleEndian>()?)),
            16 => Ok(PropertyValue::I1(reader.read_i8()?)),
            30 => {
                let length = reader.read_u32::<LittleEndian>()?;
                let length = if length == 0 { 0 } else { length - 1 };
                let mut bytes: Vec<u8> = Vec::with_capacity(length as usize);
                for _ in 0..length {
                    bytes.push(reader.read_u8()?);
                }
                if reader.read_u8()? != 0 {
                    invalid_data!("Property set string not null-terminated");
                }
                Ok(PropertyValue::LpStr(codepage.decode(&bytes)))
            }
            64 => {
                let value = reader.read_u64::<LittleEndian>()?;
                let timestamp =
                    internal::time::system_time_from_filetime(value);
                Ok(PropertyValue::FileTime(timestamp))
            }
            _ => {
                invalid_data!(
                    "Unknown property set value type ({})",
                    type_number
                );
            }
        }
    }

    fn write<W: Write>(
        &self,
        mut writer: W,
        codepage: CodePage,
    ) -> io::Result<()> {
        match self {
            PropertyValue::Empty => {
                writer.write_u32::<LittleEndian>(0)?;
            }
            PropertyValue::Null => {
                writer.write_u32::<LittleEndian>(1)?;
            }
            PropertyValue::I1(value) => {
                writer.write_u32::<LittleEndian>(16)?;
                writer.write_i8(*value)?;
                writer.write_u8(0)?; // Padding
                writer.write_u16::<LittleEndian>(0)?; // Padding
            }
            PropertyValue::I2(value) => {
                writer.write_u32::<LittleEndian>(2)?;
                writer.write_i16::<LittleEndian>(*value)?;
                writer.write_u16::<LittleEndian>(0)?; // Padding
            }
            PropertyValue::I4(value) => {
                writer.write_u32::<LittleEndian>(3)?;
                writer.write_i32::<LittleEndian>(*value)?;
            }
            PropertyValue::LpStr(ref string) => {
                writer.write_u32::<LittleEndian>(30)?;
                let bytes = codepage.encode(string.as_str());
                let length = (bytes.len() + 1) as u32;
                writer.write_u32::<LittleEndian>(length)?;
                writer.write_all(&bytes)?;
                writer.write_u8(0)?; // Null terminator
                let padding = (((length + 3) >> 2) << 2) - length;
                for _ in 0..padding {
                    writer.write_u8(0)?;
                }
            }
            PropertyValue::FileTime(timestamp) => {
                let value =
                    internal::time::filetime_from_system_time(*timestamp);
                writer.write_u32::<LittleEndian>(64)?;
                writer.write_u64::<LittleEndian>(value)?;
            }
        }
        Ok(())
    }

    /// Returns the number of bytes, including any padding bytes, that will be
    /// written by the `write()` method.  Always returns a multiple of four.
    fn size_including_padding(&self) -> u32 {
        match self {
            PropertyValue::Empty => 4,
            PropertyValue::Null => 4,
            PropertyValue::I1(_) => 8,
            PropertyValue::I2(_) => 8,
            PropertyValue::I4(_) => 8,
            PropertyValue::LpStr(ref string) => {
                ((12 + string.len() as u32) >> 2) << 2
            }
            PropertyValue::FileTime(_) => 12,
        }
    }

    /// Returns the minimum format version at which this value type is
    /// supported.
    fn minimum_version(&self) -> PropertyFormatVersion {
        match self {
            &PropertyValue::I1(_) => PropertyFormatVersion::V1,
            _ => PropertyFormatVersion::V0,
        }
    }

    /// Returns a human-readable name for this value's data type.
    fn type_name(&self) -> &str {
        match self {
            PropertyValue::Empty => "EMPTY",
            PropertyValue::Null => "NULL",
            PropertyValue::I1(_) => "I1",
            PropertyValue::I2(_) => "I2",
            PropertyValue::I4(_) => "I4",
            PropertyValue::LpStr(_) => "LPSTR",
            PropertyValue::FileTime(_) => "FILETIME",
        }
    }
}

// ========================================================================= //

pub struct PropertySet {
    os: OperatingSystem,
    os_version: u16,
    clsid: [u8; 16],
    fmtid: [u8; 16],
    codepage: CodePage,
    properties: BTreeMap<u32, PropertyValue>,
}

impl PropertySet {
    pub fn new(
        os: OperatingSystem,
        os_version: u16,
        fmtid: [u8; 16],
    ) -> PropertySet {
        PropertySet {
            os,
            os_version,
            clsid: [0; 16],
            fmtid,
            codepage: CodePage::default(),
            properties: BTreeMap::new(),
        }
    }

    pub fn read<R: Read + Seek>(mut reader: R) -> io::Result<PropertySet> {
        // Property set header:
        if reader.read_u16::<LittleEndian>()? != BYTE_ORDER_MARK {
            invalid_data!("Invalid byte order mark");
        }
        let format_version = {
            let version_number = reader.read_u16::<LittleEndian>()?;
            match version_number {
                0 => PropertyFormatVersion::V0,
                1 => PropertyFormatVersion::V1,
                _ => invalid_data!(
                    "Unsupported property set version ({})",
                    version_number
                ),
            }
        };
        let os_version = reader.read_u16::<LittleEndian>()?;
        let os = {
            let os_number = reader.read_u16::<LittleEndian>()?;
            match os_number {
                0 => OperatingSystem::Win16,
                1 => OperatingSystem::Macintosh,
                2 => OperatingSystem::Win32,
                _ => invalid_data!("Unsupported OS value ({})", os_number),
            }
        };
        let mut clsid: [u8; 16] = [0; 16];
        reader.read_exact(&mut clsid)?;
        let reserved = reader.read_u32::<LittleEndian>()?;
        if reserved < 1 {
            invalid_data!(
                "Invalid property set header reserved value ({})",
                reserved
            );
        }

        // Section header:
        let mut fmtid: [u8; 16] = [0; 16];
        reader.read_exact(&mut fmtid)?;
        let section_offset = reader.read_u32::<LittleEndian>()?;

        // Section:
        reader.seek(SeekFrom::Start(section_offset as u64))?;
        let _section_size = reader.read_u32::<LittleEndian>()?;
        let num_properties = reader.read_u32::<LittleEndian>()?;
        let mut property_offsets = BTreeMap::<u32, u32>::new();
        for _ in 0..num_properties {
            let name = reader.read_u32::<LittleEndian>()?;
            let offset = reader.read_u32::<LittleEndian>()?;
            if property_offsets.contains_key(&name) {
                invalid_data!("Repeated property name ({})", name);
            }
            property_offsets.insert(name, offset);
        }
        let codepage = if let Some(&offset) =
            property_offsets.get(&PROPERTY_CODEPAGE)
        {
            reader.seek(SeekFrom::Start(
                section_offset as u64 + offset as u64,
            ))?;
            let value =
                PropertyValue::read(reader.by_ref(), CodePage::default())?;
            if let PropertyValue::I2(codepage_id) = value {
                let codepage_id = codepage_id as u16;
                if let Some(codepage) = CodePage::from_id(codepage_id as i32) {
                    codepage
                } else {
                    invalid_data!(
                        "Unknown codepage for property set ({})",
                        codepage_id
                    );
                }
            } else {
                invalid_data!(
                    "Codepage property value has wrong type ({})",
                    value.type_name()
                );
            }
        } else {
            CodePage::default()
        };
        let mut property_values = BTreeMap::<u32, PropertyValue>::new();
        for (name, offset) in property_offsets.into_iter() {
            reader.seek(SeekFrom::Start(
                section_offset as u64 + offset as u64,
            ))?;
            let value = PropertyValue::read(reader.by_ref(), codepage)?;
            if value.minimum_version() > format_version {
                invalid_data!(
                    "Property value of type {} is not supported \
                               in format version {}",
                    value.type_name(),
                    format_version.version_number()
                );
            }
            debug_assert!(!property_values.contains_key(&name));
            property_values.insert(name, value);
        }
        Ok(PropertySet {
            os,
            os_version,
            clsid,
            fmtid,
            codepage,
            properties: property_values,
        })
    }

    pub fn write<W: Write>(&self, mut writer: W) -> io::Result<()> {
        // Property set header:
        writer.write_u16::<LittleEndian>(BYTE_ORDER_MARK)?;
        let mut format_version = PropertyFormatVersion::V0;
        for (_, value) in self.properties.iter() {
            format_version = cmp::max(format_version, value.minimum_version());
        }
        writer.write_u16::<LittleEndian>(format_version.version_number())?;
        writer.write_u16::<LittleEndian>(self.os_version)?;
        writer.write_u16::<LittleEndian>(match self.os {
            OperatingSystem::Win16 => 0,
            OperatingSystem::Macintosh => 1,
            OperatingSystem::Win32 => 2,
        })?;
        writer.write_all(&self.clsid)?;
        writer.write_u32::<LittleEndian>(1)?; // Reserved field

        // Section header:
        writer.write_all(&self.fmtid)?;
        writer.write_u32::<LittleEndian>(48)?; // Section offset

        // Section:
        let num_properties = self.properties.len() as u32;
        let mut section_size: u32 = 8 + 8 * num_properties;
        let mut property_offsets: Vec<u32> = Vec::new();
        for (_, value) in self.properties.iter() {
            property_offsets.push(section_size);
            section_size += value.size_including_padding();
        }
        writer.write_u32::<LittleEndian>(section_size)?;
        writer.write_u32::<LittleEndian>(num_properties)?;
        for (index, (&name, _)) in self.properties.iter().enumerate() {
            writer.write_u32::<LittleEndian>(name)?;
            writer.write_u32::<LittleEndian>(property_offsets[index])?;
        }
        for (_, value) in self.properties.iter() {
            value.write(writer.by_ref(), self.codepage)?;
        }
        Ok(())
    }

    pub fn format_identifier(&self) -> &[u8; 16] {
        &self.fmtid
    }

    pub fn codepage(&self) -> CodePage {
        self.codepage
    }

    pub fn set_codepage(&mut self, codepage: CodePage) {
        self.set(PROPERTY_CODEPAGE, PropertyValue::I2(codepage.id() as i16));
        debug_assert_eq!(self.codepage, codepage);
    }

    pub fn get(&self, property_name: u32) -> Option<&PropertyValue> {
        self.properties.get(&property_name)
    }

    pub fn set(&mut self, property_name: u32, property_value: PropertyValue) {
        if property_name == PROPERTY_CODEPAGE {
            if let PropertyValue::I2(codepage_id) = property_value {
                if let Some(codepage) = CodePage::from_id(codepage_id as i32) {
                    self.codepage = codepage;
                }
            }
        }
        self.properties.insert(property_name, property_value);
    }

    pub fn remove(&mut self, property_name: u32) {
        self.properties.remove(&property_name);
    }
}

// ========================================================================= //

#[cfg(test)]
mod tests {
    use super::{OperatingSystem, PropertySet, PropertyValue};
    use crate::internal::codepage::CodePage;
    use std::io::Cursor;
    use std::time::{Duration, UNIX_EPOCH};

    #[test]
    fn read_property_value() {
        let input: &[u8] = &[0, 0, 0, 0];
        assert_eq!(
            PropertyValue::read(input, CodePage::Utf8).unwrap(),
            PropertyValue::Empty
        );

        let input: &[u8] = &[1, 0, 0, 0];
        assert_eq!(
            PropertyValue::read(input, CodePage::Utf8).unwrap(),
            PropertyValue::Null
        );

        let input: &[u8] = &[2, 0, 0, 0, 0x2e, 0xfb];
        assert_eq!(
            PropertyValue::read(input, CodePage::Utf8).unwrap(),
            PropertyValue::I2(-1234)
        );

        let input: &[u8] = &[3, 0, 0, 0, 0x15, 0xcd, 0x5b, 0x07];
        assert_eq!(
            PropertyValue::read(input, CodePage::Utf8).unwrap(),
            PropertyValue::I4(123456789)
        );

        let input: &[u8] =
            b"\x1e\x00\x00\x00\x0e\x00\x00\x00Hello, world!\x00";
        assert_eq!(
            PropertyValue::read(input, CodePage::Utf8).unwrap(),
            PropertyValue::LpStr("Hello, world!".to_string())
        );

        let sat_2017_mar_18_at_18_46_36_gmt =
            UNIX_EPOCH + Duration::from_secs(1489862796);
        let input: &[u8] = &[64, 0, 0, 0, 0, 206, 112, 248, 23, 160, 210, 1];
        assert_eq!(
            PropertyValue::read(input, CodePage::Utf8).unwrap(),
            PropertyValue::FileTime(sat_2017_mar_18_at_18_46_36_gmt)
        );
    }

    #[test]
    fn write_property_value() {
        let value = PropertyValue::Empty;
        let mut output = Vec::<u8>::new();
        value.write(&mut output, CodePage::Utf8).unwrap();
        assert_eq!(&output as &[u8], &[0, 0, 0, 0]);

        let value = PropertyValue::Null;
        let mut output = Vec::<u8>::new();
        value.write(&mut output, CodePage::Utf8).unwrap();
        assert_eq!(&output as &[u8], &[1, 0, 0, 0]);

        let value = PropertyValue::I2(-1234);
        let mut output = Vec::<u8>::new();
        value.write(&mut output, CodePage::Utf8).unwrap();
        assert_eq!(&output as &[u8], &[2, 0, 0, 0, 0x2e, 0xfb, 0, 0]);

        let value = PropertyValue::I4(123456789);
        let mut output = Vec::<u8>::new();
        value.write(&mut output, CodePage::Utf8).unwrap();
        assert_eq!(&output as &[u8], &[3, 0, 0, 0, 0x15, 0xcd, 0x5b, 0x07]);

        let value = PropertyValue::LpStr("Hello, world!".to_string());
        let mut output = Vec::<u8>::new();
        value.write(&mut output, CodePage::Utf8).unwrap();
        assert_eq!(
            &output as &[u8],
            b"\x1e\x00\x00\x00\x0e\x00\x00\x00Hello, world!\x00\x00\x00"
        );

        let sat_2017_mar_18_at_18_46_36_gmt =
            UNIX_EPOCH + Duration::from_secs(1489862796);
        let value = PropertyValue::FileTime(sat_2017_mar_18_at_18_46_36_gmt);
        let mut output = Vec::<u8>::new();
        value.write(&mut output, CodePage::Utf8).unwrap();
        assert_eq!(
            &output as &[u8],
            &[64, 0, 0, 0, 0, 206, 112, 248, 23, 160, 210, 1]
        );
    }

    #[test]
    fn property_value_round_trip() {
        let sat_2017_mar_18_at_18_46_36_gmt =
            UNIX_EPOCH + Duration::from_secs(1489862796);

        let values = &[
            PropertyValue::Empty,
            PropertyValue::Null,
            PropertyValue::I1(-123),
            PropertyValue::I1(123),
            PropertyValue::I2(-12345),
            PropertyValue::I2(12345),
            PropertyValue::I4(-123456789),
            PropertyValue::I4(123456789),
            PropertyValue::LpStr("".to_string()),
            PropertyValue::LpStr("foo".to_string()),
            PropertyValue::LpStr("foobar".to_string()),
            PropertyValue::FileTime(sat_2017_mar_18_at_18_46_36_gmt),
        ];
        let codepage = CodePage::Utf8;
        for value in values.iter() {
            let mut output = Vec::<u8>::new();
            value.write(&mut output, codepage).unwrap();
            let parsed =
                PropertyValue::read(&output as &[u8], codepage).unwrap();
            assert_eq!(parsed, *value);
            let expected_size = value.size_including_padding();
            assert_eq!(output.len() as u32, expected_size);
            assert_eq!(expected_size % 4, 0);
        }
    }

    #[test]
    fn read_property_set() {
        let input: &[u8] = &[
            0xfe, 0xff, // Byte order mark
            1, 0, // Format version
            10, 0, 2, 0, // OS
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // CLSID
            1, 0, 0, 0, // Reserved
            1, 2, 3, 4, 5, 6, 7, 8, 1, 2, 3, 4, 5, 6, 7, 8, // FMTID
            48, 0, 0, 0, // Section offset
            48, 0, 0, 0, // Section size
            2, 0, 0, 0, // Number of properties
            4, 0, 0, 0, // 1st property name
            24, 0, 0, 0, // 1st property offset
            37, 0, 0, 0, // 2nd property name
            40, 0, 0, 0, // 2nd property offset
            30, 0, 0, 0, // 1st property type (LPSTR)
            8, 0, 0, 0, // String length (including terminating null)
            b'F', b'o', b'o', b' ', b'B', b'a', b'r', 0, // String value
            16, 0, 0, 0, // 2nd property type (I1)
            253, 0, 0, 0, // Int value (and padding)
        ];
        let property_set = PropertySet::read(Cursor::new(input)).unwrap();
        assert_eq!(
            property_set.format_identifier(),
            &[1, 2, 3, 4, 5, 6, 7, 8, 1, 2, 3, 4, 5, 6, 7, 8]
        );
        assert_eq!(
            property_set.get(4),
            Some(&PropertyValue::LpStr("Foo Bar".to_string()))
        );
        assert_eq!(property_set.get(37), Some(&PropertyValue::I1(-3)));
    }

    #[test]
    fn write_property_set() {
        let fmtid: [u8; 16] = [1, 2, 3, 4, 5, 6, 7, 8, 1, 2, 3, 4, 5, 6, 7, 8];
        let mut property_set =
            PropertySet::new(OperatingSystem::Win32, 10, fmtid);
        property_set.set(4, PropertyValue::LpStr("Foo Bar".to_string()));
        property_set.set(37, PropertyValue::I1(-3));
        let mut output = Vec::<u8>::new();
        property_set.write(&mut output).unwrap();
        let expected: &[u8] = &[
            0xfe, 0xff, // Byte order mark
            1, 0, // Format version
            10, 0, 2, 0, // OS
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // CLSID
            1, 0, 0, 0, // Reserved
            1, 2, 3, 4, 5, 6, 7, 8, 1, 2, 3, 4, 5, 6, 7, 8, // FMTID
            48, 0, 0, 0, // Section offset
            48, 0, 0, 0, // Section size
            2, 0, 0, 0, // Number of properties
            4, 0, 0, 0, // 1st property name
            24, 0, 0, 0, // 1st property offset
            37, 0, 0, 0, // 2nd property name
            40, 0, 0, 0, // 2nd property offset
            30, 0, 0, 0, // 1st property type (LPSTR)
            8, 0, 0, 0, // String length (including terminating null)
            b'F', b'o', b'o', b' ', b'B', b'a', b'r', 0, // String value
            16, 0, 0, 0, // 2nd property type (I1)
            253, 0, 0, 0, // Int value (and padding)
        ];
        assert_eq!(&output as &[u8], expected);
    }
}

// ========================================================================= //
