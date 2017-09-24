// This module gives a (partial) implementation of Windows property sets,
// whose serialization format is described at:
//     https://msdn.microsoft.com/en-us/library/windows/desktop/
//     aa380367(v=vs.85).aspx

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use std::cmp;
use std::io::{self, Read, Seek, SeekFrom, Write};

// ========================================================================= //

const BYTE_ORDER_MARK: u16 = 0xfffe;

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
    FileTime(u64),
}

impl PropertyValue {
    fn read<R: Read>(mut reader: R) -> io::Result<PropertyValue> {
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
                let string = String::from_utf8_lossy(&bytes).to_string();
                Ok(PropertyValue::LpStr(string))
            }
            64 => {
                let value = reader.read_u64::<LittleEndian>()?;
                Ok(PropertyValue::FileTime(value))
            }
            _ => {
                invalid_data!("Unknown property set value type ({})",
                              type_number);
            }
        }
    }

    fn write<W: Write>(&self, mut writer: W) -> io::Result<()> {
        match self {
            &PropertyValue::Empty => {
                writer.write_u32::<LittleEndian>(0)?;
            }
            &PropertyValue::Null => {
                writer.write_u32::<LittleEndian>(1)?;
            }
            &PropertyValue::I1(value) => {
                writer.write_u32::<LittleEndian>(16)?;
                writer.write_i8(value)?;
                writer.write_u8(0)?; // Padding
                writer.write_u16::<LittleEndian>(0)?; // Padding
            }
            &PropertyValue::I2(value) => {
                writer.write_u32::<LittleEndian>(2)?;
                writer.write_i16::<LittleEndian>(value)?;
                writer.write_u16::<LittleEndian>(0)?; // Padding
            }
            &PropertyValue::I4(value) => {
                writer.write_u32::<LittleEndian>(3)?;
                writer.write_i32::<LittleEndian>(value)?;
            }
            &PropertyValue::LpStr(ref string) => {
                writer.write_u32::<LittleEndian>(30)?;
                let length = (string.len() + 1) as u32;
                writer.write_u32::<LittleEndian>(length)?;
                for byte in string.bytes() {
                    writer.write_u8(byte)?;
                }
                writer.write_u8(0)?; // Null terminator
                let padding = (((length + 3) >> 2) << 2) - length;
                for _ in 0..padding {
                    writer.write_u8(0)?;
                }
            }
            &PropertyValue::FileTime(value) => {
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
            &PropertyValue::Empty => 4,
            &PropertyValue::Null => 4,
            &PropertyValue::I1(_) => 8,
            &PropertyValue::I2(_) => 8,
            &PropertyValue::I4(_) => 8,
            &PropertyValue::LpStr(ref string) => {
                (((12 + string.len() as u32) >> 2) << 2)
            }
            &PropertyValue::FileTime(_) => 12,
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
            &PropertyValue::Empty => "EMPTY",
            &PropertyValue::Null => "NULL",
            &PropertyValue::I1(_) => "I1",
            &PropertyValue::I2(_) => "I2",
            &PropertyValue::I4(_) => "I4",
            &PropertyValue::LpStr(_) => "LPSTR",
            &PropertyValue::FileTime(_) => "FILETIME",
        }
    }
}

// ========================================================================= //

pub struct PropertySet {
    os: OperatingSystem,
    os_version: u16,
    clsid: [u8; 16],
    fmtid: [u8; 16],
    properties: Vec<(u32, PropertyValue)>,
}

impl PropertySet {
    pub fn new(os: OperatingSystem, os_version: u16, fmtid: [u8; 16])
               -> PropertySet {
        PropertySet {
            os: os,
            os_version: os_version,
            clsid: [0; 16],
            fmtid: fmtid,
            properties: Vec::new(),
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
                _ => {
                    invalid_data!("Unsupported property set version ({})",
                                  version_number)
                }
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
            invalid_data!("Invalid property set header reserved value ({})",
                          reserved);
        }

        // Section header:
        let mut fmtid: [u8; 16] = [0; 16];
        reader.read_exact(&mut fmtid)?;
        let section_offset = reader.read_u32::<LittleEndian>()?;

        // Section:
        reader.seek(SeekFrom::Start(section_offset as u64))?;
        let _section_size = reader.read_u32::<LittleEndian>()?;
        let num_properties = reader.read_u32::<LittleEndian>()?;
        let mut property_offsets: Vec<(u32, u32)> = Vec::new();
        for _ in 0..num_properties {
            let name = reader.read_u32::<LittleEndian>()?;
            let offset = reader.read_u32::<LittleEndian>()?;
            property_offsets.push((name, offset));
        }
        let mut property_values: Vec<(u32, PropertyValue)> = Vec::new();
        for (name, offset) in property_offsets {
            reader.seek(SeekFrom::Start(section_offset as u64 +
                                        offset as u64))?;
            let value = PropertyValue::read(reader.by_ref())?;
            if value.minimum_version() > format_version {
                invalid_data!("Property value of type {} is not supported \
                               in format version {}",
                              value.type_name(),
                              format_version.version_number());
            }
            property_values.push((name, value));
        }
        Ok(PropertySet {
            os: os,
            os_version: os_version,
            clsid: clsid,
            fmtid: fmtid,
            properties: property_values,
        })
    }

