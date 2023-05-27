use crate::internal::codepage::CodePage;
use crate::internal::language::Language;
use crate::internal::propset::{OperatingSystem, PropertySet, PropertyValue};
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
const PROPERTY_TEMPLATE: u32 = 7;
const PROPERTY_UUID: u32 = 9;
const PROPERTY_CREATION_TIME: u32 = 12;
const PROPERTY_WORD_COUNT: u32 = 15;
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
        let mut summary = SummaryInfo { properties };
        summary.set_codepage(CodePage::Utf8);
        summary
    }

    pub(crate) fn read<R: Read + Seek>(reader: R) -> io::Result<SummaryInfo> {
        let properties = PropertySet::read(reader)?;
        if properties.format_identifier() != &FMTID {
            invalid_data!("Property set has wrong format identifier");
        }
        Ok(SummaryInfo { properties })
    }

    pub(crate) fn write<W: Write>(&self, writer: W) -> io::Result<()> {
        self.properties.write(writer)
    }

    /// Gets the architecture string from the "template" property, if one is
    /// set.  This indicates the hardware architecture that this package is
    /// intended for (e.g. `"x64"`).
    pub fn arch(&self) -> Option<&str> {
        match self.properties.get(PROPERTY_TEMPLATE) {
            Some(PropertyValue::LpStr(template)) => {
                let arch =
                    template.split_once(';').map_or(&**template, |x| x.0);
                if arch.is_empty() {
                    None
                } else {
                    Some(arch)
                }
            }
            _ => None,
        }
    }

    /// Sets the architecture string in the "template" property.
    pub fn set_arch<S: Into<String>>(&mut self, arch: S) {
        let langs = match self.properties.get(PROPERTY_TEMPLATE) {
            Some(PropertyValue::LpStr(template)) => {
                let parts: Vec<&str> = template.splitn(2, ';').collect();
                if parts.len() > 1 {
                    parts[1].to_string()
                } else {
                    String::new()
                }
            }
            _ => String::new(),
        };
        let template = format!("{};{}", arch.into(), langs);
        self.properties.set(PROPERTY_TEMPLATE, PropertyValue::LpStr(template));
    }

    /// Clears the architecture string in the "template" property.
    pub fn clear_arch(&mut self) {
        self.set_arch("");
    }

    /// Gets the "author" property, if one is set.  This indicates the name of
    /// the person or company that created the package.
    pub fn author(&self) -> Option<&str> {
        match self.properties.get(PROPERTY_AUTHOR) {
            Some(PropertyValue::LpStr(author)) => Some(author.as_str()),
            _ => None,
        }
    }

    /// Sets the "author" property.
    pub fn set_author<S: Into<String>>(&mut self, author: S) {
        self.properties
            .set(PROPERTY_AUTHOR, PropertyValue::LpStr(author.into()));
    }

    /// Clears the "author" property.
    pub fn clear_author(&mut self) {
        self.properties.remove(PROPERTY_AUTHOR);
    }

    /// Gets the code page used for serializing this summary info.
    pub fn codepage(&self) -> CodePage {
        self.properties.codepage()
    }

    /// Sets the code page used for serializing this summary info.
    pub fn set_codepage(&mut self, codepage: CodePage) {
        self.properties.set_codepage(codepage);
    }

    /// Gets the "comments" property, if one is set.  This typically gives a
    /// brief description of the application/software that will be installed by
    /// the package.
    pub fn comments(&self) -> Option<&str> {
        match self.properties.get(PROPERTY_COMMENTS) {
            Some(PropertyValue::LpStr(comments)) => Some(comments.as_str()),
            _ => None,
        }
    }

    /// Sets the "comments" property.
    pub fn set_comments<S: Into<String>>(&mut self, comments: S) {
        self.properties
            .set(PROPERTY_COMMENTS, PropertyValue::LpStr(comments.into()));
    }

    /// Clears the "comments" property.
    pub fn clear_comments(&mut self) {
        self.properties.remove(PROPERTY_COMMENTS);
    }

    /// Gets the "creating application" property, if one is set.  This
    /// indicates the name of the software application/tool that was used to
    /// create the package.
    pub fn creating_application(&self) -> Option<&str> {
        match self.properties.get(PROPERTY_CREATING_APP) {
            Some(PropertyValue::LpStr(app_name)) => Some(app_name.as_str()),
            _ => None,
        }
    }

    /// Sets the "creating application" property.
    pub fn set_creating_application<S: Into<String>>(&mut self, app_name: S) {
        self.properties
            .set(PROPERTY_CREATING_APP, PropertyValue::LpStr(app_name.into()));
    }

    /// Clears the "creating application" property.
    pub fn clear_creating_application(&mut self) {
        self.properties.remove(PROPERTY_CREATING_APP);
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

    /// Clears the "creation time" property.
    pub fn clear_creation_time(&mut self) {
        self.properties.remove(PROPERTY_CREATION_TIME);
    }

    /// Gets the list of languages from the "template" property, if one is set.
    /// This indicates the languages that this package supports.
    pub fn languages(&self) -> Vec<Language> {
        match self.properties.get(PROPERTY_TEMPLATE) {
            Some(PropertyValue::LpStr(template)) => {
                let parts: Vec<&str> = template.splitn(2, ';').collect();
                if parts.len() > 1 {
                    parts[1]
                        .split(',')
                        .filter_map(|code| code.parse().ok())
                        .map(Language::from_code)
                        .collect()
                } else {
                    Vec::new()
                }
            }
            _ => Vec::new(),
        }
    }

    /// Sets the list of languages in the "template" property.
    pub fn set_languages(&mut self, languages: &[Language]) {
        let mut template = match self.properties.get(PROPERTY_TEMPLATE) {
            Some(PropertyValue::LpStr(template)) => template
                .split_once(';')
                .map_or(&**template, |x| x.0)
                .to_string(),
            _ => String::new(),
        };
        template.push(';');
        let mut first = true;
        for language in languages.iter() {
            if first {
                first = false;
            } else {
                template.push(',');
            }
            template.push_str(&format!("{}", language.code()));
        }
        self.properties.set(PROPERTY_TEMPLATE, PropertyValue::LpStr(template));
    }

    /// Clears the list of languages in the "template" property.
    pub fn clear_languages(&mut self) {
        self.set_languages(&[]);
    }

    /// Gets the "subject" property, if one is set.  This typically indicates
    /// the name of the application/software that will be installed by the
    /// package.
    pub fn subject(&self) -> Option<&str> {
        match self.properties.get(PROPERTY_SUBJECT) {
            Some(PropertyValue::LpStr(subject)) => Some(subject.as_str()),
            _ => None,
        }
    }

    /// Sets the "subject" property.
    pub fn set_subject<S: Into<String>>(&mut self, subject: S) {
        self.properties
            .set(PROPERTY_SUBJECT, PropertyValue::LpStr(subject.into()));
    }

    /// Clears the "subject" property.
    pub fn clear_subject(&mut self) {
        self.properties.remove(PROPERTY_SUBJECT);
    }

    /// Gets the "title" property, if one is set.  This indicates the type of
    /// the installer package (e.g. "Installation Database" or "Patch").
    pub fn title(&self) -> Option<&str> {
        match self.properties.get(PROPERTY_TITLE) {
            Some(PropertyValue::LpStr(title)) => Some(title.as_str()),
            _ => None,
        }
    }

    /// Sets the "title" property.
    pub fn set_title<S: Into<String>>(&mut self, title: S) {
        self.properties
            .set(PROPERTY_TITLE, PropertyValue::LpStr(title.into()));
    }

    /// Clears the "title" property.
    pub fn clear_title(&mut self) {
        self.properties.remove(PROPERTY_TITLE);
    }

    /// Gets the "UUID" property, if one is set.
    pub fn uuid(&self) -> Option<Uuid> {
        match self.properties.get(PROPERTY_UUID) {
            Some(PropertyValue::LpStr(string)) => {
                let trimmed =
                    string.trim_start_matches('{').trim_end_matches('}');
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

    /// Clears the "UUID" property.
    pub fn clear_uuid(&mut self) {
        self.properties.remove(PROPERTY_UUID);
    }

    /// Gets the "Word Count" property, if one is set.
    pub fn word_count(&self) -> Option<i32> {
        match self.properties.get(PROPERTY_WORD_COUNT) {
            Some(PropertyValue::I4(word_count)) => Some(*word_count),
            _ => None,
        }
    }

    /// Sets the "Word Count" property.
    pub fn set_word_count(&mut self, word_count: i32) {
        self.properties
            .set(PROPERTY_WORD_COUNT, PropertyValue::I4(word_count));
    }

    /// Clears the "Word Count" property.
    pub fn clear_word_count(&mut self) {
        self.properties.remove(PROPERTY_WORD_COUNT);
    }
}

// ========================================================================= //

#[cfg(test)]
mod tests {
    use super::SummaryInfo;
    use crate::internal::language::Language;
    use std::time::SystemTime;
    use uuid::Uuid;

    #[test]
    fn set_properties() {
        let languages = vec![
            Language::from_tag("en-CA"),
            Language::from_tag("fr-CA"),
            Language::from_tag("en-US"),
            Language::from_tag("es-MX"),
        ];
        let timestamp = SystemTime::now();
        let uuid =
            Uuid::parse_str("0000002a-000c-0005-0c03-0938362b0809").unwrap();

        let mut summary_info = SummaryInfo::new();
        summary_info.set_arch("x64");
        summary_info.set_author("Jane Doe");
        summary_info.set_comments("This app is the greatest!");
        summary_info.set_creating_application("cargo-test");
        summary_info.set_creation_time(timestamp);
        summary_info.set_languages(&languages);
        summary_info.set_subject("My Great App");
        summary_info.set_title("Installation Package");
        summary_info.set_uuid(uuid);
        summary_info.set_word_count(2);

        assert_eq!(summary_info.arch(), Some("x64"));
        assert_eq!(summary_info.author(), Some("Jane Doe"));
        assert_eq!(summary_info.comments(), Some("This app is the greatest!"));
        assert_eq!(summary_info.creating_application(), Some("cargo-test"));
        assert_eq!(summary_info.creation_time(), Some(timestamp));
        assert_eq!(summary_info.languages(), languages);
        assert_eq!(summary_info.subject(), Some("My Great App"));
        assert_eq!(summary_info.title(), Some("Installation Package"));
        assert_eq!(summary_info.uuid(), Some(uuid));
        assert_eq!(summary_info.word_count(), Some(2));

        summary_info.clear_arch();
        assert_eq!(summary_info.arch(), None);
        summary_info.clear_author();
        assert_eq!(summary_info.author(), None);
        summary_info.clear_comments();
        assert_eq!(summary_info.comments(), None);
        summary_info.clear_creating_application();
        assert_eq!(summary_info.creating_application(), None);
        summary_info.clear_creation_time();
        assert_eq!(summary_info.creation_time(), None);
        summary_info.clear_languages();
        assert_eq!(summary_info.languages(), Vec::new());
        summary_info.clear_subject();
        assert_eq!(summary_info.subject(), None);
        summary_info.clear_title();
        assert_eq!(summary_info.title(), None);
        summary_info.clear_uuid();
        assert_eq!(summary_info.uuid(), None);
        summary_info.clear_word_count();
        assert_eq!(summary_info.word_count(), None);
    }

    #[test]
    fn template_property() {
        // Set language before setting arch:
        let mut summary_info = SummaryInfo::new();
        assert_eq!(summary_info.arch(), None);
        summary_info.set_languages(&[Language::from_tag("en")]);
        assert_eq!(summary_info.arch(), None);
        assert_eq!(summary_info.languages(), vec![Language::from_tag("en")]);
        summary_info.set_arch("Intel");
        assert_eq!(summary_info.arch(), Some("Intel"));
        assert_eq!(summary_info.languages(), vec![Language::from_tag("en")]);

        // Set arch before setting language:
        let mut summary_info = SummaryInfo::new();
        assert_eq!(summary_info.languages(), vec![]);
        assert_eq!(summary_info.arch(), None);
        summary_info.set_arch("Intel");
        assert_eq!(summary_info.languages(), vec![]);
        assert_eq!(summary_info.arch(), Some("Intel"));
        summary_info.set_languages(&[Language::from_tag("en")]);
        assert_eq!(summary_info.languages(), vec![Language::from_tag("en")]);
        assert_eq!(summary_info.arch(), Some("Intel"));
    }
}

// ========================================================================= //
