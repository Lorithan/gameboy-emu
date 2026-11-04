use super::System;
use crate::{cpu::CpuSnapShot, memory::Cartridge};

#[test]
fn stack_cycles_obey_bus_mapping_and_preserve_other_memory_at_every_pause() {
    for (pair, push, pop) in [
        (0, 0xC5, 0xC1),
        (1, 0xD5, 0xD1),
        (2, 0xE5, 0xE1),
        (3, 0xF5, 0xF1),
    ] {
        for sp in [0u16, 1, 0x8000, 0xC002, 0xE002, 0xFF80, 0xFFFE, 0xFFFF] {
            for budget in 0u64..=4 {
                let mut rom = vec![0; 32768];
                rom[0x014D] = 0xE7;
                rom[0x0100..0x0106].copy_from_slice(&[
                    0x31,
                    sp as u8,
                    (sp / 256) as u8,
                    push,
                    pop,
                    0xD3,
                ]);
                let mut system = System::new(Cartridge::from_bytes(&rom).unwrap());
                system.step_instruction(3).unwrap();
                let initial = system.cpu_snapshot();
                let value = match pair {
                    0 => initial.bc,
                    1 => initial.de,
                    2 => initial.hl,
                    _ => initial.af,
                };
                let before: Vec<_> = (0u16..=65535)
                    .map(|addr| system.memory.read(addr))
                    .collect();
                system.step_instruction(budget).unwrap();
                let mut expected_memory = before.clone();
                let writes = [
                    (sp.wrapping_sub(1), (value / 256) as u8),
                    (sp.wrapping_sub(2), value as u8),
                ];
                for (index, (addr, byte)) in writes.into_iter().enumerate() {
                    if budget < 3 + index as u64 {
                        continue;
                    }
                    if matches!(addr, 0xC000..=0xFDFF | 0xFF80..=0xFFFE) {
                        expected_memory[usize::from(addr)] = byte;
                        if (0xC000..=0xDDFF).contains(&addr) {
                            expected_memory[usize::from(addr + 0x2000)] = byte;
                        }
                        if (0xE000..=0xFDFF).contains(&addr) {
                            expected_memory[usize::from(addr - 0x2000)] = byte;
                        }
                    }
                }
                for addr in 0u16..=65535 {
                    assert_eq!(
                        system.memory.read(addr),
                        expected_memory[usize::from(addr)],
                        "pair {pair}, SP {sp:04X}, budget {budget}, addr {addr:04X}"
                    );
                }
                if budget < 4 {
                    system.step_instruction(100).unwrap();
                }
                let stack = sp.wrapping_sub(2);
                let word = u16::from(system.memory.read(stack))
                    | (u16::from(system.memory.read(stack.wrapping_add(1))) << 8);
                let memory_before_pop: Vec<_> = (0u16..=65535)
                    .map(|addr| system.memory.read(addr))
                    .collect();
                assert!(matches!(
                    system.step_instruction(100).unwrap(),
                    super::InstructionStep::Complete {
                        mcycles: 3,
                        ticks: 12
                    }
                ));
                let mut expected = initial;
                expected.pc = 0x0105;
                match pair {
                    0 => expected.bc = word,
                    1 => expected.de = word,
                    2 => expected.hl = word,
                    _ => expected.af = word & 0xFFF0,
                }
                assert_eq!(system.cpu_snapshot(), expected);
                for addr in 0u16..=65535 {
                    assert_eq!(
                        system.memory.read(addr),
                        memory_before_pop[usize::from(addr)]
                    );
                }
            }
        }
    }
}

fn hl_access_machine(address: u16, tail: &[u8]) -> System {
    let mut rom = vec![0; 32_768];
    rom[0x014D] = 0xE7;
    rom[0x0100..0x0105].copy_from_slice(&[0x21, address as u8, (address >> 8) as u8, 0x3E, 0x42]);
    rom[0x0105..0x0105 + tail.len()].copy_from_slice(tail);
    let mut system = System::new(Cartridge::from_bytes(&rom).expect("synthetic ROM"));
    system.step_instruction(3).expect("set HL");
    system.step_instruction(2).expect("set A");
    system
}