    pub fn write<W: Write>(&self, mut writer: W) -> io::Result<()> {
        // Property set header:
        writer.write_u16::<LittleEndian>(BYTE_ORDER_MARK)?;
        let mut format_version = PropertyFormatVersion::V0;
        for &(_, ref value) in self.properties.iter() {
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
        for &(_, ref value) in self.properties.iter() {
            property_offsets.push(section_size);
            section_size += value.size_including_padding();
        }
        writer.write_u32::<LittleEndian>(section_size)?;
        writer.write_u32::<LittleEndian>(num_properties)?;
        for (index, &(name, _)) in self.properties.iter().enumerate() {
            writer.write_u32::<LittleEndian>(name)?;
            writer.write_u32::<LittleEndian>(property_offsets[index])?;
        }
        for &(_, ref value) in self.properties.iter() {
            value.write(writer.by_ref())?;
        }
        Ok(())
    }

    pub fn format_identifier(&self) -> &[u8; 16] { &self.fmtid }

    pub fn get(&self, property_name: u32) -> Option<&PropertyValue> {
        for &(name, ref value) in self.properties.iter() {
            if name == property_name {
                return Some(value);
            }
        }
        None
    }

    pub fn set(&mut self, property_name: u32, property_value: PropertyValue) {
        for &mut (name, ref mut value) in self.properties.iter_mut() {
            if name == property_name {
                *value = property_value;
                return;
            }
        }
        self.properties.push((property_name, property_value));
    }
}

// ========================================================================= //

#[cfg(test)]
mod tests {
    use super::{OperatingSystem, PropertySet, PropertyValue};
    use std::io::Cursor;

    #[test]
    fn read_property_value() {
        let input: &[u8] = &[0, 0, 0, 0];
        assert_eq!(PropertyValue::read(input).unwrap(), PropertyValue::Empty);

        let input: &[u8] = &[1, 0, 0, 0];
        assert_eq!(PropertyValue::read(input).unwrap(), PropertyValue::Null);

        let input: &[u8] = &[2, 0, 0, 0, 0x2e, 0xfb];
        assert_eq!(PropertyValue::read(input).unwrap(),
                   PropertyValue::I2(-1234));

        let input: &[u8] = &[3, 0, 0, 0, 0x15, 0xcd, 0x5b, 0x07];
        assert_eq!(PropertyValue::read(input).unwrap(),
                   PropertyValue::I4(123456789));

        let input: &[u8] = &[30, 0, 0, 0, 14, 0, 0, 0, b'H', b'e', b'l',
                             b'l', b'o', b',', b' ', b'w', b'o', b'r', b'l',
                             b'd', b'!', 0];
        assert_eq!(PropertyValue::read(input).unwrap(),
                   PropertyValue::LpStr("Hello, world!".to_string()));

        let input: &[u8] = &[64, 0, 0, 0, 0x80, 0x68, 0xd8, 0x8f, 0xec, 0x71,
                             0xd1, 0x01];
        assert_eq!(PropertyValue::read(input).unwrap(),
                   PropertyValue::FileTime(131011125010000000));
    }

