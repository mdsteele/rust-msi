use cfb::CompoundFile;
use msi::{Package, PackageType};
use std::io::{self, Cursor, Write};

//===========================================================================//

#[test]
fn remove_signature_from_unsigned_package() -> io::Result<()> {
    // Create a new package.  It should be unsigned initially.
    let cursor = Cursor::new(Vec::new());
    let mut package = Package::create(PackageType::Installer, cursor)?;
    assert!(!package.has_digital_signature());

    // Trying to remove a signature when there already isn't one should have no
    // effect.
    package.remove_digital_signature()?;
    assert!(!package.has_digital_signature());
    Ok(())
}

#[test]
fn remove_signature_from_signed_package() -> io::Result<()> {
    // Create a new package and edit it to add a (bogus) digital signature.
    let cursor = Cursor::new(Vec::new());
    let package = Package::create(PackageType::Installer, cursor)?;
    let cursor = package.into_inner()?;
    let mut comp = CompoundFile::open(cursor)?;
    comp.create_stream("\u{5}DigitalSignature")?.write(b"foo")?;
    comp.create_stream("\u{5}MsiDigitalSignatureEx")?.write(b"bar")?;

    // Open the package again.  It should now have a signature.  However, the
    // signature data should not show up in the list of MSI streams.
    let cursor = comp.into_inner();
    let mut package = Package::open(cursor)?;
    assert!(package.has_digital_signature());
    assert_eq!(
        package.streams().collect::<Vec<String>>(),
        Vec::<String>::new()
    );

    // Remove the signature.
    package.remove_digital_signature()?;
    assert!(!package.has_digital_signature());

    // Check that the signature data really is gone from the underlying CFB
    // file.
    let cursor = package.into_inner()?;
    let comp = CompoundFile::open(cursor)?;
    assert!(!comp.exists("\u{5}DigitalSignature"));
    assert!(!comp.exists("\u{5}MsiDigitalSignatureEx"));
    Ok(())
}

//===========================================================================//
