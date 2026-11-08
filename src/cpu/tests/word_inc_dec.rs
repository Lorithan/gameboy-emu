use super::{CPU, CpuError, CycleAction, Registers};
use crate::cpu::CpuSnapShot;
use crate::cpu::{
    opcode::{DecodedOpcode, UnaryOperation},
    registers::Register16,
};

const CASES: [(u8, Register16, bool); 8] = [
    (0x03, Register16::BC, true),
    (0x0B, Register16::BC, false),
    (0x13, Register16::DE, true),
    (0x1B, Register16::DE, false),
    (0x23, Register16::HL, true),
    (0x2B, Register16::HL, false),
    (0x33, Register16::SP, true),
    (0x3B, Register16::SP, false),
];

fn set(r: &mut Registers, target: Register16, value: u16) {
    match target {
        Register16::BC => r.set_bc(value),
        Register16::DE => r.set_de(value),
        Register16::HL => r.set_hl(value),
        Register16::SP => r.set_sp(value),
    }
}

fn expected(mut snapshot: CpuSnapShot, target: Register16, value: u16) -> CpuSnapShot {
    match target {
        Register16::BC => snapshot.bc = value,
        Register16::DE => snapshot.de = value,
        Register16::HL => snapshot.hl = value,
        Register16::SP => snapshot.sp = value,
    }
    snapshot
}

fn machine(target: Register16, value: u16, flags: u8, pc: u16) -> CPU {
    let mut r = Registers::dmg_post_boot(1);
    r.set_af(0x5A00 + u16::from(flags));
    r.set_bc(0x1234);
    r.set_de(0x5678);
    r.set_hl(0x9ABC);
    r.sp = 0xDEF0;
    r.pc = pc;
    set(&mut r, target, value);
    CPU::from_registers(r)
}

fn result(value: u16, increment: bool) -> u16 {
    (i32::from(value) + if increment { 1 } else { -1 }).rem_euclid(65536) as u16
}

#[test]
fn word_inc_dec_mappings_are_explicit() {
    for (opcode, target, increment) in CASES {
        assert_eq!(
            DecodedOpcode::from(opcode),
            DecodedOpcode::Unary16 {
                target,
                operation: if increment {
                    UnaryOperation::Increment
                } else {
                    UnaryOperation::Decrement
                }
            }
        );
    }
}

#[test]
fn word_inc_dec_exhaustive_values_commit_only_on_idle_and_preserve_flags() {
    for (opcode, target, increment) in CASES {
        for value in 0u16..=65535 {
            let mut cpu = machine(target, value, ((value % 16) * 16) as u8, 0x0100);
            let mut pending = cpu.snapshot();
            pending.pc += 1;
            assert!(!cpu.complete_cycle(Some(opcode)).unwrap());
            assert_eq!(cpu.snapshot(), pending);
            assert!(matches!(cpu.request_cycle(), Ok(CycleAction::Idle)));
            assert!(cpu.complete_cycle(None).unwrap());
            assert_eq!(
                cpu.snapshot(),
                expected(pending, target, result(value, increment)),
                "opcode {opcode:02X} value {value:04X}"
            );
        }
    }
}

#[test]
fn word_inc_dec_capture_values_through_requests_retries_and_pc_wrap() {
    for (opcode, target, increment) in CASES {
        for value in [0, 1, 0x000F, 0x00FF, 0x0100, 0x0FFF, 0x7FFF, 0xFFFE, 0xFFFF] {
            for flags in (0u8..=0xF0).step_by(16) {
                let mut cpu = machine(target, value, flags, 0xFFFF);
                let before = cpu.snapshot();
                assert!(matches!(
                    cpu.complete_cycle(None),
                    Err(CpuError::MissingReadValue { addr: 0xFFFF })
                ));
                assert_eq!(cpu.snapshot(), before);
                assert!(!cpu.complete_cycle(Some(opcode)).unwrap());
                let mut pending = before;
                pending.pc = 0;
                assert_eq!(cpu.snapshot(), pending);
                // Test-only mutation verifies the pending cycle owns its operand.
                set(&mut cpu.registers, target, 0x4321);
                let changed = cpu.snapshot();
                for _ in 0..3 {
                    assert!(matches!(cpu.request_cycle(), Ok(CycleAction::Idle)));
                    assert_eq!(cpu.snapshot(), changed);
                }
                assert!(cpu.complete_cycle(None).unwrap());
                assert_eq!(
                    cpu.snapshot(),
                    expected(pending, target, result(value, increment))
                );
            }
        }
    }
}

#[test]
fn word_inc_dec_completion_resumes_opcode_fetch_and_runs_inverse() {
    for (opcode, target, increment) in CASES {
        let mut cpu = machine(target, 0xFFFF, 0xF0, 0x0100);
        let before = cpu.snapshot();
        assert!(!cpu.complete_cycle(Some(opcode)).unwrap());
        assert!(cpu.complete_cycle(None).unwrap());
        assert!(
            matches!(cpu.request_cycle(), Ok(CycleAction::Read { addr: 0x0101 })),
            "opcode {opcode:02X} must leave its idle state after completion"
        );
        let inverse = if increment { opcode + 8 } else { opcode - 8 };
        assert!(!cpu.complete_cycle(Some(inverse)).unwrap());
        assert!(cpu.complete_cycle(None).unwrap());
        let mut expected = before;
        expected.pc += 2;
        assert_eq!(cpu.snapshot(), expected);
        assert!(cpu.complete_cycle(Some(0x00)).unwrap());
        expected.pc += 1;
        assert_eq!(cpu.snapshot(), expected);
    }
}