    #[test]
    fn write_property_value() {
        let value = PropertyValue::Empty;
        let mut output = Vec::<u8>::new();
        value.write(&mut output).unwrap();
        assert_eq!(&output as &[u8], &[0, 0, 0, 0]);

        let value = PropertyValue::Null;
        let mut output = Vec::<u8>::new();
        value.write(&mut output).unwrap();
        assert_eq!(&output as &[u8], &[1, 0, 0, 0]);

        let value = PropertyValue::I2(-1234);
        let mut output = Vec::<u8>::new();
        value.write(&mut output).unwrap();
        assert_eq!(&output as &[u8], &[2, 0, 0, 0, 0x2e, 0xfb, 0, 0]);

        let value = PropertyValue::I4(123456789);
        let mut output = Vec::<u8>::new();
        value.write(&mut output).unwrap();
        assert_eq!(&output as &[u8], &[3, 0, 0, 0, 0x15, 0xcd, 0x5b, 0x07]);

        let value = PropertyValue::LpStr("Hello, world!".to_string());
        let mut output = Vec::<u8>::new();
        value.write(&mut output).unwrap();
        assert_eq!(&output as &[u8],
                   &[30, 0, 0, 0, 14, 0, 0, 0, b'H', b'e', b'l', b'l', b'o',
                     b',', b' ', b'w', b'o', b'r', b'l', b'd', b'!', 0, 0, 0]);

        let value = PropertyValue::FileTime(131011125010000000);
        let mut output = Vec::<u8>::new();
        value.write(&mut output).unwrap();
        assert_eq!(&output as &[u8],
                   &[64, 0, 0, 0, 0x80, 0x68, 0xd8, 0x8f, 0xec, 0x71, 0xd1,
                     0x01]);
    }

    #[test]
    fn property_value_round_trip() {
        let values = &[PropertyValue::Empty,
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
                       PropertyValue::FileTime(123456789123456789)];
        for value in values.iter() {
            let mut output = Vec::<u8>::new();
            value.write(&mut output).unwrap();
            let parsed = PropertyValue::read(&output as &[u8]).unwrap();
            assert_eq!(parsed, *value);
            let expected_size = value.size_including_padding();
            assert_eq!(output.len() as u32, expected_size);
            assert_eq!(expected_size % 4, 0);
        }
    }

    #[test]
    fn read_property_set() {
        let input: &[u8] = &[
            0xfe, 0xff,  // Byte order mark
            1, 0,        // Format version
            10, 0, 2, 0, // OS
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // CLSID
            1, 0, 0, 0,  // Reserved
            1, 2, 3, 4, 5, 6, 7, 8, 1, 2, 3, 4, 5, 6, 7, 8, // FMTID
            48, 0, 0, 0, // Section offset
            48, 0, 0, 0, // Section size
            2, 0, 0, 0,  // Number of properties
            4, 0, 0, 0,  // 1st property name
            24, 0, 0, 0, // 1st property offset
            37, 0, 0, 0, // 2nd property name
            40, 0, 0, 0, // 2nd property offset
            30, 0, 0, 0, // 1st property type (LPSTR)
            8, 0, 0, 0,  // String length (including terminating null)
            b'F', b'o', b'o', b' ', b'B', b'a', b'r', 0,  // String value
            16, 0, 0, 0, // 2nd property type (I1)
            253, 0, 0, 0, // Int value (and padding)
        ];
        let property_set = PropertySet::read(Cursor::new(input)).unwrap();
        assert_eq!(property_set.format_identifier(),
                   &[1, 2, 3, 4, 5, 6, 7, 8, 1, 2, 3, 4, 5, 6, 7, 8]);
        assert_eq!(property_set.get(4),
                   Some(&PropertyValue::LpStr("Foo Bar".to_string())));
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
            0xfe, 0xff,  // Byte order mark
            1, 0,        // Format version
            10, 0, 2, 0, // OS
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // CLSID
            1, 0, 0, 0,  // Reserved
            1, 2, 3, 4, 5, 6, 7, 8, 1, 2, 3, 4, 5, 6, 7, 8, // FMTID
            48, 0, 0, 0, // Section offset
            48, 0, 0, 0, // Section size
            2, 0, 0, 0,  // Number of properties
            4, 0, 0, 0,  // 1st property name
            24, 0, 0, 0, // 1st property offset
            37, 0, 0, 0, // 2nd property name
            40, 0, 0, 0, // 2nd property offset
            30, 0, 0, 0, // 1st property type (LPSTR)
            8, 0, 0, 0,  // String length (including terminating null)
            b'F', b'o', b'o', b' ', b'B', b'a', b'r', 0,  // String value
            16, 0, 0, 0, // 2nd property type (I1)
            253, 0, 0, 0, // Int value (and padding)
        ];
        assert_eq!(&output as &[u8], expected);
    }
}

// ========================================================================= //
