use super::{Flag, Flags, Registers};
use crate::cpu::CPU;

fn explicit_registers() -> Registers {
    Registers {
        a: 0x42,
        b: 0x12,
        c: 0x34,
        d: 0x56,
        e: 0x78,
        f: Flags::new(0xA0),
        h: 0x9A,
        l: 0xBC,
        pc: 0x2345,
        sp: 0xCDEF,
    }
}

#[test]
fn register_pairs_round_trip_and_preserve_other_state() {
    let mut registers = explicit_registers();
    let original_a = registers.a;
    let original_pc = registers.pc;
    let original_sp = registers.sp;
    for value in 0u16..=u16::MAX {
        registers.set_bc(value);
        registers.set_de(value ^ 0x55AA);
        registers.set_hl(!value);
        assert_eq!(registers.bc(), value);
        assert_eq!(registers.b, (value >> 8) as u8);
        assert_eq!(registers.c, value as u8);
        assert_eq!(registers.de(), value ^ 0x55AA);
        assert_eq!(registers.d, ((value ^ 0x55AA) >> 8) as u8);
        assert_eq!(registers.e, (value ^ 0x55AA) as u8);
        assert_eq!(registers.hl(), !value);
        assert_eq!(registers.h, ((!value) >> 8) as u8);
        assert_eq!(registers.l, (!value) as u8);
    }
    assert_eq!(registers.a, original_a);
    assert_eq!(registers.pc, original_pc);
    assert_eq!(registers.sp, original_sp);
}

#[test]
fn byte_writes_update_the_correct_half_of_each_pair() {
    let mut registers = explicit_registers();
    registers.set_bc(0x1234);
    registers.set_de(0x5678);
    registers.set_hl(0x9ABC);

    registers.b = 0xAB;
    assert_eq!(registers.bc(), 0xAB34);
    registers.c = 0xCD;
    assert_eq!(registers.bc(), 0xABCD);
    registers.d = 0xDE;
    assert_eq!(registers.de(), 0xDE78);
    registers.e = 0xEF;
    assert_eq!(registers.de(), 0xDEEF);
    registers.h = 0x12;
    assert_eq!(registers.hl(), 0x12BC);
    registers.l = 0x34;
    assert_eq!(registers.hl(), 0x1234);
    assert_eq!(registers.bc(), 0xABCD);
    assert_eq!(registers.de(), 0xDEEF);
}

#[test]
fn af_writes_discard_the_unused_flag_bits() {
    let mut registers = explicit_registers();
    for value in 0u16..=u16::MAX {
        registers.set_af(value);
        assert_eq!(registers.af(), value & 0xFFF0, "AF input {value:#06X}");
    }
}

#[test]
fn flag_getters_decode_all_valid_combinations() {
    for combination in 0u8..16 {
        let raw = combination << 4;
        let flags = Flags::new(raw);
        assert_eq!(flags.get(Flag::Z), raw & 0x80 != 0);
        assert_eq!(flags.get(Flag::N), raw & 0x40 != 0);
        assert_eq!(flags.get(Flag::H), raw & 0x20 != 0);
        assert_eq!(flags.get(Flag::C), raw & 0x10 != 0);
    }
}

#[test]
fn flag_setters_preserve_other_flags_for_all_valid_combinations() {
    let selectors = [Flag::Z, Flag::N, Flag::H, Flag::C];
    for combination in 0u8..16 {
        let raw = combination << 4;
        for (selector, mask) in selectors.into_iter().zip([0x80, 0x40, 0x20, 0x10]) {
            for enabled in [false, true] {
                let mut flags = Flags::new(raw);
                flags.set(selector, enabled);
                let expected = if enabled { raw | mask } else { raw & !mask };
                assert_eq!(flags.0, expected);
                assert_eq!(flags.0 & 0x0F, 0);
            }
        }
    }
}

