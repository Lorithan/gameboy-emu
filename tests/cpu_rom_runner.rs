#[path = "support/rom_runner.rs"]
mod runner;

use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
    time::Duration,
};

use gameboy_emu::{
    cpu::{CpuError, CycleAction, CycleOutcome, OpcodeEncoding},
    memory::Cartridge,
    system::{MachineStep, System, SystemError},
};
use runner::{FAIL_SIGNATURE, Limits, Outcome, PASS_SIGNATURE, Timeout};

#[test]
#[ignore = "Requires the pinned external ROM; run tests/scripts/run-mooneye-reg-f.ps1 to verify its SHA-256 first"]
fn mooneye_reg_f_external() {
    let path = std::env::var_os("MOONEYE_REG_F")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("target/test-roms/mooneye/acceptance/bits/reg_f.gb")
        });
    assert_external_pass(&path);
}

#[test]
#[ignore = "Requires pinned ROMs; verify hashes with tests/scripts/run-mooneye-cpu-subset.ps1"]
fn mooneye_cpu_if_ie_registers() {
    run_cpu_fixture("acceptance/if_ie_registers.gb");
}

#[test]
#[ignore = "Requires pinned ROMs; verify hashes with tests/scripts/run-mooneye-cpu-subset.ps1"]
fn mooneye_cpu_ie_push() {
    run_cpu_fixture("acceptance/interrupts/ie_push.gb");
}

#[test]
#[ignore = "Requires pinned ROMs; verify hashes with tests/scripts/run-mooneye-cpu-subset.ps1"]
fn mooneye_cpu_ei_sequence() {
    run_cpu_fixture("acceptance/ei_sequence.gb");
}

#[test]
#[ignore = "Requires pinned ROMs; verify hashes with tests/scripts/run-mooneye-cpu-subset.ps1"]
fn mooneye_cpu_div_timing() {
    run_cpu_fixture("acceptance/div_timing.gb");
}

#[test]
#[ignore = "Requires pinned ROMs; verify hashes with tests/scripts/run-mooneye-timer.ps1"]
fn mooneye_timer_div_write() {
    run_cpu_fixture("acceptance/timer/div_write.gb");
}

#[test]
#[ignore = "Requires pinned ROMs; verify hashes with tests/scripts/run-mooneye-timer.ps1"]
fn mooneye_timer_rapid_toggle() {
    run_cpu_fixture("acceptance/timer/rapid_toggle.gb");
}

#[test]
#[ignore = "Requires pinned ROMs; verify hashes with tests/scripts/run-mooneye-timer.ps1"]
fn mooneye_timer_tim00() {
    run_cpu_fixture("acceptance/timer/tim00.gb");
}

#[test]
#[ignore = "Requires pinned ROMs; verify hashes with tests/scripts/run-mooneye-timer.ps1"]
fn mooneye_timer_tim00_div_trigger() {
    run_cpu_fixture("acceptance/timer/tim00_div_trigger.gb");
}

#[test]
#[ignore = "Requires pinned ROMs; verify hashes with tests/scripts/run-mooneye-timer.ps1"]
fn mooneye_timer_tim01() {
    run_cpu_fixture("acceptance/timer/tim01.gb");
}

#[test]
#[ignore = "Requires pinned ROMs; verify hashes with tests/scripts/run-mooneye-timer.ps1"]
fn mooneye_timer_tim01_div_trigger() {
    run_cpu_fixture("acceptance/timer/tim01_div_trigger.gb");
}

#[test]
#[ignore = "Requires pinned ROMs; verify hashes with tests/scripts/run-mooneye-timer.ps1"]
fn mooneye_timer_tim10() {
    run_cpu_fixture("acceptance/timer/tim10.gb");
}

#[test]
#[ignore = "Requires pinned ROMs; verify hashes with tests/scripts/run-mooneye-timer.ps1"]
fn mooneye_timer_tim10_div_trigger() {
    run_cpu_fixture("acceptance/timer/tim10_div_trigger.gb");
}

#[test]
#[ignore = "Requires pinned ROMs; verify hashes with tests/scripts/run-mooneye-timer.ps1"]
fn mooneye_timer_tim11() {
    run_cpu_fixture("acceptance/timer/tim11.gb");
}

#[test]
#[ignore = "Requires pinned ROMs; verify hashes with tests/scripts/run-mooneye-timer.ps1"]
fn mooneye_timer_tim11_div_trigger() {
    run_cpu_fixture("acceptance/timer/tim11_div_trigger.gb");
}

