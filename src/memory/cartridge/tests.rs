use super::{Cartridge, CartridgeError};
use super::{CartridgeKind, CgbSupport, Header, RamSize, RomSize};
use std::{
    fs, io,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

// Synthetic, non-bootable inputs; no game or boot ROM is required.
const ROM_LENGTH: usize = 32_768;

fn valid_rom() -> Vec<u8> {
    let mut rom = vec![0; ROM_LENGTH];
    rom[0x014D] = 0xE7; // Independently known: -25 modulo 256.
    rom
}

// Fixture preparation uses the sum formula, not the production subtraction loop.
fn refresh_checksum(rom: &mut [u8]) {
    let sum: i32 = rom[0x0134..=0x014C]
        .iter()
        .map(|&byte| i32::from(byte))
        .sum();
    rom[0x014D] = (-sum - 25).rem_euclid(256) as u8;
}

#[test]
fn accepts_zero_header_with_known_checksum_and_preserves_every_byte() {
    let rom = valid_rom();
    let cartridge = Cartridge::from_bytes(&rom).expect("valid synthetic ROM");
    assert_eq!(cartridge.rom(), rom.as_slice());
}

#[test]
fn preserves_rom_region_boundaries() {
    let mut rom = valid_rom();
    for (offset, value) in [(0, 0x12), (0x3FFF, 0x34), (0x4000, 0x56), (0x7FFF, 0x78)] {
        rom[offset] = value;
    }
    let cartridge = Cartridge::from_bytes(&rom).expect("valid marked ROM");
    assert_eq!(cartridge.rom(), rom.as_slice());
}

#[test]
fn distinguishes_incomplete_headers_without_panicking() {
    for length in [0, 1, 0x014E, 0x014F] {
        let mut rom = valid_rom();
        rom.resize(length, 0);
        let error = Cartridge::from_bytes(&rom).expect_err("incomplete header must fail");
        assert!(
            matches!(error, CartridgeError::IncompleteHeader { minimum: 0x0150, actual } if actual == length),
            "wrong diagnostic for length {length}: {error:?}"
        );
    }
}

#[test]
fn rejects_wrong_rom_lengths_after_a_complete_header_without_panicking() {
    for length in [0x0150, 32_767, 32_769, 65_536] {
        let mut rom = valid_rom();
        rom.resize(length, 0);
        let error = Cartridge::from_bytes(&rom).expect_err("wrong length must fail");
        assert!(
            matches!(error, CartridgeError::LengthError { expected: ROM_LENGTH, actual } if actual == length),
            "wrong diagnostic for length {length}: {error:?}"
        );
    }
}

#[test]
fn public_rom_accessor_exposes_all_bytes_including_header() {
    let mut rom = valid_rom();
    rom[0] = 0x12;
    rom[0x7FFF] = 0x34;
    let cartridge = Cartridge::from_bytes(&rom).expect("valid synthetic ROM");
    let exposed: &[u8] = cartridge.rom();
    assert_eq!(exposed.len(), ROM_LENGTH);
    assert_eq!(exposed, rom.as_slice());
    assert_eq!(exposed[0x014D], 0xE7);
}

#[test]
fn a_zero_header_is_a_32_kib_rom_only_dmg_cartridge() {
    let cartridge = Cartridge::from_bytes(&valid_rom()).expect("valid synthetic ROM");
    let header = cartridge.header();
    assert_eq!(header.kind, CartridgeKind::RomOnly);
    assert_eq!(
        header.rom_size.bytes(),
        ROM_LENGTH,
        "code 0 means 32 KiB, not zero"
    );
    assert_eq!(header.rom_size.banks(), 2);
    assert_eq!(header.cgb, CgbSupport::DmgOnly);
    assert_eq!(header.title, None);
}

#[test]
fn accepts_dmg_compatible_color_header_with_known_checksum() {
    let mut rom = valid_rom();
    rom[0x0143] = 0x80;
    rom[0x014D] = 0x67;
    let cartridge = Cartridge::from_bytes(&rom).expect("DMG-compatible header");
    assert_eq!(cartridge.rom(), rom.as_slice());
}

#[test]
fn rejects_color_only_header() {
    let mut rom = valid_rom();
    rom[0x0143] = 0xC0;
    rom[0x014D] = 0x27;
    assert!(matches!(
        Cartridge::from_bytes(&rom),
        Err(CartridgeError::GBCUnsupported)
    ));
}

#[test]
fn accepts_legacy_title_byte_in_color_flag_position() {
    let mut rom = valid_rom();
    rom[0x0143] = b'A';
    refresh_checksum(&mut rom);
    assert!(Cartridge::from_bytes(&rom).is_ok());
}

#[test]
fn rejects_incorrect_stored_checksum_with_expected_and_actual_values() {
    let mut rom = valid_rom();
    rom[0x014D] = 0xE6;
    assert!(matches!(
        Cartridge::from_bytes(&rom),
        Err(CartridgeError::ChecksumFailure {
            expected: 0xE7,
            actual: 0xE6
        })
    ));
}

#[test]
fn checksum_includes_both_ends_of_the_header_range() {
    for address in [0x0134, 0x014C] {
        let mut rom = valid_rom();
        rom[address] = 1;
        assert!(matches!(
            Cartridge::from_bytes(&rom),
            Err(CartridgeError::ChecksumFailure {
                expected: 0xE6,
                actual: 0xE7
            })
        ));
        rom[0x014D] = 0xE6;
        assert!(Cartridge::from_bytes(&rom).is_ok(), "offset {address:#06X}");
    }
}

#[test]
fn checksum_wraps_with_nonzero_header_bytes() {
    let mut rom = valid_rom();
    rom[0x0134] = 0xFF;
    rom[0x0135] = 0xFF;
    rom[0x014D] = 0xE9; // -(255 + 255) - 25 modulo 256.
    assert!(Cartridge::from_bytes(&rom).is_ok());
}

#[test]
fn logo_and_global_checksum_are_not_loader_validation_requirements() {
    let mut rom = valid_rom();
    rom[0x0104..=0x0133].fill(0xA5);
    rom[0x014E] = 0x12;
    rom[0x014F] = 0x34;
    let cartridge = Cartridge::from_bytes(&rom).expect("ignored fields must not reject input");
    assert_eq!(cartridge.rom(), rom.as_slice());
}

struct ScratchDirectory(PathBuf);

impl ScratchDirectory {
    fn create() -> Self {
        static NEXT_ID: AtomicU64 = AtomicU64::new(0);
        loop {
            let path = std::env::temp_dir().join(format!(
                "gameboy-emu-cartridge-tests-{}-{}",
                std::process::id(),
                NEXT_ID.fetch_add(1, Ordering::Relaxed)
            ));
            match fs::create_dir(&path) {
                Ok(()) => return Self(path),
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(error) => panic!("cannot create scratch directory: {error}"),
            }
        }
    }
}

impl Drop for ScratchDirectory {
    fn drop(&mut self) {
        // This directory was created exclusively by this test helper.
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn raw_file_loading_matches_in_memory_loading() {
    let scratch = ScratchDirectory::create();
    let path = scratch.0.join("synthetic.gb");
    let rom = valid_rom();
    fs::write(&path, &rom).expect("write fixture");
    let from_file = Cartridge::from_filepath(&path).expect("read fixture");
    let from_memory = Cartridge::from_bytes(&rom).expect("parse fixture");
    assert_eq!(from_file.rom(), from_memory.rom());
}

#[test]
fn missing_file_retains_path_and_io_cause() {
    let scratch = ScratchDirectory::create();
    let path = scratch.0.join("does-not-exist.gb");
    match Cartridge::from_filepath(&path).expect_err("missing file must fail") {
        CartridgeError::FileIO { err, path: actual } => {
            assert_eq!(actual, path);
            assert_eq!(err.kind(), io::ErrorKind::NotFound);
        }
        error => panic!("wrong error: {error:?}"),
    }
}

#[test]
fn directory_input_retains_path_and_io_cause() {
    let scratch = ScratchDirectory::create();
    // Windows may reject opening a directory; Unix may reject reading it.
    match Cartridge::from_filepath(&scratch.0).expect_err("directory is not a ROM file") {
        CartridgeError::FileIO { err, path } => {
            assert_eq!(path, scratch.0);
            assert_ne!(err.kind(), io::ErrorKind::NotFound);
        }
        error => panic!("wrong error: {error:?}"),
    }
}

#[test]
fn invalid_file_content_reports_validation_failure() {
    let scratch = ScratchDirectory::create();
    let path = scratch.0.join("bad-checksum.gb");
    let mut rom = valid_rom();
    rom[0x014D] = 0xE6;
    fs::write(&path, rom).expect("write invalid fixture");
    assert!(matches!(
        Cartridge::from_filepath(&path),
        Err(CartridgeError::ChecksumFailure {
            expected: 0xE7,
            actual: 0xE6
        })
    ));
}
#[test]
fn cartridge_validation_errors_are_standard_leaves_with_useful_metadata() {
    use std::error::Error;
    for (error, details) in [
        (
            CartridgeError::IncompleteHeader {
                minimum: 336,
                actual: 100,
            },
            vec!["336", "100"],
        ),
        (
            CartridgeError::LengthError {
                expected: 32768,
                actual: 32767,
            },
            vec!["32768", "32767"],
        ),
        (
            CartridgeError::UnsupportedCartridgeType { code: 0x19 },
            vec!["19"],
        ),
        (
            CartridgeError::UnsupportedRomSize { code: 0x52 },
            vec!["52"],
        ),
        (
            CartridgeError::UnsupportedRamSize { code: 0x01 },
            vec!["01"],
        ),
        (
            CartridgeError::InvalidHeaderCombination {
                cartridge_type: 0x03,
                rom_size: 0x05,
                ram_size: 0x03,
            },
            vec!["03", "05"],
        ),
        (CartridgeError::GBCUnsupported, vec![]),
    ] {
        let standard: &dyn Error = &error;
        assert!(standard.source().is_none());
        let message = standard.to_string();
        assert!(!message.is_empty());
        for detail in details {
            assert!(message.contains(detail), "missing {detail}: {message}");
        }
    }
    let error = CartridgeError::ChecksumFailure {
        expected: 0xE7,
        actual: 0xE6,
    };
    assert!(error.source().is_none());
    let message = error.to_string().to_ascii_lowercase();
    assert!(
        message.contains("231") || message.contains("e7"),
        "missing expected checksum: {message}"
    );
    assert!(
        message.contains("230") || message.contains("e6"),
        "missing actual checksum: {message}"
    );
}

#[test]
fn cartridge_file_error_preserves_path_and_typed_io_source_without_duplicate_cause_text() {
    use std::error::Error;
    let path = PathBuf::from("ROM folder/missing-夜.gb");
    let error = CartridgeError::FileIO {
        path: path.clone(),
        err: io::Error::new(io::ErrorKind::NotFound, "synthetic missing cartridge"),
    };
    let source = error
        .source()
        .expect("original I/O cause must be inspectable");
    let original = source.downcast_ref::<io::Error>().unwrap();
    assert_eq!(original.kind(), io::ErrorKind::NotFound);
    assert_eq!(original.to_string(), "synthetic missing cartridge");
    let message = error.to_string();
    assert!(message.contains(&path.display().to_string()), "{message}");
    assert!(
        !message.contains(&original.to_string()),
        "cause should be rendered through source: {message}"
    );
}

// --- C1 (#90): headers for ROM-only and MBC1 cartridges ---------------------
// Rules: docs/tasks/MBC1.md, "Accepted cartridges" and "Header".

/// A header-only image (0x150 bytes) with the given type, ROM and RAM codes
/// and a correct checksum.
fn header_image(cartridge_type: u8, rom_size: u8, ram_size: u8) -> Vec<u8> {
    let mut image = vec![0; 0x0150];
    image[0x0147] = cartridge_type;
    image[0x0148] = rom_size;
    image[0x0149] = ram_size;
    refresh_checksum(&mut image);
    image
}

/// A whole cartridge of the declared size. The first and last byte of every
/// 16 KiB bank is marked, so lost or moved banks show up.
fn full_image(cartridge_type: u8, rom_size: u8, ram_size: u8) -> Vec<u8> {
    let length = 0x8000usize << rom_size;
    let mut image = vec![0; length];
    for bank in 0..length / 0x4000 {
        image[bank * 0x4000 + 0x3FFF] = bank as u8 ^ 0xA5;
        if bank > 0 {
            image[bank * 0x4000] = bank as u8;
        }
    }
    image[0x0147] = cartridge_type;
    image[0x0148] = rom_size;
    image[0x0149] = ram_size;
    refresh_checksum(&mut image);
    image
}

#[derive(Debug, PartialEq)]
enum Expected {
    Kind(CartridgeKind),
    Combination,
}

/// Independent model of the accepted table, for valid codes only.
fn expected_kind(cartridge_type: u8, rom_size: u8, ram_size: u8) -> Expected {
    let ram = match ram_size {
        0x00 => RamSize::None,
        0x02 => RamSize::Kib8,
        0x03 => RamSize::Kib32,
        _ => unreachable!(),
    };
    let large_rom = rom_size >= 0x05; // 1 MiB or more
    match (cartridge_type, ram) {
        (0x00, RamSize::None) if rom_size == 0x00 => Expected::Kind(CartridgeKind::RomOnly),
        (0x01, RamSize::None) => Expected::Kind(CartridgeKind::Mbc1 {
            ram: RamSize::None,
            battery: false,
        }),
        (0x02 | 0x03, RamSize::Kib8) => Expected::Kind(CartridgeKind::Mbc1 {
            ram,
            battery: cartridge_type == 0x03,
        }),
        (0x02 | 0x03, RamSize::Kib32) if !large_rom => Expected::Kind(CartridgeKind::Mbc1 {
            ram,
            battery: cartridge_type == 0x03,
        }),
        _ => Expected::Combination,
    }
}

#[test]
fn every_rom_size_code_decodes_or_is_rejected_with_its_code() {
    for code in 0..=u8::MAX {
        match (code, RomSize::from_code(code)) {
            (0x00..=0x06, Ok(size)) => {
                assert_eq!(size.banks(), 2 << code, "code {code:#04X}");
                assert_eq!(size.bytes(), 0x8000 << code, "code {code:#04X}");
            }
            (0x00..=0x06, Err(error)) => panic!("code {code:#04X} rejected: {error:?}"),
            (_, Ok(size)) => panic!("code {code:#04X} accepted as {size:?}"),
            (_, Err(error)) => assert!(
                matches!(error, CartridgeError::UnsupportedRomSize { code: c } if c == code),
                "code {code:#04X}: {error:?}"
            ),
        }
    }
    assert_eq!(RomSize::from_code(0x06).unwrap().bytes(), 2 * 1024 * 1024);
}

#[test]
fn size_ordering_agrees_with_byte_counts() {
    // The derived order follows declaration order; it must mean "smaller".
    let roms: Vec<RomSize> = (0x00..=0x06)
        .map(|code| RomSize::from_code(code).unwrap())
        .collect();
    let rams: Vec<RamSize> = [0x00, 0x02, 0x03]
        .into_iter()
        .map(|code| RamSize::from_code(code).unwrap())
        .collect();
    for a in &roms {
        for b in &roms {
            assert_eq!(a.cmp(b), a.bytes().cmp(&b.bytes()), "{a:?} vs {b:?}");
        }
    }
    for a in &rams {
        for b in &rams {
            assert_eq!(a.cmp(b), a.bytes().cmp(&b.bytes()), "{a:?} vs {b:?}");
        }
    }
}

#[test]
fn every_ram_size_code_decodes_or_is_rejected_with_its_code() {
    for code in 0..=u8::MAX {
        let expected = match code {
            0x00 => Some((RamSize::None, 0)),
            0x02 => Some((RamSize::Kib8, 8 * 1024)),
            0x03 => Some((RamSize::Kib32, 32 * 1024)),
            // 01 (the unofficial 2 KiB), 04, 05 and the rest (decision 2).
            _ => None,
        };
        match (expected, RamSize::from_code(code)) {
            (Some((size, bytes)), Ok(actual)) => {
                assert_eq!(actual, size, "code {code:#04X}");
                assert_eq!(actual.bytes(), bytes, "code {code:#04X}");
            }
            (None, Err(error)) => assert!(
                matches!(error, CartridgeError::UnsupportedRamSize { code: c } if c == code),
                "code {code:#04X}: {error:?}"
            ),
            (expected, actual) => {
                panic!("code {code:#04X}: expected {expected:?}, got {actual:?}")
            }
        }
    }
}

#[test]
fn unsupported_cartridge_types_are_rejected_with_their_code() {
    for code in 0x04..=u8::MAX {
        let error = Header::parse(&header_image(code, 0x00, 0x00)).expect_err("unsupported type");
        assert!(
            matches!(error, CartridgeError::UnsupportedCartridgeType { code: c } if c == code),
            "type {code:#04X}: {error:?}"
        );
    }
}

#[test]
fn unsupported_size_codes_are_rejected_by_the_header() {
    for code in [0x07, 0x08, 0x52, 0x53, 0x54, 0xFF] {
        let error = Header::parse(&header_image(0x01, code, 0x00)).expect_err("ROM size");
        assert!(
            matches!(error, CartridgeError::UnsupportedRomSize { code: c } if c == code),
            "ROM code {code:#04X}: {error:?}"
        );
    }
    for code in [0x01, 0x04, 0x05, 0xFF] {
        let error = Header::parse(&header_image(0x03, 0x00, code)).expect_err("RAM size");
        assert!(
            matches!(error, CartridgeError::UnsupportedRamSize { code: c } if c == code),
            "RAM code {code:#04X}: {error:?}"
        );
    }
}

#[test]
fn every_type_size_combination_is_accepted_or_rejected_as_a_combination() {
    let mut accepted = 0;
    for cartridge_type in 0x00..=0x03 {
        for rom_size in 0x00..=0x06 {
            for ram_size in [0x00, 0x02, 0x03] {
                let context =
                    format!("type {cartridge_type:02X}, ROM {rom_size:02X}, RAM {ram_size:02X}");
                let parsed = Header::parse(&header_image(cartridge_type, rom_size, ram_size));
                match (expected_kind(cartridge_type, rom_size, ram_size), parsed) {
                    (Expected::Kind(kind), Ok(header)) => {
                        assert_eq!(header.kind, kind, "{context}");
                        assert_eq!(
                            header.rom_size,
                            RomSize::from_code(rom_size).unwrap(),
                            "{context}"
                        );
                        accepted += 1;
                    }
                    (Expected::Combination, Err(error)) => assert!(
                        matches!(
                            error,
                            CartridgeError::InvalidHeaderCombination {
                                cartridge_type: t,
                                rom_size: r,
                                ram_size: m
                            } if (t, r, m) == (cartridge_type, rom_size, ram_size)
                        ),
                        "{context}: {error:?}"
                    ),
                    (expected, actual) => {
                        panic!("{context}: expected {expected:?}, got {actual:?}")
                    }
                }
            }
        }
    }
    // ROM-only 1; MBC1 7; MBC1+RAM and +BATTERY 7 (8 KiB) + 5 (32 KiB) each.
    assert_eq!(accepted, 1 + 7 + 2 * (7 + 5));
}

#[test]
fn a_cartridge_must_be_exactly_its_declared_size() {
    for rom_size in 0x00..=0x06u8 {
        let cartridge_type = if rom_size == 0 { 0x00 } else { 0x01 };
        let image = full_image(cartridge_type, rom_size, 0x00);
        let expected = 0x8000 << rom_size;
        assert_eq!(image.len(), expected);
        let cartridge = Cartridge::from_bytes(&image).expect("exact size");
        assert_eq!(
            cartridge.rom(),
            image.as_slice(),
            "ROM code {rom_size:02X}: bytes kept"
        );
        assert_eq!(cartridge.header().rom_size.bytes(), expected);
        for length in [
            0x0150,
            expected - 1,
            expected + 1,
            expected * 2,
            expected / 2,
        ] {
            let mut wrong = image.clone();
            wrong.resize(length, 0);
            let error = Cartridge::from_bytes(&wrong).expect_err("wrong size");
            assert!(
                matches!(error, CartridgeError::LengthError { expected: e, actual: a } if e == expected && a == length),
                "ROM code {rom_size:02X}, length {length}: {error:?}"
            );
        }
    }
}

#[test]
fn header_parse_needs_only_the_header_and_agrees_with_the_cartridge() {
    for length in [0, 0x014F] {
        let error = Header::parse(&vec![0; length]).expect_err("short");
        assert!(
            matches!(error, CartridgeError::IncompleteHeader { minimum: 0x0150, actual } if actual == length),
            "length {length}: {error:?}"
        );
    }
    let image = full_image(0x03, 0x04, 0x03);
    let header = Header::parse(&image[..0x0150]).expect("header alone");
    assert_eq!(
        header.kind,
        CartridgeKind::Mbc1 {
            ram: RamSize::Kib32,
            battery: true
        }
    );
    let cartridge = Cartridge::from_bytes(&image).expect("whole cartridge");
    assert_eq!(cartridge.header(), &header);
    // The checksum still guards every header field.
    let mut bad = image[..0x0150].to_vec();
    bad[0x0149] = 0x02;
    assert!(matches!(
        Header::parse(&bad),
        Err(CartridgeError::ChecksumFailure { .. })
    ));
}

#[test]
fn the_cgb_flag_is_decoded_and_cgb_only_is_rejected() {
    for (flag, expected) in [
        (0x00, Some(CgbSupport::DmgOnly)),
        (b'A', Some(CgbSupport::DmgOnly)),
        (0x7F, Some(CgbSupport::DmgOnly)),
        (0x80, Some(CgbSupport::CgbCompatible)),
        (0xC0, None),
    ] {
        let mut image = header_image(0x01, 0x00, 0x00);
        image[0x0143] = flag;
        refresh_checksum(&mut image);
        match (expected, Header::parse(&image)) {
            (Some(cgb), Ok(header)) => assert_eq!(header.cgb, cgb, "flag {flag:#04X}"),
            (None, Err(CartridgeError::GBCUnsupported)) => {}
            (expected, actual) => {
                panic!("flag {flag:#04X}: expected {expected:?}, got {actual:?}")
            }
        }
    }
}

/// Parse a ROM-only header whose title area 0134-0143 holds `bytes`.
fn title_of(bytes: &[u8]) -> Option<String> {
    assert!(bytes.len() <= 16);
    let mut image = header_image(0x00, 0x00, 0x00);
    image[0x0134..0x0134 + bytes.len()].copy_from_slice(bytes);
    refresh_checksum(&mut image);
    Header::parse(&image).expect("valid header").title
}

#[test]
fn titles_are_printable_ascii_up_to_the_padding() {
    assert_eq!(title_of(b""), None, "all padding");
    assert_eq!(title_of(b"TETRIS"), Some("TETRIS".into()));
    assert_eq!(title_of(b"MAX PIRATE"), Some("MAX PIRATE".into()));
    assert_eq!(
        title_of(b"SIXTEEN CHARS OK"),
        Some("SIXTEEN CHARS OK".into()),
        "16 characters when 0143 is not a CGB flag"
    );
    assert_eq!(
        title_of(b"AB\0CD"),
        Some("AB".into()),
        "stops at the first 00"
    );
    assert_eq!(
        title_of(b"ZELDA   "),
        Some("ZELDA".into()),
        "trailing spaces"
    );
    assert_eq!(title_of(b"     "), None, "only spaces");
    assert_eq!(title_of(b"Lower Case ok"), Some("Lower Case ok".into()));
}

#[test]
fn a_cgb_flag_ends_the_title_at_15_characters() {
    let mut image = header_image(0x00, 0x00, 0x00);
    image[0x0134..=0x0142].copy_from_slice(b"FIFTEEN CHARS!!");
    image[0x0143] = 0x80;
    refresh_checksum(&mut image);
    let header = Header::parse(&image).expect("CGB-compatible");
    assert_eq!(header.title.as_deref(), Some("FIFTEEN CHARS!!"));
    // A short title padded before a manufacturer code.
    let mut image = header_image(0x00, 0x00, 0x00);
    image[0x0134..0x0134 + 8].copy_from_slice(b"POKEMON ");
    image[0x013F..=0x0142].copy_from_slice(b"AAUE");
    image[0x0143] = 0x80;
    refresh_checksum(&mut image);
    assert_eq!(
        Header::parse(&image).unwrap().title.as_deref(),
        Some("POKEMON"),
        "padding ends the title before the manufacturer code"
    );
}

#[test]
fn a_title_with_unprintable_bytes_is_no_title() {
    for bad in [&b"AB\x01CD"[..], b"\x7F", b"CAF\xC9", b"\x1BESC"] {
        assert_eq!(title_of(bad), None, "{bad:?}");
    }
    // Unprintable bytes after the padding do not matter.
    assert_eq!(title_of(b"OK\0\x01\xFF"), Some("OK".into()));
}