#[test]
fn flag_constructor_masks_unused_bits_for_every_byte() {
    for raw in 0u8..=u8::MAX {
        let flags = Flags::new(raw);
        assert_eq!(flags.0, raw & 0xF0, "input {raw:#04X}");
        assert_eq!(flags.get(Flag::Z), raw & 0x80 != 0);
        assert_eq!(flags.get(Flag::N), raw & 0x40 != 0);
        assert_eq!(flags.get(Flag::H), raw & 0x20 != 0);
        assert_eq!(flags.get(Flag::C), raw & 0x10 != 0);
    }
}

#[test]
fn dmg_post_boot_state_matches_the_profile_for_every_checksum() {
    for checksum in 0u8..=u8::MAX {
        let registers = Registers::dmg_post_boot(checksum);
        let expected_af = if checksum == 0 { 0x0180 } else { 0x01B0 };
        assert_eq!(registers.af(), expected_af, "checksum {checksum:#04X}");
        assert_eq!(registers.bc(), 0x0013);
        assert_eq!(registers.de(), 0x00D8);
        assert_eq!(registers.hl(), 0x014D);
        assert_eq!(registers.pc, 0x0100);
        assert_eq!(registers.sp, 0xFFFE);
    }
}

#[test]
fn cpu_construction_preserves_explicit_state_without_applying_a_boot_profile() {
    let input = explicit_registers();
    let cpu = CPU::from_registers(input);
    let snapshot = cpu.snapshot();
    assert_eq!(snapshot.af, input.af());
    assert_eq!(snapshot.bc, input.bc());
    assert_eq!(snapshot.de, input.de());
    assert_eq!(snapshot.hl, input.hl());
    assert_eq!(snapshot.pc, input.pc);
    assert_eq!(snapshot.sp, input.sp);
}

#[test]
fn explicit_cpu_snapshot_is_owned_and_does_not_change_live_registers() {
    let cpu = CPU::from_registers(explicit_registers());
    let original = cpu.snapshot();
    let mut changed = original;
    changed.af = 0;
    changed.bc = 0;
    changed.de = 0;
    changed.hl = 0;
    changed.pc = 0;
    changed.sp = 0;
    assert_ne!(changed, original);
    assert_eq!(cpu.snapshot(), original);
}

#[test]
fn register_flag_getters_decode_all_valid_combinations() {
    for combination in 0u8..16 {
        let raw = combination << 4;
        let mut registers = explicit_registers();
        registers.set_af(0x4200 | u16::from(raw));
        assert_eq!(registers.get_flag(Flag::Z), raw & 0x80 != 0);
        assert_eq!(registers.get_flag(Flag::N), raw & 0x40 != 0);
        assert_eq!(registers.get_flag(Flag::H), raw & 0x20 != 0);
        assert_eq!(registers.get_flag(Flag::C), raw & 0x10 != 0);
    }
}

#[test]
fn register_flag_setters_preserve_other_flags_and_registers() {
    let selectors: [fn() -> Flag; 4] = [|| Flag::Z, || Flag::N, || Flag::H, || Flag::C];
    for combination in 0u8..16 {
        let raw = combination << 4;
        for (selector, mask) in selectors.into_iter().zip([0x80, 0x40, 0x20, 0x10]) {
            for enabled in [false, true] {
                let mut registers = explicit_registers();
                registers.set_af(0x4200 | u16::from(raw));
                registers.set_flag(selector(), enabled);
                assert_eq!(registers.get_flag(selector()), enabled);
                let expected = if enabled { raw | mask } else { raw & !mask };
                assert_eq!(registers.af(), 0x4200 | u16::from(expected));
                assert_eq!(registers.bc(), 0x1234);
                assert_eq!(registers.de(), 0x5678);
                assert_eq!(registers.hl(), 0x9ABC);
                assert_eq!(registers.pc, 0x2345);
                assert_eq!(registers.sp, 0xCDEF);
            }
        }
    }
}
