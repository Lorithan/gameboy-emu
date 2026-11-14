use gameboy_emu::{
    cpu::{CpuSnapShot, CycleAction},
    memory::Cartridge,
    system::{InstructionStep, System},
};
#[path = "support/cb.rs"]
mod oracle;

fn program(bytes: &[u8]) -> System {
    let mut rom = vec![0; 32768];
    rom[0x014D] = 0xE7;
    rom[0x100..0x100 + bytes.len()].copy_from_slice(bytes);
    System::new(Cartridge::from_bytes(&rom).unwrap())
}
fn complete(system: &mut System, cycles: u64) {
    assert!(matches!(system.step_instruction(100).unwrap(),
        InstructionStep::Complete { mcycles, ticks } if mcycles == cycles && ticks == cycles * 4));
}
fn cycles(opcode: u8) -> u64 {
    if opcode % 8 != 6 {
        2
    } else if (0x40..0x80).contains(&opcode) {
        3
    } else {
        4
    }
}
fn setup(opcode: u8, value: u8, flags: u8) -> System {
    // PUSH BC / POP AF sets exact flags through the public CPU/Bus interface.
    let load = [0x06, 0x0E, 0x16, 0x1E, 0x26, 0x2E, 0x36, 0x3E][usize::from(opcode % 8)];
    let mut system = program(&[
        0x01, flags, 0x55, 0xC5, 0xF1, 0x21, 0x23, 0xC1, load, value, 0xCB, opcode, 0, 0xD3,
    ]);
    for n in [3, 4, 3, 3, if opcode % 8 == 6 { 3 } else { 2 }] {
        complete(&mut system, n);
    }
    assert_eq!(system.cpu_snapshot().pc, 0x010A);
    system
}
fn expected(mut snapshot: CpuSnapShot, opcode: u8, value: u8, flags: u8) -> CpuSnapShot {
    let (result, f) = oracle::result(opcode, value, flags);
    snapshot.pc += 2;
    snapshot.af = (snapshot.af & 0xFF00) | u16::from(f);
    let result = u16::from(result);
    match opcode % 8 {
        0 => snapshot.bc = (snapshot.bc & 0x00FF) | (result * 256),
        1 => snapshot.bc = (snapshot.bc & 0xFF00) | result,
        2 => snapshot.de = (snapshot.de & 0x00FF) | (result * 256),
        3 => snapshot.de = (snapshot.de & 0xFF00) | result,
        4 => snapshot.hl = (snapshot.hl & 0x00FF) | (result * 256),
        5 => snapshot.hl = (snapshot.hl & 0xFF00) | result,
        6 => {}
        7 => snapshot.af = (result * 256) | u16::from(f),
        _ => unreachable!(),
    }
    snapshot
}
#[test]
fn every_cb_encoding_has_exact_bus_trace_timing_and_result() {
    for opcode in 0u8..=255 {
        for value in [0, 1, 0x80, 0x81, 0xFF] {
            for flags in [0, 0x10, 0x80, 0xF0] {
                let mut system = setup(opcode, value, flags);
                let before = system.cpu_snapshot();
                let start = system.elapsed_ticks();
                let n = cycles(opcode);
                for cycle in 0..n {
                    let trace = system.step_machine_cycle().unwrap();
                    match cycle {
                        0 | 1 => {
                            assert!(
                                matches!(trace.action, CycleAction::Read { addr } if addr == 0x010A + cycle as u16)
                            );
                            assert_eq!(
                                trace.read_value,
                                Some(if cycle == 0 { 0xCB } else { opcode })
                            );
                        }
                        2 => {
                            assert!(matches!(trace.action, CycleAction::Read { addr: 0xC123 }));
                            assert_eq!(trace.read_value, Some(value));
                        }
                        3 => {
                            assert!(
                                matches!(trace.action, CycleAction::Write { addr: 0xC123, value: actual } if actual == oracle::result(opcode, value, flags).0)
                            );
                            assert_eq!(trace.read_value, None);
                        }
                        _ => unreachable!(),
                    }
                    assert_eq!(
                        (trace.tick_before, trace.tick_after, trace.complete),
                        (start + cycle * 4, start + (cycle + 1) * 4, cycle + 1 == n)
                    );
                    if cycle == 0 || (cycle == 1 && n > 2) {
                        let mut pending = before;
                        pending.pc += cycle as u16 + 1;
                        assert_eq!(system.cpu_snapshot(), pending);
                    }
                }
                assert_eq!(
                    system.cpu_snapshot(),
                    expected(before, opcode, value, flags),
                    "CB {opcode:02X}, value {value:02X}, flags {flags:02X}"
                );
                // A following NOP proves no extra fetch or duplicate write at completion.
                complete(&mut system, 1);
                assert_eq!(system.cpu_snapshot().pc, 0x010D);
            }
        }
    }
}
#[test]
fn every_cb_encoding_resumes_at_each_budget_boundary() {
    for opcode in 0u8..=255 {
        for flags in [0, 0x10] {
            let n = cycles(opcode);
            for budget in (0..=n).chain([100]) {
                let mut system = setup(opcode, 0x81, flags);
                let before = system.cpu_snapshot();
                let start = system.elapsed_ticks();
                let outcome = system.step_instruction(budget).unwrap();
                if budget < n {
                    assert!(
                        matches!(outcome, InstructionStep::BudgetExhausted { mcycles, ticks } if mcycles == budget && ticks == budget * 4)
                    );
                    let pending = system.cpu_snapshot();
                    let ticks = system.elapsed_ticks();
                    assert!(matches!(
                        system.step_instruction(0).unwrap(),
                        InstructionStep::BudgetExhausted {
                            mcycles: 0,
                            ticks: 0
                        }
                    ));
                    assert_eq!(system.cpu_snapshot(), pending);
                    assert_eq!(system.elapsed_ticks(), ticks);
                    complete(&mut system, n - budget);
                } else {
                    assert!(
                        matches!(outcome, InstructionStep::Complete { mcycles, ticks } if mcycles == n && ticks == n * 4)
                    );
                }
                assert_eq!(system.cpu_snapshot(), expected(before, opcode, 0x81, flags));
                assert_eq!(system.elapsed_ticks(), start + n * 4);
                complete(&mut system, 1);
                assert_eq!(system.cpu_snapshot().pc, 0x010D);
            }
        }
    }
}
#[test]
fn cb_memory_flags_drive_taken_and_untaken_branches() {
    let mut system = program(&[
        0x21, 0x23, 0xC1, 0x36, 0, 0x37, 0xCB, 0xDE, 0xCB, 0x5E, 0x28, 2, 0xCB, 0x9E, 0xCB, 0x5E,
        0x28, 2, 0x3E, 0xEE, 0x7E, 0x3E, 0x81, 0xCB, 0x1F, 0x38, 2, 0x3E, 0xDD, 0, 0xD3,
    ]);
    for (n, pc) in [
        (3, 0x103),
        (3, 0x105),
        (1, 0x106),
        (4, 0x108),
        (3, 0x10A),
        (2, 0x10C),
        (4, 0x10E),
        (3, 0x110),
        (3, 0x114),
        (2, 0x115),
        (2, 0x117),
        (2, 0x119),
        (3, 0x11D),
        (1, 0x11E),
    ] {
        complete(&mut system, n);
        assert_eq!(system.cpu_snapshot().pc, pc);
        if pc == 0x115 {
            assert_eq!(system.cpu_snapshot().af, 0x00B0);
        }
    }
    assert_eq!(system.cpu_snapshot().af, 0xC010);
    assert_eq!(system.elapsed_ticks(), 144);
}
#[test]
fn cb_second_opcode_read_crosses_hram_into_unmapped_ie_and_wraps_pc() {
    let mut system = program(&[0x21, 0xFE, 0xFF, 0x36, 0xCB, 0x3E, 0, 0xC3, 0xFE, 0xFF]);
    for n in [3, 3, 2, 4] {
        complete(&mut system, n);
    }
    let start = system.elapsed_ticks();
    let prefix = system.step_machine_cycle().unwrap();
    assert!(matches!(prefix.action, CycleAction::Read { addr: 0xFFFE }));
    assert_eq!(prefix.read_value, Some(0xCB));
    assert!(!prefix.complete);
    let second = system.step_machine_cycle().unwrap();
    assert!(matches!(second.action, CycleAction::Read { addr: 0xFFFF }));
    assert_eq!(second.read_value, Some(0xFF)); // current Bus returns FF for IE
    assert!(second.complete); // SET 7,A
    assert_eq!(system.cpu_snapshot().pc, 0);
    assert_eq!(system.cpu_snapshot().af, 0x80B0);
    assert_eq!(system.elapsed_ticks(), start + 8);
    complete(&mut system, 1); // ROM[0000] NOP
    assert_eq!(system.cpu_snapshot().pc, 1);
}