#[test]
fn high_ram_store_budgets_delay_writes_until_the_write_cycle() {
    use super::InstructionStep;
    for address in [0xFF80u16, 0xFFFE] {
        for prefix in 0u64..=3 {
            let mut system = hl_access_machine(address, &[0x36, 0x5A, 0x7E, 0xD3]);
            system.memory.write(address, 0x27);
            let outcome = system.step_instruction(prefix).unwrap();
            if prefix == 3 {
                assert!(matches!(
                    outcome,
                    InstructionStep::Complete {
                        mcycles: 3,
                        ticks: 12
                    }
                ));
            } else {
                assert!(
                    matches!(outcome, InstructionStep::BudgetExhausted { mcycles, ticks } if mcycles == prefix && ticks == prefix * 4)
                );
            }
            assert_eq!(
                system.memory.read(address),
                if prefix == 3 { 0x5A } else { 0x27 }
            );
            let snapshot = system.cpu_snapshot();
            let tick = system.elapsed_ticks();
            assert!(matches!(
                system.step_instruction(0).unwrap(),
                InstructionStep::BudgetExhausted {
                    mcycles: 0,
                    ticks: 0
                }
            ));
            assert_eq!(system.cpu_snapshot(), snapshot);
            assert_eq!(system.elapsed_ticks(), tick);
            if prefix < 3 {
                assert!(
                    matches!(system.step_instruction(100).unwrap(), InstructionStep::Complete { mcycles, ticks } if mcycles == 3 - prefix && ticks == (3 - prefix) * 4)
                );
            }
            assert_eq!(system.memory.read(address), 0x5A);
            assert_eq!(system.memory.read(0xC000 + (address - 0xFF80)), 0);
            assert_eq!(system.elapsed_ticks(), 32);
        }
    }
}

#[test]
fn memory_accumulator_reads_follow_bus_mapping_without_writes() {
    for (opcode, add) in [(0x86, true), (0x96, false), (0xBE, false)] {
        for address in [
            0u16, 0x7FFF, 0x8000, 0xBFFF, 0xC000, 0xDDFF, 0xDE00, 0xDFFF, 0xE000, 0xFDFF, 0xFE00,
            0xFF80, 0xFFFF,
        ] {
            let mut system = hl_access_machine(address, &[opcode, 0xD3]);
            system.memory.write(address, 0x0F);
            let operand = system.memory.read(address);
            let memory_before: Vec<_> = (0u16..=u16::MAX)
                .map(|addr| system.memory.read(addr))
                .collect();
            let mut expected = system.cpu_snapshot();
            let sign = if add { 1i16 } else { -1 };
            let full = 0x42i16 + sign * i16::from(operand);
            let low = 2i16 + sign * i16::from(operand % 16);
            let value = full.rem_euclid(256) as u16;
            let flags = if value == 0 { 0x80 } else { 0 }
                | if add { 0 } else { 0x40 }
                | if !(0..16).contains(&low) { 0x20 } else { 0 }
                | if !(0..256).contains(&full) { 0x10 } else { 0 };
            expected.af = if opcode == 0xBE {
                0x4200 | flags
            } else {
                value << 8 | flags
            };
            expected.pc += 1;
            assert!(matches!(
                system.step_instruction(100),
                Ok(super::InstructionStep::Complete {
                    mcycles: 2,
                    ticks: 8
                })
            ));
            assert_eq!(system.cpu_snapshot(), expected);
            for (addr, before) in memory_before.into_iter().enumerate() {
                assert_eq!(
                    system.memory.read(addr as u16),
                    before,
                    "opcode {opcode:02X}, memory at {addr:04X}"
                );
            }
        }
    }
}

#[test]
fn memory_modification_budgets_delay_writes_and_never_apply_arithmetic_twice() {
    use super::InstructionStep;
    for (opcode, old, value, flags) in [(0x34, 0x0F, 0x10, 0x30u16), (0x35, 0x10, 0x0F, 0x70)] {
        for prefix in 0u64..=3 {
            let mut system = hl_access_machine(0xC123, &[opcode, 0xD3]);
            system.memory.write(0xC123, old);
            let mut expected = system.cpu_snapshot();
            match system
                .step_instruction(prefix)
                .expect("partial instruction")
            {
                InstructionStep::Complete { mcycles, ticks } => {
                    assert_eq!((prefix, mcycles, ticks), (3, 3, 12))
                }
                InstructionStep::BudgetExhausted { mcycles, ticks } => {
                    assert!(prefix < 3);
                    assert_eq!((mcycles, ticks), (prefix, prefix * 4));
                }
            }
            if prefix > 0 {
                expected.pc += 1;
            }
            if prefix >= 2 {
                expected.af = (expected.af & 0xFF00) | flags;
            }
            assert_eq!(system.cpu_snapshot(), expected);
            assert_eq!(
                system.memory.read(0xC123),
                if prefix == 3 { value } else { old }
            );
            let tick = system.elapsed_ticks();
            for _ in 0..2 {
                system.cpu.request_cycle().expect("inspect pending cycle");
                assert!(matches!(
                    system.step_instruction(0),
                    Ok(InstructionStep::BudgetExhausted {
                        mcycles: 0,
                        ticks: 0
                    })
                ));
                assert_eq!(system.cpu_snapshot(), expected);
                assert_eq!(system.elapsed_ticks(), tick);
                assert_eq!(
                    system.memory.read(0xC123),
                    if prefix == 3 { value } else { old }
                );
            }
            if prefix < 3 {
                if prefix == 0 {
                    expected.pc += 1;
                }
                expected.af = (expected.af & 0xFF00) | flags;
                assert!(
                    matches!(system.step_instruction(100), Ok(InstructionStep::Complete { mcycles, ticks }) if mcycles == 3 - prefix && ticks == (3 - prefix) * 4)
                );
            }
            assert_eq!(system.cpu_snapshot(), expected);
            assert_eq!(system.memory.read(0xC123), value);
            assert_eq!(system.elapsed_ticks(), 32);
        }
    }
}