#[test]
#[ignore = "Requires pinned ROMs; verify hashes with tests/scripts/run-mooneye-timer.ps1"]
fn mooneye_timer_tima_reload() {
    run_cpu_fixture("acceptance/timer/tima_reload.gb");
}

#[test]
#[ignore = "Requires pinned ROMs; verify hashes with tests/scripts/run-mooneye-timer.ps1"]
fn mooneye_timer_tima_write_reloading() {
    run_cpu_fixture("acceptance/timer/tima_write_reloading.gb");
}

#[test]
#[ignore = "Requires pinned ROMs; verify hashes with tests/scripts/run-mooneye-timer.ps1"]
fn mooneye_timer_tma_write_reloading() {
    run_cpu_fixture("acceptance/timer/tma_write_reloading.gb");
}

// #73 D1: pinned Mooneye PPU ROMs (tests/fixtures/mooneye-ppu.json). The
// three regression ROMs must pass; the other nine are calibration targets
// measured by the non-asserting probe below.

#[test]
#[ignore = "Requires pinned ROMs; verify hashes with tests/scripts/run-mooneye-ppu.ps1"]
fn mooneye_ppu_regression_stat_irq_blocking() {
    run_cpu_fixture("acceptance/ppu/stat_irq_blocking.gb");
}

#[test]
#[ignore = "Requires pinned ROMs; verify hashes with tests/scripts/run-mooneye-ppu.ps1"]
fn mooneye_ppu_regression_stat_lyc_onoff() {
    run_cpu_fixture("acceptance/ppu/stat_lyc_onoff.gb");
}

#[test]
#[ignore = "Requires pinned ROMs; verify hashes with tests/scripts/run-mooneye-ppu.ps1"]
fn mooneye_ppu_regression_vblank_stat_intr_gs() {
    run_cpu_fixture("acceptance/ppu/vblank_stat_intr-GS.gb");
}

/// Mooneye's `setup_assertions` saves F,A,C,B,E,D,L,H at FF80-FF87, the
/// assertion mask at FF88 and the expected values at FF89-FF90 (per each
/// ROM's .sym). Returns the last value the CPU wrote to each of FF80-FF90.
fn mooneye_assertion_writes(path: &std::path::Path, mcycles: u64) -> [Option<u8>; 17] {
    let mut system = System::new(Cartridge::from_filepath(path).unwrap());
    let mut written = [None; 17];
    for _ in 0..mcycles {
        if let Ok(MachineStep::Cycle(trace)) = system.step_machine_cycle()
            && let CycleAction::Write {
                addr: addr @ 0xFF80..=0xFF90,
                value,
            } = trace.record.action
        {
            written[usize::from(addr - 0xFF80)] = Some(value);
        }
    }
    written
}

#[test]
#[ignore = "Calibration probe; never asserts a pass. Run tests/scripts/run-mooneye-ppu.ps1 -Mode probe"]
fn mooneye_ppu_calibration_probe() {
    let root = std::env::var_os("MOONEYE_CPU_ROM_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/test-roms/mooneye")
        });
    for name in [
        "intr_1_2_timing-GS",
        "intr_2_0_timing",
        "intr_2_mode0_timing",
        "intr_2_mode3_timing",
        "intr_2_oam_ok_timing",
        "lcdon_timing-GS",
        "lcdon_write_timing-GS",
        "hblank_ly_scx_timing-GS",
        "intr_2_mode0_timing_sprites",
    ] {
        let path = root.join(format!("acceptance/ppu/{name}.gb"));
        let report = runner::run_file(&path, runner::MOONEYE_LIMITS)
            .unwrap_or_else(|error| panic!("{name} setup failed: {error}"));
        // A complete report, not a hang or malformed output, and below one
        // silent wait_ly_with_timeout timeout.
        assert!(
            matches!(report.outcome, Outcome::Passed | Outcome::ReportedFailure),
            "{name}: {report:#?}"
        );
        assert!(
            report.mcycles < 851_968,
            "{name}: possible silent LY-wait timeout"
        );
        let written = mooneye_assertion_writes(&path, report.mcycles);
        let byte = |index: usize| written[index].map_or("--".to_owned(), |v| format!("{v:02X}"));
        let registers = ["F", "A", "C", "B", "E", "D", "L", "H"];
        let saved: Vec<_> = (0..8)
            .map(|i| format!("{}={}", registers[i], byte(i)))
            .collect();
        let expected: Vec<_> = (0..8)
            .map(|i| format!("{}={}", registers[i], byte(9 + i)))
            .collect();
        eprintln!(
            "PPU probe {name}: {:?}, {} M-cycles; assert mask {}; saved [{}]; expected [{}]",
            report.outcome,
            report.mcycles,
            byte(8),
            saved.join(" "),
            expected.join(" ")
        );
    }
}

