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
    system::{System, SystemError},
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
    let started = std::time::Instant::now();
    let report = runner::run_file(&path, runner::REG_F_LIMITS)
        .unwrap_or_else(|error| panic!("external fixture setup failed: {error}"));
    eprintln!(
        "Mooneye reg_f: {:?}, {} M-cycles, {} ticks, host {:?}, packet {:?}, CPU {:?}",
        report.outcome,
        report.mcycles,
        report.ticks,
        started.elapsed(),
        report.packet,
        report.snapshot
    );
    assert!(matches!(report.outcome, Outcome::Passed), "{report:#?}");
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
        ..runner::REG_F_LIMITS
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
