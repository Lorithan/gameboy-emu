use crate::cpu::{CPU, CpuError, CycleAction, Registers, opcode::DecodedOpcode, registers::Flag};
const CASES: [(u8, bool, usize); 2] = [(0xF8, true, 3), (0xE8, false, 4)];
fn machine(sp: u16, flags: u8, pc: u16) -> CPU {
    let mut r = Registers::dmg_post_boot(1);
    r.set_af(0x5A00 + u16::from(flags));
    r.set_bc(0x1234);
    r.set_de(0x5678);
    r.set_hl(0x9ABC);
    r.sp = sp;
    r.pc = pc;
    CPU::from_registers(r)
}
fn finish(cpu: &mut CPU, opcode: u8, raw: u8, cycles: usize) {
    assert!(!cpu.complete_cycle(Some(opcode)).unwrap());
    assert!(!cpu.complete_cycle(Some(raw)).unwrap());
    for cycle in 2..cycles {
        assert!(matches!(cpu.request_cycle(), Ok(CycleAction::Idle)));
        assert_eq!(cpu.complete_cycle(None).unwrap(), cycle + 1 == cycles);
    }
}
#[test]
fn signed_sp_opcode_mappings_are_explicit() {
    assert_eq!(DecodedOpcode::from(0xF8), DecodedOpcode::LoadHlFromSpOffset);
    assert_eq!(
        DecodedOpcode::from(0xE8),
        DecodedOpcode::AddSignedOffsetToSp
    );
}
#[test]
fn signed_sp_operand_read_advances_pc_and_next_fetch_skips_operand() {
    for (opcode, _, cycles) in CASES {
        for pc in [0x0100u16, 0xFFFE, 0xFFFF] {
            let mut cpu = machine(0x1000, 0xF0, pc);
            assert!(!cpu.complete_cycle(Some(opcode)).unwrap());
            assert_eq!(cpu.snapshot().pc, pc.wrapping_add(1));
            assert!(!cpu.complete_cycle(Some(0)).unwrap());
            assert_eq!(
                cpu.snapshot().pc,
                pc.wrapping_add(2),
                "successful operand read must advance PC"
            );
            for cycle in 2..cycles {
                assert_eq!(cpu.complete_cycle(None).unwrap(), cycle + 1 == cycles);
            }
            assert!(
                matches!(cpu.request_cycle(), Ok(CycleAction::Read { addr }) if addr == pc.wrapping_add(2))
            );
            assert!(cpu.complete_cycle(Some(0)).unwrap());
            assert_eq!(cpu.snapshot().pc, pc.wrapping_add(3));
        }
    }
}
#[test]
fn signed_sp_half_carry_uses_only_low_nibbles() {
    for (opcode, _, cycles) in CASES {
        for (sp, raw, half) in [
            (0x0010u16, 0u8, false),
            (0, 0x10, false),
            (0x000F, 1, true),
            (0x0000, 0xFF, false),
            (0x0001, 0xFF, true),
        ] {
            let mut cpu = machine(sp, 0, 0x0100);
            finish(&mut cpu, opcode, raw, cycles);
            assert_eq!(
                cpu.registers.get_flag(Flag::H),
                half,
                "opcode {opcode:02X} SP={sp:04X} raw={raw:02X}"
            );
        }
    }
}
#[test]
fn signed_sp_carry_uses_low_byte_overflow_not_nibble_or_word() {
    for (opcode, _, cycles) in CASES {
        for (sp, raw, carry) in [
            (0x000Fu16, 1u8, false),
            (0, 0xFF, false),
            (0x00FF, 1, true),
            (0xFFFF, 0, false),
            (0x0001, 0xFF, true),
        ] {
            let mut cpu = machine(sp, 0, 0x0100);
            finish(&mut cpu, opcode, raw, cycles);
            assert_eq!(
                cpu.registers.get_flag(Flag::C),
                carry,
                "opcode {opcode:02X} SP={sp:04X} raw={raw:02X}"
            );
        }
    }
}
#[test]
fn signed_sp_all_operands_match_signed_result_and_unsigned_flag_oracles() {
    for (opcode, hl, cycles) in CASES {
        for sp in 0u16..=65535 {
            for raw in 0u8..=255 {
                let mut cpu = machine(sp, ((sp % 16) * 16) as u8, 0x0100);
                let before = cpu.snapshot();
                finish(&mut cpu, opcode, raw, cycles);
                let signed = if raw < 128 {
                    i32::from(raw)
                } else {
                    i32::from(raw) - 256
                };
                let result = (i32::from(sp) + signed).rem_euclid(65536) as u16;
                let h = u32::from(sp % 16) + u32::from(raw % 16) >= 16;
                let c = u32::from(sp % 256) + u32::from(raw) >= 256;
                let after = cpu.snapshot();
                assert_eq!(if hl { after.hl } else { after.sp }, result);
                assert_eq!(
                    after.af,
                    (before.af & 0xFF00) + if h { 0x20 } else { 0 } + if c { 0x10 } else { 0 },
                    "opcode {opcode:02X} SP={sp:04X} raw={raw:02X}"
                );
                assert_eq!((after.bc, after.de), (before.bc, before.de));
                assert_eq!(
                    if hl { after.sp } else { after.hl },
                    if hl { before.sp } else { before.hl }
                );
            }
        }
    }
}
#[test]
fn signed_sp_retries_are_atomic_and_original_sp_survives_all_idle_cycles() {
    for (opcode, hl, cycles) in CASES {
        let mut cpu = machine(0x1001, 0xF0, 0x0100);
        let before = cpu.snapshot();
        assert!(matches!(
            cpu.complete_cycle(None),
            Err(CpuError::MissingReadValue { addr: 0x0100 })
        ));
        assert_eq!(cpu.snapshot(), before);
        assert!(!cpu.complete_cycle(Some(opcode)).unwrap());
        let pending = cpu.snapshot();
        for _ in 0..2 {
            assert!(matches!(
                cpu.request_cycle(),
                Ok(CycleAction::Read { addr: 0x0101 })
            ));
            assert!(matches!(
                cpu.complete_cycle(None),
                Err(CpuError::MissingReadValue { addr: 0x0101 })
            ));
            assert_eq!(cpu.snapshot(), pending);
        }
        cpu.registers.sp = 0x4321;
        assert!(!cpu.complete_cycle(Some(0xFF)).unwrap());
        let ready = cpu.snapshot();
        for cycle in 2..cycles {
            for _ in 0..2 {
                assert!(matches!(cpu.request_cycle(), Ok(CycleAction::Idle)));
                assert_eq!(cpu.snapshot(), ready);
            }
            assert_eq!(cpu.complete_cycle(None).unwrap(), cycle + 1 == cycles);
            if cycle + 1 < cycles {
                assert_eq!(cpu.snapshot(), ready);
            }
        }
        let after = cpu.snapshot();
        assert_eq!(if hl { after.hl } else { after.sp }, 0x1000);
        assert_eq!(after.af, 0x5A30);
        assert_eq!(
            if hl { after.sp } else { after.hl },
            if hl { 0x4321 } else { before.hl }
        );
    }
}