#[test]
fn memory_modifications_obey_bus_boundaries_even_when_writes_are_ignored() {
    for (opcode, increment) in [(0x34, true), (0x35, false)] {
        for address in [
            0u16, 0x7FFF, 0x8000, 0xBFFF, 0xC000, 0xDDFF, 0xDE00, 0xDFFF, 0xE000, 0xFDFF, 0xFE00,
            0xFF80, 0xFFFF,
        ] {
            let mut system = hl_access_machine(address, &[opcode, 0xD3]);
            system.memory.write(address, 0x0F);
            let old = system.memory.read(address);
            let delta = if increment { 1i16 } else { -1 };
            let value = (i16::from(old) + delta).rem_euclid(256) as u8;
            let half = !(0..16).contains(&(i16::from(old % 16) + delta));
            let flags = 0x10u16
                | if value == 0 { 0x80 } else { 0 }
                | if increment { 0 } else { 0x40 }
                | if half { 0x20 } else { 0 };
            let mut expected = system.cpu_snapshot();
            expected.pc += 1;
            expected.af = (expected.af & 0xFF00) | flags;
            assert!(matches!(
                system.step_instruction(100),
                Ok(super::InstructionStep::Complete {
                    mcycles: 3,
                    ticks: 12
                })
            ));
            assert_eq!(system.cpu_snapshot(), expected);
            assert_eq!(
                system.memory.read(address),
                if matches!(address, 0xC000..=0xFDFF | 0xFF80..=0xFFFE) {
                    value
                } else {
                    old
                }
            );
            match address {
                0xC000..=0xDDFF => assert_eq!(system.memory.read(address + 0x2000), value),
                0xE000..=0xFDFF => assert_eq!(system.memory.read(address - 0x2000), value),
                _ => {}
            }
        }
    }
}

#[test]
fn hl_access_budgets_pause_before_side_effects_and_resume_without_extra_fetches() {
    use super::InstructionStep;
    for (opcode, cycles, bytes) in [(0x77, 2u64, 1u16), (0x36, 3, 2), (0x7E, 2, 1)] {
        let tail: &[u8] = if opcode == 0x36 {
            &[0x36, 0x42, 0xD3]
        } else {
            &[opcode, 0xD3]
        };
        for prefix in 0..=cycles {
            let mut system = hl_access_machine(0xC123, tail);
            if opcode == 0x7E {
                system.memory.write(0xC123, 0x5A);
            }
            let mut expected = system.cpu_snapshot();
            let result = system.step_instruction(prefix).expect("bounded access");
            match result {
                InstructionStep::Complete { mcycles, ticks } => {
                    assert_eq!(prefix, cycles);
                    assert_eq!((mcycles, ticks), (cycles, cycles * 4));
                }
                InstructionStep::BudgetExhausted { mcycles, ticks } => {
                    assert!(prefix < cycles);
                    assert_eq!((mcycles, ticks), (prefix, prefix * 4));
                }
            }
            let paused = system.cpu_snapshot();
            let paused_ticks = system.elapsed_ticks();
            let paused_memory = system.memory.read(0xC123);
            assert_eq!(paused_ticks, 20 + prefix * 4);
            if prefix < cycles && opcode != 0x7E {
                assert_eq!(paused_memory, 0, "write must wait for its own cycle");
            }
            for _ in 0..2 {
                system.cpu.request_cycle().expect("inspect pending action");
                assert!(matches!(
                    system.step_instruction(0),
                    Ok(InstructionStep::BudgetExhausted {
                        mcycles: 0,
                        ticks: 0
                    })
                ));
                assert_eq!(system.cpu_snapshot(), paused);
                assert_eq!(system.elapsed_ticks(), paused_ticks);
                assert_eq!(system.memory.read(0xC123), paused_memory);
            }
            if prefix < cycles {
                assert!(
                    matches!(system.step_instruction(100), Ok(InstructionStep::Complete { mcycles, ticks }) if mcycles == cycles - prefix && ticks == (cycles - prefix) * 4)
                );
            }
            expected.pc += bytes;
            if opcode == 0x7E {
                expected.af = 0x5A00 | (expected.af & 0xFF);
            }
            assert_eq!(system.cpu_snapshot(), expected);
            assert_eq!(system.elapsed_ticks(), 20 + cycles * 4);
            assert_eq!(
                system.memory.read(0xC123),
                if opcode == 0x7E { 0x5A } else { 0x42 }
            );
            assert!(
                matches!(system.cpu.request_cycle(), Ok(crate::cpu::CycleAction::Read { addr }) if addr == expected.pc)
            );
        }
    }
}

