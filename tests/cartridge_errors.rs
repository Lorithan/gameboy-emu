use std::{error::Error, io};

use gameboy_emu::memory::Cartridge;

#[test]
fn public_cartridge_loader_errors_support_standard_reporting_and_io_downcasting() {
    let error = Cartridge::from_bytes(&[0]).unwrap_err();
    let boxed: Box<dyn Error> = error.into();
    assert!(boxed.source().is_none());
    assert!(!boxed.to_string().is_empty());

    let path = std::env::temp_dir().join(format!(
        "gameboy-emu-missing-{}-{}.gb",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    assert!(!path.exists());
    let error = Cartridge::from_filepath(&path).unwrap_err();
    let boxed: Box<dyn Error> = error.into();
    assert!(boxed.to_string().contains(&path.display().to_string()));
    assert_eq!(
        boxed
            .source()
            .unwrap()
            .downcast_ref::<io::Error>()
            .unwrap()
            .kind(),
        io::ErrorKind::NotFound
    );
}
