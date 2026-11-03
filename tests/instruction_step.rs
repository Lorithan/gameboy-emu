use gameboy_emu::{
    cpu::CpuError,
    memory::Cartridge,
    system::{InstructionStep, System, SystemError},
};

fn machine(program: &[u8]) -> System {
    let mut rom = vec![0; 32_768];
    rom[0x014D] = 0xE7;
    rom[0x0100..0x0100 + program.len()].copy_from_slice(program);
    System::new(Cartridge::from_bytes(&rom).expect("valid synthetic program"))
}

fn assert_completed(outcome: InstructionStep, cycles: u64, ticks: u64) {
    match outcome {
        InstructionStep::Complete {
            mcycles,
            ticks: actual_ticks,
        } => {
            assert_eq!(mcycles, cycles);
            assert_eq!(actual_ticks, ticks);
        }
        other => panic!("expected instruction completion, got {other:?}"),
    }
}

fn assert_exhausted(outcome: InstructionStep, cycles: u64, ticks: u64) {
    match outcome {
        InstructionStep::BudgetExhausted {
            mcycles,
            ticks: actual_ticks,
        } => {
            assert_eq!(mcycles, cycles);
            assert_eq!(actual_ticks, ticks);
        }
        other => panic!("expected budget exhaustion, got {other:?}"),
    }
}

#[test]
fn zero_budget_changes_neither_fresh_nor_partial_instruction_state() {
    let mut system = machine(&[0x3E, 0x42]);
    let fresh = system.cpu_snapshot();
    assert_exhausted(system.step_instruction(0).expect("zero budget"), 0, 0);
    assert_eq!(system.cpu_snapshot(), fresh);
    assert_eq!(system.elapsed_ticks(), 0);

    assert!(!system.step_machine_cycle().expect("opcode cycle").complete);
    let partial = system.cpu_snapshot();
    assert_exhausted(
        system.step_instruction(0).expect("zero budget during LD"),
        0,
        0,
    );
    assert_eq!(system.cpu_snapshot(), partial);
    assert_eq!(system.elapsed_ticks(), 4);
    let operand = system
        .step_machine_cycle()
        .expect("operand must remain pending");
    assert_eq!(operand.read_value, Some(0x42));
    assert!(operand.complete);
}

#[test]
fn nop_uses_one_cycle_and_does_not_fetch_the_following_instruction() {
    for budget in [1, 100] {
        let mut system = machine(&[0x00, 0xD3]);
        let mut expected = system.cpu_snapshot();
        expected.pc = 0x0101;
        let result = system
            .step_instruction(budget)
            .expect("NOP must finish before D3");
        assert_eq!(system.cpu_snapshot(), expected);
        assert_eq!(system.elapsed_ticks(), 4);
        assert_completed(result, 1, 4);
    }
}

#[test]
fn immediate_load_completes_exactly_at_two_cycles() {
    for budget in [2, 100] {
        let mut system = machine(&[0x3E, 0x42, 0xD3]);
        let mut expected = system.cpu_snapshot();
        expected.pc = 0x0102;
        expected.af = 0x42B0;
        let result = system
            .step_instruction(budget)
            .expect("LD must finish before D3");
        assert_eq!(system.cpu_snapshot(), expected);
        assert_eq!(system.elapsed_ticks(), 8);
        assert_completed(result, 2, 8);
    }
}

#[test]
fn exhausted_load_resumes_without_refetching_its_opcode() {
    let mut system = machine(&[0x3E, 0x80, 0xD3]);
    let mut pending = system.cpu_snapshot();
    pending.pc = 0x0101;
    assert_exhausted(system.step_instruction(1).expect("partial LD"), 1, 4);
    assert_eq!(system.cpu_snapshot(), pending);
    assert_eq!(system.elapsed_ticks(), 4);

    let result = system.step_instruction(1).expect("resume operand cycle");
    pending.pc = 0x0102;
    pending.af = 0x80B0;
    assert_eq!(system.cpu_snapshot(), pending);
    assert_eq!(system.elapsed_ticks(), 8);
    assert_completed(result, 1, 4);
}

#[test]
fn returned_counts_are_local_to_each_instruction_call() {
    let mut system = machine(&[0x00, 0x3E, 0xFF, 0x00]);
    system.step_machine_cycle().expect("initial NOP");
    assert_eq!(system.elapsed_ticks(), 4);
    let load = system
        .step_instruction(2)
        .expect("LD after earlier execution");
    assert_eq!(system.elapsed_ticks(), 12);
    assert_eq!(system.cpu_snapshot().pc, 0x0103);
    assert_completed(load, 2, 8);
    let nop = system.step_instruction(1).expect("following NOP");
    assert_eq!(system.elapsed_ticks(), 16);
    assert_completed(nop, 1, 4);
}

#[test]
fn instruction_step_propagates_fault_trace_and_does_not_retry_a_faulted_cpu() {
    let mut system = machine(&[0xD3]);
    match system
        .step_instruction(100)
        .expect_err("unsupported opcode")
    {
        SystemError::CpuCompletionError(CpuError::UnsupportedOpcode(_), trace) => {
            assert_eq!((trace.tick_before, trace.tick_after), (0, 4));
            assert_eq!(trace.read_value, Some(0xD3));
            assert!(!trace.complete);
        }
        other => panic!("wrong error: {other:?}"),
    }
    let faulted = system.cpu_snapshot();
    assert_eq!(faulted.pc, 0x0101);
    assert_eq!(system.elapsed_ticks(), 4);
    assert!(matches!(
        system.step_instruction(100),
        Err(SystemError::CpuRequestError(CpuError::UnsupportedOpcode(_)))
    ));
    assert_eq!(system.cpu_snapshot(), faulted);
    assert_eq!(system.elapsed_ticks(), 4);
    assert_exhausted(
        system
            .step_instruction(0)
            .expect("zero budget makes no request"),
        0,
        0,
    );
    assert_eq!(system.cpu_snapshot(), faulted);
    assert_eq!(system.elapsed_ticks(), 4);
}
