use internal::codepage::CodePage;
use internal::propset::{OperatingSystem, PropertySet, PropertyValue};
use std::io::{self, Read, Seek, Write};
use std::time::SystemTime;
use uuid::Uuid;

// ========================================================================= //

// This constant is this UUID:
//     F29F85E0-4FF9-1068-AB91-08002B27B3D9
// Which comes from this page:
//     https://msdn.microsoft.com/en-us/library/windows/desktop/
//     aa380052(v=vs.85).aspx
// The first three fields are in little-endian, and the last two in big-endian,
// because that's how Windows encodes UUIDs.  For details, see:
//     https://en.wikipedia.org/wiki/Universally_unique_identifier#Encoding
const FMTID: [u8; 16] =
    *b"\xe0\x85\x9f\xf2\xf9\x4f\x68\x10\xab\x91\x08\x00\x2b\x27\xb3\xd9";

const PROPERTY_TITLE: u32 = 2;
const PROPERTY_SUBJECT: u32 = 3;
const PROPERTY_AUTHOR: u32 = 4;
const PROPERTY_COMMENTS: u32 = 6;
const PROPERTY_UUID: u32 = 9;
const PROPERTY_CREATION_TIME: u32 = 12;
const PROPERTY_CREATING_APP: u32 = 18;

// ========================================================================= //

/// Summary information (e.g. title, author) about an MSI package.
pub struct SummaryInfo {
    properties: PropertySet,
}

impl SummaryInfo {
    /// Creates an empty `SummaryInfo` with no properties set.
    pub(crate) fn new() -> SummaryInfo {
        let properties = PropertySet::new(OperatingSystem::Win32, 10, FMTID);
        let mut summary = SummaryInfo { properties: properties };
        summary.set_codepage(CodePage::Utf8);
        summary
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

    /// Gets the "author" property, if one is set.  This indicates the name of
    /// the person or company that created the package.
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

    /// Gets the code page used for serializing this summary info.
    pub fn codepage(&self) -> CodePage { self.properties.codepage() }

    /// Sets the code page used for serializing this summary info.
    pub fn set_codepage(&mut self, codepage: CodePage) {
        self.properties.set_codepage(codepage);
    }

    /// Gets the "comments" property, if one is set.  This typically gives a
    /// brief description of the application/software that will be installed by
    /// the package.
    pub fn comments(&self) -> Option<&str> {
        match self.properties.get(PROPERTY_COMMENTS) {
            Some(&PropertyValue::LpStr(ref comments)) => {
                Some(comments.as_str())
            }
            _ => None,
        }
    }

    /// Sets the "comments" property.
    pub fn set_comments(&mut self, comments: String) {
        self.properties.set(PROPERTY_COMMENTS, PropertyValue::LpStr(comments));
    }

    /// Gets the "creating application" property, if one is set.  This
    /// indicates the name of the software application/tool that was used to
    /// create the package.
    pub fn creating_application(&self) -> Option<&str> {
        match self.properties.get(PROPERTY_CREATING_APP) {
            Some(&PropertyValue::LpStr(ref app_name)) => {
                Some(app_name.as_str())
            }
            _ => None,
        }
    }

    /// Sets the "creating application" property.
    pub fn set_creating_application(&mut self, app_name: String) {
        self.properties
            .set(PROPERTY_CREATING_APP, PropertyValue::LpStr(app_name));
    }

    /// Gets the "creation time" property, if one is set.  This indicates the
    /// date/time when the package was created.
    pub fn creation_time(&self) -> Option<SystemTime> {
        match self.properties.get(PROPERTY_CREATION_TIME) {
            Some(&PropertyValue::FileTime(timestamp)) => Some(timestamp),
            _ => None,
        }
    }

    /// Sets the "creation time" property.
    pub fn set_creation_time(&mut self, timestamp: SystemTime) {
        self.properties
            .set(PROPERTY_CREATION_TIME, PropertyValue::FileTime(timestamp));
    }

    /// Sets the "creation time" property to the current time.
    pub fn set_creation_time_to_now(&mut self) {
        self.set_creation_time(SystemTime::now());
    }

    /// Gets the "subject" property, if one is set.  This typically indicates
    /// the name of the application/software that will be installed by the
    /// package.
    pub fn subject(&self) -> Option<&str> {
        match self.properties.get(PROPERTY_SUBJECT) {
            Some(&PropertyValue::LpStr(ref subject)) => Some(subject.as_str()),
            _ => None,
        }
    }

    /// Sets the "subject" property.
    pub fn set_subject(&mut self, subject: String) {
        self.properties.set(PROPERTY_SUBJECT, PropertyValue::LpStr(subject));
    }

    /// Gets the "title" property, if one is set.  This indicates the type of
    /// the installer package (e.g. "Installation Database" or "Patch").
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

    /// Gets the "UUID" property, if one is set.
    pub fn uuid(&self) -> Option<Uuid> {
        match self.properties.get(PROPERTY_UUID) {
            Some(&PropertyValue::LpStr(ref string)) => {
                let trimmed =
                    string.trim_left_matches('{').trim_right_matches('}');
                Uuid::parse_str(trimmed).ok()
            }
            _ => None,
        }
    }

    /// Sets the "UUID" property.
    pub fn set_uuid(&mut self, uuid: Uuid) {
        let mut string = format!("{{{}}}", uuid.hyphenated());
        string.make_ascii_uppercase();
        self.properties.set(PROPERTY_UUID, PropertyValue::LpStr(string));
    }
}

// ========================================================================= //

#[cfg(test)]
mod tests {
    use super::SummaryInfo;
    use std::time::SystemTime;
    use uuid::Uuid;

    #[test]
    fn set_properties() {
        let timestamp = SystemTime::now();
        let uuid = Uuid::parse_str("0000002a-000c-0005-0c03-0938362b0809")
            .unwrap();

        let mut summary_info = SummaryInfo::new();
        summary_info.set_author("Jane Doe".to_string());
        summary_info.set_comments("This app is the greatest!".to_string());
        summary_info.set_creating_application("cargo-test".to_string());
        summary_info.set_creation_time(timestamp);
        summary_info.set_subject("My Great App".to_string());
        summary_info.set_title("Installation Package".to_string());
        summary_info.set_uuid(uuid);

        assert_eq!(summary_info.author(), Some("Jane Doe"));
        assert_eq!(summary_info.comments(), Some("This app is the greatest!"));
        assert_eq!(summary_info.creating_application(), Some("cargo-test"));
        assert_eq!(summary_info.creation_time(), Some(timestamp));
        assert_eq!(summary_info.subject(), Some("My Great App"));
        assert_eq!(summary_info.title(), Some("Installation Package"));
        assert_eq!(summary_info.uuid(), Some(uuid));
    }
}

// ========================================================================= //