#[test]
fn signed_sp_boundary_inputs_overwrite_all_initial_flags() {
    for (opcode, hl, cycles) in CASES {
        for sp in [0u16, 1, 0x000F, 0x0010, 0x00FF, 0x0100, 0x7FFF, 0xFFFF] {
            for raw in [0u8, 1, 0x7F, 0x80, 0xFF] {
                for flags in (0u8..=0xF0).step_by(16) {
                    let mut cpu = machine(sp, flags, 0x0100);
                    let before = cpu.snapshot();
                    finish(&mut cpu, opcode, raw, cycles);
                    let signed = if raw < 128 {
                        i32::from(raw)
                    } else {
                        i32::from(raw) - 256
                    };
                    let value = (i32::from(sp) + signed).rem_euclid(65536) as u16;
                    let mut expected = before;
                    expected.pc += 2;
                    if hl {
                        expected.hl = value;
                    } else {
                        expected.sp = value;
                    }
                    expected.af = (before.af & 0xFF00)
                        + if u32::from(sp % 16) + u32::from(raw % 16) >= 16 {
                            0x20
                        } else {
                            0
                        }
                        + if u32::from(sp % 256) + u32::from(raw) >= 256 {
                            0x10
                        } else {
                            0
                        };
                    assert_eq!(cpu.snapshot(), expected);
                }
            }
        }
    }
}
