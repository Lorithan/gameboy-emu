use crate::cartridge::Cartridge;

#[test]
fn metadata_is_public_and_mutating_the_snapshot_does_not_change_the_cartridge() {
    let mut rom = vec![0; 32_768];
    rom[0x014D] = 0xE7;
    let cartridge = Cartridge::from_bytes(&rom).expect("valid synthetic ROM");

    // This sibling module can only access the cartridge's public API.
    let mut metadata = cartridge.metadata();
    assert_eq!(metadata.cartridge_type_code, 0);
    assert_eq!(metadata.rom_size, 32_768);
    assert_eq!(metadata.ram_size, 0);

    metadata.cartridge_type_code = 0xFF;
    metadata.rom_size = 1;
    metadata.ram_size = 8_192;
    assert_eq!(metadata.cartridge_type_code, 0xFF);
    assert_eq!(metadata.rom_size, 1);
    assert_eq!(metadata.ram_size, 8_192);

    let unchanged = cartridge.metadata();
    assert_eq!(unchanged.cartridge_type_code, 0);
    assert_eq!(unchanged.rom_size, 32_768);
    assert_eq!(unchanged.ram_size, 0);
    assert_eq!(cartridge.rom(), rom.as_slice());
}
