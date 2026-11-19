//! Test-only observer for the audited Mooneye reg_f fast-report path.
//! This observes requested FF01 writes; it does not emulate serial hardware.
use std::{
    collections::VecDeque,
    path::Path,
    time::{Duration, Instant},
};

use gameboy_emu::{
    cpu::{CpuSnapShot, CycleAction},
    memory::Cartridge,
    system::{CycleTrace, MachineStep, System, SystemError},
};

pub const PASS_SIGNATURE: [u8; 6] = [3, 5, 8, 13, 21, 34];
pub const FAIL_SIGNATURE: [u8; 6] = [0x42; 6];
pub const TRACE_CAPACITY: usize = 64;

#[derive(Debug, Clone, Copy)]
pub struct Limits {
    pub mcycles: u64,
    pub wall_time: Duration,
}

pub const REG_F_LIMITS: Limits = Limits {
    mcycles: 1_000_000,
    wall_time: Duration::from_secs(10),
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Timeout {
    CycleBudget,
    WallTime,
}

#[derive(Debug, Clone, Copy)]
pub enum Outcome {
    Passed,
    ReportedFailure,
    MalformedReport,
    TimedOut(Timeout),
    UnexpectedStop,
    ExecutionError(SystemError),
}

#[derive(Debug)]
pub struct Report {
    pub fixture: String,
    pub outcome: Outcome,
    pub packet: Vec<u8>,
    pub mcycles: u64,
    pub ticks: u64,
    pub snapshot: CpuSnapShot,
    pub recent: VecDeque<CycleTrace>,
}

/// Fixture provenance and SHA-256 must be checked during preparation, before
/// calling this. A missing/invalid fixture is an error, never a skipped pass.
pub fn run_file(path: &Path, limits: Limits) -> Result<Report, String> {
    let cartridge = Cartridge::from_filepath(path)
        .map_err(|error| format!("fixture {}: {error:?}", path.display()))?;
    Ok(run(
        &mut System::new(cartridge),
        path.display().to_string(),
        limits,
    ))
}

pub fn run(system: &mut System, fixture: impl Into<String>, limits: Limits) -> Report {
    let fixture = fixture.into();
    let started = Instant::now();
    let starting_ticks = system.elapsed_ticks();
    let mut mcycles = 0;
    let mut packet = Vec::with_capacity(6);
    let mut recent = VecDeque::with_capacity(TRACE_CAPACITY);
    let outcome = loop {
        if mcycles == limits.mcycles {
            break Outcome::TimedOut(Timeout::CycleBudget);
        }
        if started.elapsed() >= limits.wall_time {
            break Outcome::TimedOut(Timeout::WallTime);
        }
        match system.step_machine_cycle() {
            Ok(MachineStep::Stopped) => break Outcome::UnexpectedStop,
            Err(error) => {
                // Completion errors consumed a real cycle. Its record remains
                // in SystemError; request errors consume no time or cycle.
                if matches!(error, SystemError::CpuCompletionError(..)) {
                    mcycles += 1;
                }
                break Outcome::ExecutionError(error);
            }
            Ok(MachineStep::Cycle(trace)) => {
                mcycles += 1;
                if recent.len() == TRACE_CAPACITY {
                    recent.pop_front();
                }
                recent.push_back(trace);
                if let CycleAction::Write {
                    addr: 0xFF01,
                    value,
                } = trace.record.action
                {
                    packet.push(value);
                    if packet.len() == 6 {
                        let snapshot = system.cpu_snapshot();
                        let [b, c] = snapshot.bc.to_be_bytes();
                        let [d, e] = snapshot.de.to_be_bytes();
                        let [h, l] = snapshot.hl.to_be_bytes();
                        let signature = [b, c, d, e, h, l];
                        break if packet == PASS_SIGNATURE && signature == PASS_SIGNATURE {
                            Outcome::Passed
                        } else if packet == FAIL_SIGNATURE && signature == FAIL_SIGNATURE {
                            Outcome::ReportedFailure
                        } else {
                            // The first packet decides the result. Never scan
                            // onward for an accidental later passing packet.
                            Outcome::MalformedReport
                        };
                    }
                }
            }
        }
    };
    Report {
        fixture,
        outcome,
        packet,
        mcycles,
        ticks: system.elapsed_ticks() - starting_ticks,
        snapshot: system.cpu_snapshot(),
        recent,
    }
}
