use msi::{CodePage, Package, PackageType};
use std::io::{Cursor, Write};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use uuid::Uuid;

// ========================================================================= //

#[test]
fn set_summary_info_properties() {
    let sat_2017_mar_18_at_18_46_36_gmt =
        UNIX_EPOCH + Duration::from_secs(1489862796);
    let uuid =
        Uuid::parse_str("9bb29b0d-edc7-4699-9607-a5e201d67ed1").unwrap();

    let cursor = Cursor::new(Vec::new());
    let mut package = Package::create(PackageType::Installer, cursor).unwrap();
    {
        let summary_info = package.summary_info_mut();
        summary_info.set_author("Jane Doe".to_string());
        summary_info.set_comments("This app is the greatest!".to_string());
        summary_info.set_creating_application("cargo-test".to_string());
        summary_info.set_creation_time(sat_2017_mar_18_at_18_46_36_gmt);
        summary_info.set_subject("My Great App".to_string());
        summary_info.set_title("Awesome Package".to_string());
        summary_info.set_uuid(uuid);
    }

    let cursor = package.into_inner().unwrap();
    let package = Package::open(cursor).unwrap();
    let summary_info = package.summary_info();
    assert_eq!(summary_info.author(), Some("Jane Doe"));
    assert_eq!(summary_info.comments(), Some("This app is the greatest!"));
    assert_eq!(summary_info.creating_application(), Some("cargo-test"));
    assert_eq!(
        summary_info.creation_time(),
        Some(sat_2017_mar_18_at_18_46_36_gmt)
    );
    assert_eq!(summary_info.subject(), Some("My Great App"));
    assert_eq!(summary_info.title(), Some("Awesome Package"));
    assert_eq!(summary_info.uuid(), Some(uuid));
}

#[test]
fn set_summary_info_codepage() {
    let cursor = Cursor::new(Vec::new());
    let mut package = Package::create(PackageType::Installer, cursor).unwrap();
    package.summary_info_mut().set_codepage(CodePage::Utf8);
    package.summary_info_mut().set_author("Snowman=\u{2603}".to_string());

    let cursor = package.into_inner().unwrap();
    let mut package = Package::open(cursor).unwrap();
    assert_eq!(package.summary_info().codepage(), CodePage::Utf8);
    assert_eq!(package.summary_info().author(), Some("Snowman=\u{2603}"));
    package.summary_info_mut().set_codepage(CodePage::Windows1252);

    let cursor = package.into_inner().unwrap();
    let package = Package::open(cursor).unwrap();
    assert_eq!(package.summary_info().codepage(), CodePage::Windows1252);
    assert_eq!(package.summary_info().author(), Some("Snowman=?"));
}

#[test]
fn dropping_package_persists_changes() {
    let mut cursor = Cursor::new(Vec::new());
    {
        let mut package =
            Package::create(PackageType::Installer, cursor.by_ref()).unwrap();
        assert_eq!(package.summary_info().comments(), None);
        package.summary_info_mut().set_comments("Hello, world!".to_string());
    }
    let package = Package::open(cursor).unwrap();
    assert_eq!(package.summary_info().comments(), Some("Hello, world!"));
}

#[test]
fn set_creation_time_to_now() {
    let timestamp = SystemTime::now();

    let cursor = Cursor::new(Vec::new());
    let mut package = Package::create(PackageType::Installer, cursor).unwrap();
    package.summary_info_mut().set_creation_time_to_now();

    let cursor = package.into_inner().unwrap();
    let package = Package::open(cursor).unwrap();
    assert!(package.summary_info().creation_time().is_some());
    assert!(package.summary_info().creation_time().unwrap() > timestamp);
}

// ========================================================================= //