#[test]
fn cpu_memory_accesses_respect_wram_echo_rom_and_unimplemented_boundaries() {
    for opcode in [0x77, 0x36] {
        for address in [
            0x0000u16, 0x7FFF, 0x8000, 0xBFFF, 0xC000, 0xDDFF, 0xDE00, 0xDFFF, 0xE000, 0xFDFF,
            0xFE00, 0xFF80, 0xFFFF,
        ] {
            let tail: &[u8] = if opcode == 0x36 {
                &[0x36, 0x42, 0x7E, 0xD3]
            } else {
                &[0x77, 0x7E, 0xD3]
            };
            let mut system = hl_access_machine(address, tail);
            let original = system.memory.read(address);
            system.step_instruction(3).expect("store");
            let expected = match address {
                0x0000..=0x7FFF => original,
                0xC000..=0xFDFF | 0xFF80..=0xFFFE => 0x42,
                _ => 0xFF,
            };
            assert_eq!(system.memory.read(address), expected);
            match address {
                0xC000..=0xDDFF => assert_eq!(system.memory.read(address + 0x2000), 0x42),
                0xE000..=0xFDFF => assert_eq!(system.memory.read(address - 0x2000), 0x42),
                _ => {}
            }
            let mut expected_snapshot = system.cpu_snapshot();
            expected_snapshot.pc += 1;
            expected_snapshot.af = u16::from(expected) << 8 | (expected_snapshot.af & 0xFF);
            system.step_instruction(2).expect("read back through CPU");
            assert_eq!(system.cpu_snapshot(), expected_snapshot);
        }
    }
}

fn assert_post_boot_registers(system: &System, expected_af: u16) {
    assert_eq!(
        system.cpu_snapshot(),
        CpuSnapShot {
            af: expected_af,
            bc: 0x0013,
            de: 0x00D8,
            hl: 0x014D,
            pc: 0x0100,
            sp: 0xFFFE,
        }
    );
}

fn construct_system(title_byte: u8, checksum: u8) -> (System, Vec<u8>) {
    let mut rom = vec![0; 32_768];
    rom[0x0134] = title_byte;
    rom[0x014D] = checksum;
    rom[0] = 0x35;
    rom[0x7FFF] = 0x76;
    let cartridge = Cartridge::from_bytes(&rom).expect("valid synthetic cartridge");
    (System::new(cartridge), rom)
}

#[test]
fn system_uses_zero_checksum_boot_flags() {
    // -(0xE7 + 25) modulo 256 is zero.
    let (system, _) = construct_system(0xE7, 0);
    assert_post_boot_registers(&system, 0x0180);
}

#[test]
fn system_uses_nonzero_checksum_boot_flags() {
    // Independent known checksums for two synthetic headers.
    for (title_byte, checksum) in [(0, 0xE7), (0xE8, 0xFF)] {
        let (system, _) = construct_system(title_byte, checksum);
        assert_post_boot_registers(&system, 0x01B0);
    }
}

#[test]
fn system_moves_the_validated_cartridge_into_its_bus_unchanged() {
    let (system, rom) = construct_system(0, 0xE7);
    for address in 0u16..=0x7FFF {
        assert_eq!(system.memory.read(address), rom[usize::from(address)]);
    }
}

