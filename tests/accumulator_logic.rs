use gameboy_emu::{
    cpu::CycleAction,
    memory::Cartridge,
    system::{InstructionStep, System},
};

fn program(bytes: &[u8]) -> System {
    let mut rom = vec![0; 32768];
    rom[0x014D] = 0xE7;
    rom[0x0100..0x0100 + bytes.len()].copy_from_slice(bytes);
    System::new(Cartridge::from_bytes(&rom).unwrap())
}

fn complete(system: &mut System, cycles: u64) {
    assert!(
        matches!(system.step_instruction(100).unwrap(), InstructionStep::Complete { mcycles, ticks } if mcycles == cycles && ticks == 4 * cycles)
    );
}

fn machine(opcode: u8, source: usize, a: u8, right: u8) -> System {
    let (mut bytes, setup): (Vec<u8>, Vec<u64>) = match source {
        6 => (vec![0x21, 0x23, 0xC1, 0x36, right, 0x3E, a], vec![3, 3, 2]),
        7 | 8 => (vec![0x3E, a], vec![2]),
        _ => (
            vec![[0x06, 0x0E, 0x16, 0x1E, 0x26, 0x2E][source], right, 0x3E, a],
            vec![2, 2],
        ),
    };
    bytes.push(opcode);
    if source == 8 {
        bytes.push(right);
    }
    bytes.push(0xD3);
    let mut system = program(&bytes);
    for cycles in setup {
        complete(&mut system, cycles);
    }
    system
}

#[test]
fn every_logic_form_has_exact_read_traces_and_resumes_all_budgets() {
    for (group, (base, immediate)) in [(0xA0u8, 0xE6u8), (0xA8, 0xEE), (0xB0, 0xF6)]
        .into_iter()
        .enumerate()
    {
        for source in 0..9 {
            let opcode = if source == 8 {
                immediate
            } else {
                base + source as u8
            };
            for (a, operand, results) in [
                (0x96u8, 0x69u8, [0u8, 0xFF, 0xFF]),
                (0, 0, [0, 0, 0]),
                (0xFF, 0x0F, [0x0F, 0xF0, 0xFF]),
            ] {
                let right = if source == 7 { a } else { operand };
                let result = if source == 7 {
                    if group == 1 { 0 } else { a }
                } else {
                    results[group]
                };
                let mut system = machine(opcode, source, a, right);
                let before = system.cpu_snapshot();
                let ticks = system.elapsed_ticks();
                let cycles = if source == 6 || source == 8 { 2 } else { 1 };
                let mut expected = before;
                expected.pc += if source == 8 { 2 } else { 1 };
                expected.af = u16::from(result) * 256
                    + if result == 0 { 0x80 } else { 0 }
                    + if group == 0 { 0x20 } else { 0 };
                for cycle in 0..cycles {
                    let trace = system.step_machine_cycle().unwrap();
                    let addr = if cycle == 0 {
                        before.pc
                    } else if source == 6 {
                        0xC123
                    } else {
                        before.pc + 1
                    };
                    assert!(
                        matches!(trace.action, CycleAction::Read { addr: actual } if actual == addr)
                    );
                    assert_eq!(
                        trace.read_value,
                        Some(if cycle == 0 { opcode } else { right })
                    );
                    assert_eq!(
                        (trace.tick_before, trace.tick_after),
                        (ticks + 4 * cycle, ticks + 4 * (cycle + 1))
                    );
                    assert_eq!(trace.complete, cycle + 1 == cycles);
                    if !trace.complete {
                        let mut pending = before;
                        pending.pc += 1;
                        assert_eq!(system.cpu_snapshot(), pending);
                    }
                }
                assert_eq!(system.cpu_snapshot(), expected);
                for budget in [0, 1, 2, 100] {
                    let mut system = machine(opcode, source, a, right);
                    let outcome = system.step_instruction(budget).unwrap();
                    if budget < cycles {
                        assert!(
                            matches!(outcome, InstructionStep::BudgetExhausted { mcycles, ticks } if mcycles == budget && ticks == 4 * budget)
                        );
                        let mut pending = before;
                        pending.pc += budget as u16;
                        assert_eq!(system.cpu_snapshot(), pending);
                        assert!(matches!(
                            system.step_instruction(0).unwrap(),
                            InstructionStep::BudgetExhausted {
                                mcycles: 0,
                                ticks: 0
                            }
                        ));
                        complete(&mut system, cycles - budget);
                    } else {
                        assert!(
                            matches!(outcome, InstructionStep::Complete { mcycles, ticks } if mcycles == cycles && ticks == 4 * cycles)
                        );
                    }
                    assert_eq!(system.cpu_snapshot(), expected);
                    assert_eq!(system.elapsed_ticks(), ticks + 4 * cycles);
                }
            }
        }
    }
}

#[test]
fn logic_flags_control_taken_and_untaken_branches() {
    let mut system = program(&[
        0x3E, 0xF0, // A=F0
        0xE6, 0x0F, // AND -> A=00, F=A0
        0x20, 0x02, // JR NZ untaken
        0x28, 0x01, // JR Z taken, skip illegal sentinel
        0xD3, 0xF6, 0x81, // OR -> A=81, F=00
        0x38, 0x02, // JR C untaken (logic cleared C)
        0xEE, 0x81, // XOR -> A=00, F=80
        0x30, 0x01, // JR NC taken, skip sentinel
        0xD3, 0x00, 0xD3,
    ]);
    for (cycles, af) in [
        (2, 0xF0B0),
        (2, 0x00A0),
        (2, 0x00A0),
        (3, 0x00A0),
        (2, 0x8100),
        (2, 0x8100),
        (2, 0x0080),
        (3, 0x0080),
        (1, 0x0080),
    ] {
        complete(&mut system, cycles);
        assert_eq!(system.cpu_snapshot().af, af);
    }
    assert_eq!(system.cpu_snapshot().pc, 0x0113);
    assert_eq!(system.elapsed_ticks(), 76);
}