fn run_cpu_fixture(relative: &str) {
    let root = std::env::var_os("MOONEYE_CPU_ROM_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/test-roms/mooneye")
        });
    assert_external_pass(&root.join(relative));
}

fn assert_external_pass(path: &std::path::Path) {
    let started = std::time::Instant::now();
    let report = runner::run_file(path, runner::MOONEYE_LIMITS)
        .unwrap_or_else(|error| panic!("external fixture setup failed: {error}"));
    eprintln!(
        "Mooneye {}: {:?}, {} M-cycles, {} ticks, host {:?}, packet {:?}, CPU {:?}",
        path.display(),
        report.outcome,
        report.mcycles,
        report.ticks,
        started.elapsed(),
        report.packet,
        report.snapshot
    );
    if !matches!(report.outcome, Outcome::Passed)
        && path
            .file_name()
            .is_some_and(|name| name == "rapid_toggle.gb")
    {
        diagnose_rapid_toggle_handler(path);
    }
    assert!(matches!(report.outcome, Outcome::Passed), "{report:#?}");
}

fn diagnose_rapid_toggle_handler(path: &std::path::Path) {
    // Pinned symbol test_finish=0190: observe before assertion/report macros
    // replace the body registers. Diagnostics never alter emulated state.
    let mut system = System::new(Cartridge::from_filepath(path).unwrap());
    let started = std::time::Instant::now();
    let mut recent = std::collections::VecDeque::with_capacity(32);
    for _ in 0..runner::MOONEYE_LIMITS.mcycles {
        if started.elapsed() >= runner::MOONEYE_LIMITS.wall_time {
            break;
        }
        let before = system.cpu_snapshot();
        let Ok(gameboy_emu::system::MachineStep::Cycle(trace)) = system.step_machine_cycle() else {
            break;
        };
        if recent.len() == 32 {
            recent.pop_front();
        }
        recent.push_back((before, trace, system.cpu_snapshot()));
        if matches!(
            trace.outcome,
            CycleOutcome::InterruptEntryComplete { vector: 0x50 }
        ) {
            eprintln!(
                "rapid_toggle handler entry: {:?}, ticks {}",
                system.cpu_snapshot(),
                system.elapsed_ticks()
            );
        }
        if system.cpu_snapshot().pc == 0x190 && trace.outcome == CycleOutcome::InstructionComplete {
            eprintln!(
                "rapid_toggle body assertions: BC={:04X}, expected FFD9, ticks {}",
                system.cpu_snapshot().bc,
                system.elapsed_ticks()
            );
            for (before, trace, after) in recent {
                eprintln!(
                    "rapid_toggle boundary: PC={:04X} BC={:04X}, {:?}, after PC={:04X} BC={:04X}",
                    before.pc, before.bc, trace, after.pc, after.bc
                );
            }
            break;
        }
    }
}

// Original synthetic programs only. No external suite's ROM or code is embedded.
fn rom(program: &[u8]) -> Vec<u8> {
    let mut bytes = vec![0; 32768];
    bytes[0x14D] = 0xE7;
    bytes[0x100..0x103].copy_from_slice(&[0xC3, 0x50, 1]);
    bytes[0x150..0x150 + program.len()].copy_from_slice(program);
    bytes
}

fn system(program: &[u8]) -> System {
    System::new(Cartridge::from_bytes(&rom(program)).unwrap())
}

fn limits(mcycles: u64) -> Limits {
    Limits {
        mcycles,
        ..runner::MOONEYE_LIMITS
    }
}

fn report_program(signature: [u8; 6], packet: &[u8], offset: u8) -> Vec<u8> {
    let [b, c, d, e, h, l] = signature;
    let mut program = vec![0x01, c, b, 0x11, e, d, 0x21, l, h];
    for &byte in packet {
        program.extend_from_slice(&[0x3E, byte, 0xE0, offset]);
    }
    program.extend_from_slice(&[0x18, 0xFE]);
    program
}