#[test]
fn systems_have_independent_zero_initialized_work_ram() {
    let (mut first, _) = construct_system(0, 0xE7);
    let (second, _) = construct_system(0xE7, 0);
    for address in 0xC000u16..=0xFDFF {
        assert_eq!(first.memory.read(address), 0);
        assert_eq!(second.memory.read(address), 0);
    }
    first.memory.write(0xC123, 0xA5);
    assert_eq!(first.memory.read(0xE123), 0xA5);
    assert_eq!(second.memory.read(0xC123), 0);
    assert_eq!(second.memory.read(0xE123), 0);
    assert_post_boot_registers(&first, 0x01B0);
    assert_post_boot_registers(&second, 0x0180);
}

fn program_system(program: &[u8]) -> System {
    let mut rom = vec![0; 32_768];
    rom[0x014D] = 0xE7;
    rom[0x0100..0x0100 + program.len()].copy_from_slice(program);
    System::new(Cartridge::from_bytes(&rom).expect("valid synthetic program"))
}

#[test]
fn clock_starts_at_zero_and_inspection_does_not_advance_time() {
    let system = program_system(&[0]);
    let original = system.cpu_snapshot();
    for _ in 0..3 {
        assert_eq!(system.elapsed_ticks(), 0);
        assert_eq!(system.cpu_snapshot(), original);
    }
}

#[test]
fn immediate_values_are_data_and_preserve_flags() {
    for immediate in [0, 0x3E, 0x80, 0xFF] {
        let mut system = program_system(&[0x3E, immediate]);
        let initial = system.cpu_snapshot();
        assert!(
            !system
                .step_machine_cycle()
                .expect("LD opcode cycle")
                .complete
        );
        assert_eq!(system.cpu_snapshot().af, initial.af);
        assert_eq!(system.elapsed_ticks(), 4);
        assert!(
            system
                .step_machine_cycle()
                .expect("LD operand cycle")
                .complete
        );
        let mut expected = initial;
        expected.pc = 0x0102;
        expected.af = u16::from(immediate) << 8 | (initial.af & 0xFF);
        assert_eq!(system.cpu_snapshot(), expected);
        assert_eq!(system.elapsed_ticks(), 8);
    }
}

#[test]
fn system_clocks_and_execution_states_are_independent() {
    let mut first = program_system(&[0, 0]);
    let second = program_system(&[0x3E, 0x42]);
    let untouched = second.cpu_snapshot();
    first.step_machine_cycle().expect("first system NOP");
    first.step_machine_cycle().expect("second NOP");
    assert_eq!(first.elapsed_ticks(), 8);
    assert_eq!(second.elapsed_ticks(), 0);
    assert_eq!(second.cpu_snapshot(), untouched);
}

#[test]
fn each_trace_records_its_own_bus_action_and_read_byte() {
    use crate::cpu::CycleAction;

    let mut system = program_system(&[0x00, 0x3E, 0x42, 0x00]);
    for (address, value, complete) in [
        (0x0100, 0x00, true),
        (0x0101, 0x3E, false),
        (0x0102, 0x42, true),
        (0x0103, 0x00, true),
    ] {
        let trace = system.step_machine_cycle().expect("supported cycle");
        match trace.action {
            CycleAction::Read { addr } => assert_eq!(addr, address),
            _ => panic!("expected a read cycle at {address:#06X}"),
        }
        assert_eq!(trace.read_value, Some(value));
        assert_eq!(trace.complete, complete);
    }
}

#[test]
fn each_trace_counts_one_machine_cycle_not_the_whole_instruction() {
    let mut system = program_system(&[0x00, 0x3E, 0x42, 0x00]);
    for expected_elapsed in [4, 8, 12, 16] {
        let trace = system.step_machine_cycle().expect("supported cycle");
        assert_eq!(trace.tick_before, expected_elapsed - 4);
        assert_eq!(trace.tick_after, expected_elapsed);
        assert_eq!(system.elapsed_ticks(), expected_elapsed);
    }
}

#[test]
fn failing_read_retains_its_byte_and_tick_span_in_the_error_trace() {
    use super::SystemError;
    use crate::cpu::{CpuError, CycleAction};

    let mut system = program_system(&[0xD3]);
    match system.step_machine_cycle().expect_err("unsupported opcode") {
        SystemError::CpuCompletionError(CpuError::UnsupportedOpcode(_), trace) => {
            assert_eq!(trace.tick_before, 0);
            assert_eq!(trace.tick_after, 4);
            assert!(matches!(trace.action, CycleAction::Read { addr: 0x0100 }));
            assert_eq!(trace.read_value, Some(0xD3));
            assert!(!trace.complete);
        }
        other => panic!("wrong execution error: {other:?}"),
    }
    assert_eq!(system.elapsed_ticks(), 4);
}