#[test]
fn runner_requires_exact_success_packet_and_register_signature() {
    let mut s = system(&report_program(PASS_SIGNATURE, &PASS_SIGNATURE, 1));
    let report = runner::run(&mut s, "synthetic success", limits(100));
    assert!(matches!(report.outcome, Outcome::Passed), "{report:#?}");
    assert_eq!(report.fixture, "synthetic success");
    assert_eq!(report.packet, PASS_SIGNATURE);
    // JP is four cycles, three word loads are nine, six LD A/LDH pairs are 30.
    assert_eq!((report.mcycles, report.ticks), (43, 172));
    assert_eq!(report.snapshot, s.cpu_snapshot());
    assert_eq!(report.snapshot.pc, 0x171);
    assert_eq!(report.recent.len(), 43);
    assert_eq!(
        report.recent.back().unwrap().record.action,
        CycleAction::Write {
            addr: 0xFF01,
            value: 34
        }
    );
}

#[test]
fn runner_reports_explicit_failure_and_rejects_mismatched_failure_registers() {
    for (signature, fails) in [(FAIL_SIGNATURE, true), (PASS_SIGNATURE, false)] {
        let mut s = system(&report_program(signature, &FAIL_SIGNATURE, 1));
        let report = runner::run(&mut s, "synthetic failure", limits(100));
        if fails {
            assert!(
                matches!(report.outcome, Outcome::ReportedFailure),
                "{report:#?}"
            );
        } else {
            assert!(
                matches!(report.outcome, Outcome::MalformedReport),
                "{report:#?}"
            );
        }
        assert_eq!(report.packet, FAIL_SIGNATURE);
        assert_eq!(report.mcycles, 43);
    }
}

#[test]
fn runner_rejects_each_wrong_signature_register_despite_a_passing_packet() {
    for index in 0..6 {
        let mut signature = PASS_SIGNATURE;
        signature[index] ^= 0x80;
        let mut s = system(&report_program(signature, &PASS_SIGNATURE, 1));
        let report = runner::run(&mut s, "wrong registers", limits(100));
        assert!(
            matches!(report.outcome, Outcome::MalformedReport),
            "{report:#?}"
        );
    }
}

#[test]
fn runner_stops_on_first_malformed_packet_without_searching_for_later_success() {
    let malformed = [0, 5, 8, 13, 21, 34];
    let mut packet = malformed.to_vec();
    packet.extend_from_slice(&PASS_SIGNATURE);
    let mut s = system(&report_program(PASS_SIGNATURE, &packet, 1));
    let report = runner::run(&mut s, "malformed then success", limits(100));
    assert!(
        matches!(report.outcome, Outcome::MalformedReport),
        "{report:#?}"
    );
    assert_eq!(report.packet, malformed);
    assert_eq!(report.mcycles, 43);
}

#[test]
fn runner_never_passes_partial_packets_wrong_addresses_or_incidental_breakpoint_bytes() {
    for (program, expected_packet) in [
        (
            report_program(PASS_SIGNATURE, &PASS_SIGNATURE[..5], 1),
            PASS_SIGNATURE[..5].to_vec(),
        ),
        (report_program(PASS_SIGNATURE, &PASS_SIGNATURE, 2), vec![]),
        (vec![0x40, 0x3E, 0x40, 0x18, 0xFE], vec![]),
    ] {
        let report = runner::run(&mut system(&program), "non-result", limits(100));
        assert!(
            matches!(report.outcome, Outcome::TimedOut(Timeout::CycleBudget)),
            "{report:#?}"
        );
        assert_eq!(report.packet, expected_packet);
        assert_eq!((report.mcycles, report.ticks), (100, 400));
    }
}

#[test]
fn runner_accepts_success_exactly_at_budget_boundary_and_never_executes_beyond_it() {
    for (budget, passes, count) in [
        (0, false, 0),
        (42, false, 42),
        (43, true, 43),
        (44, true, 43),
    ] {
        let mut s = system(&report_program(PASS_SIGNATURE, &PASS_SIGNATURE, 1));
        let report = runner::run(&mut s, "budget boundary", limits(budget));
        assert_eq!(
            matches!(report.outcome, Outcome::Passed),
            passes,
            "{report:#?}"
        );
        if !passes {
            assert!(matches!(
                report.outcome,
                Outcome::TimedOut(Timeout::CycleBudget)
            ));
        }
        assert_eq!((report.mcycles, report.ticks), (count, count * 4));
    }
}

#[test]
fn runner_terminates_stopped_execution_including_an_already_stopped_system() {
    let mut s = system(&[0x10, 0]);
    let report = runner::run(&mut s, "unexpected stop", limits(100));
    assert!(
        matches!(report.outcome, Outcome::UnexpectedStop),
        "{report:#?}"
    );
    assert_eq!((report.mcycles, report.ticks), (5, 20));
    assert_eq!(
        report.recent.back().unwrap().outcome,
        CycleOutcome::StopEntered
    );
    let report = runner::run(&mut s, "already stopped", limits(100));
    assert!(matches!(report.outcome, Outcome::UnexpectedStop));
    assert_eq!((report.mcycles, report.ticks), (0, 0));
    assert!(report.recent.is_empty());
}

#[test]
fn runner_preserves_cpu_errors_and_counts_completion_but_not_request_failures() {
    let mut s = system(&[0xD3]);
    let report = runner::run(&mut s, "invalid opcode", limits(100));
    match report.outcome {
        Outcome::ExecutionError(SystemError::CpuCompletionError(
            CpuError::UnsupportedOpcode(fault),
            record,
        )) => {
            assert_eq!(fault.opcode, OpcodeEncoding::Base(0xD3));
            assert_eq!(fault.addr, 0x150);
            assert_eq!(record.action, CycleAction::Read { addr: 0x150 });
            assert_eq!((record.tick_before, record.tick_after), (16, 20));
        }
        _ => panic!("missing completion diagnostics: {report:#?}"),
    }
    assert_eq!((report.mcycles, report.ticks), (5, 20));
    assert_eq!(report.recent.len(), 4);
    let report = runner::run(&mut s, "sticky fault", limits(100));
    assert!(matches!(
        report.outcome,
        Outcome::ExecutionError(SystemError::CpuRequestError(CpuError::UnsupportedOpcode(_)))
    ));
    assert_eq!((report.mcycles, report.ticks), (0, 0));
    assert!(report.recent.is_empty());
}

#[test]
fn runner_bounds_loops_and_halt_and_retains_only_recent_completed_traces() {
    for program in [&[0x18, 0xFE][..], &[0x76][..]] {
        let report = runner::run(&mut system(program), "loop or HALT", limits(100));
        assert!(matches!(
            report.outcome,
            Outcome::TimedOut(Timeout::CycleBudget)
        ));
        assert_eq!((report.mcycles, report.ticks), (100, 400));
        assert_eq!(report.recent.len(), runner::TRACE_CAPACITY);
        assert_eq!(report.recent.front().unwrap().record.tick_before, 144);
        assert_eq!(report.recent.back().unwrap().record.tick_after, 400);
    }
}

#[test]
fn runner_zero_wall_time_limit_returns_without_mutating_the_machine() {
    let mut s = system(&[0x18, 0xFE]);
    let before = s.cpu_snapshot();
    let report = runner::run(
        &mut s,
        "wall deadline",
        Limits {
            wall_time: Duration::ZERO,
            ..limits(100)
        },
    );
    assert!(matches!(
        report.outcome,
        Outcome::TimedOut(Timeout::WallTime)
    ));
    assert_eq!(report.snapshot, before);
    assert_eq!((report.mcycles, report.ticks), (0, 0));
}

struct Scratch(PathBuf);

impl Scratch {
    fn new() -> Self {
        static NEXT_ID: AtomicU64 = AtomicU64::new(0);
        Self(std::env::temp_dir().join(format!(
            "gameboy-emu-runner-{}-{}.gb",
            std::process::id(),
            NEXT_ID.fetch_add(1, Ordering::Relaxed)
        )))
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

#[test]
fn runner_file_setup_errors_are_visible_and_valid_files_use_the_same_execution_path() {
    let scratch = Scratch::new();
    let error = runner::run_file(&scratch.0, limits(100)).unwrap_err();
    assert!(error.contains("FileIO"));
    assert!(error.contains(&scratch.0.display().to_string()));
    fs::write(&scratch.0, [0, 1, 2]).unwrap();
    assert!(
        runner::run_file(&scratch.0, limits(100))
            .unwrap_err()
            .contains("IncompleteHeader")
    );
    fs::write(
        &scratch.0,
        rom(&report_program(PASS_SIGNATURE, &PASS_SIGNATURE, 1)),
    )
    .unwrap();
    let report = runner::run_file(&scratch.0, limits(100)).unwrap();
    assert!(matches!(report.outcome, Outcome::Passed), "{report:#?}");
    assert_eq!(report.fixture, scratch.0.display().to_string());
    assert_eq!((report.mcycles, report.ticks), (43, 172));
}
