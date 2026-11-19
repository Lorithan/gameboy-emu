use super::{Timer, TimerEvent, TimerPhase};

#[test]
fn stop_reset_preserves_registers_and_every_phase_without_synthesizing_an_edge() {
    for counter in [0, 8, 0x200, 0xABC8, 0xFFFF] {
        for tac in 0..8 {
            for phase in [
                TimerPhase::Counting,
                TimerPhase::Overflowing { ticks_left: 1 },
                TimerPhase::Overflowing { ticks_left: 2 },
                TimerPhase::Overflowing { ticks_left: 3 },
                TimerPhase::Overflowing { ticks_left: 4 },
                TimerPhase::Reloading,
            ] {
                for tima in [0, 0x42, 255] {
                    let mut timer = Timer {
                        counter,
                        tima,
                        tma: 0x69,
                        tac,
                        phase,
                    };
                    let mut expected = timer;
                    expected.counter = 0;
                    timer.on_stop_entered();
                    assert_eq!(timer, expected);
                    timer.on_stop_entered();
                    assert_eq!(timer, expected);
                }
            }
        }
    }
}

#[test]
fn stop_retains_each_remaining_reload_delay_and_resumes_counting_from_zero() {
    for ticks_left in 1..=4 {
        let mut timer = Timer {
            counter: 0xFFFF,
            tima: 0,
            tma: 0x42,
            tac: 5,
            phase: TimerPhase::Overflowing { ticks_left },
        };
        timer.on_stop_entered();
        let paused = timer;
        for _ in 0..8 {
            assert_eq!(
                (
                    timer.read_div(),
                    timer.read_tima(),
                    timer.read_tma(),
                    timer.read_tac()
                ),
                (0, 0, 0x42, 0xFD)
            );
            assert_eq!(timer, paused);
        }
        for resumed_ticks in 1..=ticks_left {
            assert_eq!(
                timer.tick(),
                if resumed_ticks == ticks_left {
                    TimerEvent::InterruptRequested
                } else {
                    TimerEvent::None
                }
            );
            assert_eq!(timer.counter, u16::from(resumed_ticks));
            assert_eq!(
                timer.read_tima(),
                if resumed_ticks == ticks_left { 0x42 } else { 0 }
            );
        }
        assert_eq!(timer.phase, TimerPhase::Reloading);
        for resumed_ticks in ticks_left + 1..=16 {
            assert_eq!(timer.tick(), TimerEvent::None);
            assert_eq!(timer.counter, u16::from(resumed_ticks));
            assert_eq!(
                timer.read_tima(),
                if resumed_ticks == 16 { 0x43 } else { 0x42 }
            );
            assert_eq!(timer.phase, TimerPhase::Counting);
        }
    }
}

#[test]
fn div_write_edges_exhaust_counter_phases_and_control_settings() {
    for counter in 0u16..=u16::MAX {
        for (selection, half_period) in [512u32, 8, 32, 128].into_iter().enumerate() {
            for enabled in [false, true] {
                let tac = selection as u8 + if enabled { 4 } else { 0 };
                let mut timer = Timer {
                    counter,
                    tima: 0x42,
                    tma: 0xA5,
                    tac,
                    phase: TimerPhase::Counting,
                };
                let falling = enabled && (u32::from(counter) / half_period) % 2 == 1;
                timer.write_div(counter as u8);
                assert_eq!(timer.counter, 0);
                assert_eq!(timer.read_tima(), 0x42 + u8::from(falling));
                assert_eq!(
                    (timer.read_tma(), timer.tac, timer.phase),
                    (0xA5, tac, TimerPhase::Counting)
                );
                let reset = timer;
                timer.write_div(!(counter as u8));
                assert_eq!(
                    timer, reset,
                    "resetting zero again cannot create another pulse"
                );
            }
        }
    }
}

#[test]
fn tac_write_edges_exhaust_selected_bit_combinations_old_controls_and_written_bytes() {
    let half_periods = [512u32, 8, 32, 128];
    for combination in 0..16 {
        let counter = half_periods
            .iter()
            .enumerate()
            .fold(0x1001u16, |counter, (index, half)| {
                counter
                    + if combination & (1 << index) != 0 {
                        *half as u16
                    } else {
                        0
                    }
            });
        for old_control in 0u8..8 {
            for value in 0u8..=255 {
                let old_high = old_control >= 4
                    && (u32::from(counter) / half_periods[usize::from(old_control % 4)]) % 2 == 1;
                let new_control = value % 8;
                let new_high = new_control >= 4
                    && (u32::from(counter) / half_periods[usize::from(new_control % 4)]) % 2 == 1;
                let mut timer = Timer {
                    counter,
                    tima: 0x42,
                    tma: 0xA5,
                    tac: old_control,
                    phase: TimerPhase::Counting,
                };
                timer.write_tac(value);
                assert_eq!(
                    timer.read_tima(),
                    0x42 + u8::from(old_high && !new_high),
                    "counter {counter:04X}, old {old_control:02X}, write {value:02X}"
                );
                assert_eq!(
                    (timer.counter, timer.tac, timer.read_tma(), timer.phase),
                    (counter, new_control, 0xA5, TimerPhase::Counting)
                );
                let after = timer;
                timer.write_tac(value ^ 0xF8);
                assert_eq!(
                    timer, after,
                    "same effective control with different upper bits must be idempotent"
                );
            }
        }
    }
}

#[test]
fn write_edges_preserve_pending_and_reloading_phases_without_touching_tima() {
    for phase in [
        TimerPhase::Overflowing { ticks_left: 1 },
        TimerPhase::Overflowing { ticks_left: 2 },
        TimerPhase::Overflowing { ticks_left: 3 },
        TimerPhase::Overflowing { ticks_left: 4 },
        TimerPhase::Reloading,
    ] {
        for tima in [0, 0x42, 0xFF] {
            for operation in 0..6 {
                let mut timer = Timer {
                    counter: 8,
                    tima,
                    tma: 0xA5,
                    tac: 5,
                    phase,
                };
                let mut expected = timer;
                if operation < 2 {
                    timer.write_div(if operation == 0 { 0 } else { 255 });
                    expected.counter = 0;
                } else {
                    let value = [0, 4, 0xF8, 0xFC][operation - 2];
                    timer.write_tac(value);
                    expected.tac = value % 8;
                }
                assert_eq!(
                    timer, expected,
                    "phase {phase:?}, TIMA {tima:02X}, operation {operation}"
                );
            }
        }
    }
}

#[test]
fn write_created_overflow_waits_four_real_ticks_even_after_disabling() {
    for via_div in [false, true] {
        let mut timer = Timer {
            counter: 8,
            tima: 255,
            tma: 0x42,
            tac: 5,
            phase: TimerPhase::Counting,
        };
        if via_div {
            timer.write_div(0xA5);
        } else {
            timer.write_tac(0);
        }
        assert_eq!(timer.read_tima(), 0);
        assert_eq!(timer.phase, TimerPhase::Overflowing { ticks_left: 4 });
        for _ in 0..3 {
            assert_eq!(timer.tick(), TimerEvent::None);
            assert_eq!(timer.read_tima(), 0);
        }
        assert_eq!(timer.tick(), TimerEvent::InterruptRequested);
        assert_eq!(timer.read_tima(), 0x42);
        assert_eq!(timer.tick(), TimerEvent::None);
    }
}

#[test]
fn tima_writes_cancel_at_every_delay_tick_without_scheduling_another_reload() {
    for elapsed in 0..4 {
        for value in 0u8..=255 {
            let mut timer = Timer {
                counter: 15,
                tima: 255,
                tma: 0x42,
                tac: 5,
                phase: TimerPhase::Counting,
            };
            assert_eq!(timer.tick(), TimerEvent::None);
            timer.write_tac(0);
            for _ in 0..elapsed {
                assert_eq!(timer.tick(), TimerEvent::None);
            }
            let counter = timer.counter;
            timer.write_tima(value);
            assert_eq!(timer.counter, counter);
            assert_eq!(timer.read_tma(), 0x42);
            assert_eq!(timer.phase, TimerPhase::Counting);
            for ticks in 1..=8 {
                assert_eq!(timer.tick(), TimerEvent::None);
                assert_eq!(timer.read_tima(), value);
                assert_eq!(timer.counter, counter.wrapping_add(ticks));
            }
        }
    }
}

#[test]
fn tma_writes_during_every_delay_tick_change_only_the_eventual_reload_value() {
    for elapsed in 0..4 {
        for value in 0u8..=255 {
            let mut timer = Timer {
                counter: 15,
                tima: 255,
                tma: 0x42,
                tac: 5,
                phase: TimerPhase::Counting,
            };
            assert_eq!(timer.tick(), TimerEvent::None);
            timer.write_tac(0);
            for _ in 0..elapsed {
                assert_eq!(timer.tick(), TimerEvent::None);
            }
            let before = timer;
            timer.write_tma(value);
            assert_eq!(timer.counter, before.counter);
            assert_eq!(timer.phase, before.phase);
            assert_eq!(timer.read_tima(), 0);
            assert_eq!(timer.read_tma(), value);
            for remaining in (1..=4 - elapsed).rev() {
                let event = timer.tick();
                assert_eq!(
                    event,
                    if remaining == 1 {
                        TimerEvent::InterruptRequested
                    } else {
                        TimerEvent::None
                    }
                );
                assert_eq!(timer.read_tima(), if remaining == 1 { value } else { 0 });
            }
        }
    }
}

#[test]
fn reload_window_ignores_tima_and_copies_tma_writes_until_the_next_tick() {
    for original in [0, 0x42, 0xFF] {
        for value in 0u8..=255 {
            let mut timer = Timer {
                counter: 15,
                tima: 255,
                tma: original,
                tac: 5,
                phase: TimerPhase::Counting,
            };
            for _ in 0..4 {
                assert_eq!(timer.tick(), TimerEvent::None);
            }
            assert_eq!(timer.tick(), TimerEvent::InterruptRequested);
            let before = timer;
            timer.write_tima(value);
            assert_eq!(
                timer, before,
                "reload-window TIMA write must have no effects"
            );
            timer.write_tma(value);
            assert_eq!((timer.read_tima(), timer.read_tma()), (value, value));
            assert_eq!(timer.counter, before.counter);
            assert_eq!(timer.phase, TimerPhase::Reloading);
            assert_eq!(timer.tick(), TimerEvent::None);
            timer.write_tima(!value);
            assert_eq!((timer.read_tima(), timer.read_tma()), (!value, value));
            assert_eq!(timer.phase, TimerPhase::Counting);
        }
    }
}

#[test]
fn counting_ticks_increment_only_at_enabled_period_boundaries() {
    for (selection, period) in [1024u32, 16, 64, 256].into_iter().enumerate() {
        for enabled in [false, true] {
            let mut timer = Timer {
                counter: 0,
                tima: 0x20,
                tma: 0xA5,
                tac: selection as u8 + if enabled { 4 } else { 0 },
                phase: TimerPhase::Counting,
            };
            for ticks in 1..=period * 3 {
                assert!(matches!(timer.tick(), TimerEvent::None));
                assert_eq!(timer.counter, ticks as u16);
                assert_eq!(
                    timer.read_tima(),
                    0x20 + if enabled { (ticks / period) as u8 } else { 0 }
                );
                assert_eq!(timer.read_tma(), 0xA5);
                assert!(matches!(timer.phase, TimerPhase::Counting));
            }
        }
    }
}

#[test]
fn overflow_waits_four_further_ticks_then_reloads_current_tma_and_emits_once() {
    for (selection, period) in [1024u16, 16, 64, 256].into_iter().enumerate() {
        for counter in [period - 1, 0xFFFF] {
            for reload in [0, 0x42, 0xFF] {
                let mut timer = Timer {
                    counter,
                    tima: 0xFF,
                    tma: 0xA5,
                    tac: 4 + selection as u8,
                    phase: TimerPhase::Counting,
                };
                assert!(matches!(timer.tick(), TimerEvent::None));
                assert_eq!(timer.read_tima(), 0);
                assert!(matches!(
                    timer.phase,
                    TimerPhase::Overflowing { ticks_left: 4 }
                ));
                // Reload delay is elapsed time, independent of further edges
                // or the enable gate. Disabling must not cancel the request.
                timer.write_tac(0);
                for ticks_left in [3, 2, 1] {
                    assert!(matches!(timer.tick(), TimerEvent::None));
                    assert_eq!(timer.read_tima(), 0);
                    assert!(
                        matches!(timer.phase, TimerPhase::Overflowing { ticks_left: remaining } if remaining == ticks_left)
                    );
                }
                timer.write_tma(reload);
                assert!(matches!(timer.tick(), TimerEvent::InterruptRequested));
                assert_eq!(timer.read_tima(), reload);
                assert_eq!(timer.counter, counter.wrapping_add(5));
                assert!(matches!(timer.phase, TimerPhase::Reloading));
                assert!(matches!(timer.tick(), TimerEvent::None));
                assert_eq!(timer.counter, counter.wrapping_add(6));
                assert_eq!(timer.read_tima(), reload);
                assert!(matches!(timer.phase, TimerPhase::Counting));
                for _ in 0..8 {
                    assert!(matches!(timer.tick(), TimerEvent::None));
                }
            }
        }
    }
}

#[test]
fn timer_input_exhausts_enable_selection_and_divider_phase_without_mutation() {
    // Half-periods in base ticks from the four documented frequencies.
    // Arithmetic waveform oracle avoids repeating the production bit mask.
    for counter in 0u16..=u16::MAX {
        for (selection, half_period) in [512u32, 8, 32, 128].into_iter().enumerate() {
            for enabled in [false, true] {
                let tac = selection as u8 + if enabled { 4 } else { 0 };
                let timer = Timer {
                    counter,
                    tima: 0xA5,
                    tma: 0x5A,
                    tac,
                    phase: TimerPhase::Counting,
                };
                let expected = enabled && (u32::from(counter) / half_period) % 2 == 1;
                assert_eq!(
                    timer.timer_input(),
                    expected,
                    "counter {counter:04X}, TAC {tac:02X}"
                );
                assert_eq!(
                    (timer.counter, timer.tima, timer.tma, timer.tac),
                    (counter, 0xA5, 0x5A, tac)
                );
            }
        }
    }
}

#[test]
fn programmable_register_startup_and_reads_preserve_all_timer_state() {
    let mut timer = Timer::dmg_post_boot();
    assert_eq!(timer.counter, 0xABC8);
    assert_eq!(
        (timer.read_tima(), timer.read_tma(), timer.read_tac()),
        (0, 0, 0xF8)
    );
    for counter in [0, 7, 0xFF, 0xABC8, 0xFFFF] {
        for control in 0..8 {
            timer.counter = counter;
            timer.tima = 0x96;
            timer.tma = 0x69;
            timer.tac = control;
            for _ in 0..2 {
                assert_eq!(timer.read_div(), (u32::from(counter) / 256) as u8);
                assert_eq!(timer.read_tima(), 0x96);
                assert_eq!(timer.read_tma(), 0x69);
                assert_eq!(timer.read_tac(), 248 + control);
                assert_eq!(
                    (timer.counter, timer.tima, timer.tma, timer.tac),
                    (counter, 0x96, 0x69, control)
                );
            }
        }
    }
}

#[test]
fn programmable_register_writes_exhaust_bytes_without_changing_other_storage_or_phase() {
    for counter in [0, 0xFF, 0xABC8, 0xFFFF] {
        for value in 0u8..=255 {
            for target in 0..3 {
                let mut timer = Timer {
                    counter,
                    tima: 0xA5,
                    tma: 0x5A,
                    tac: 3,
                    phase: TimerPhase::Counting,
                };
                let expected = match target {
                    0 => {
                        timer.write_tima(value);
                        (value, 0x5A, 251)
                    }
                    1 => {
                        timer.write_tma(value);
                        (0xA5, value, 251)
                    }
                    _ => {
                        timer.write_tac(value);
                        (0xA5, 0x5A, 248 + value % 8)
                    }
                };
                assert_eq!(
                    (timer.read_tima(), timer.read_tma(), timer.read_tac()),
                    expected
                );
                assert_eq!(
                    timer.counter, counter,
                    "register write reset/advanced hidden DIV phase"
                );
                assert!(timer.tac < 8, "unused control bits leaked into storage");
            }
        }
    }
}

#[test]
fn divider_reads_and_one_tick_exhaust_all_counter_states() {
    for counter in 0u16..=u16::MAX {
        let mut timer = Timer {
            counter,
            tima: 0,
            tma: 0,
            tac: 0,
            phase: TimerPhase::Counting,
        };
        for _ in 0..2 {
            assert_eq!(timer.read_div(), (u32::from(counter) / 256) as u8);
            assert_eq!(timer.counter, counter, "DIV read changed phase");
        }
        timer.tick();
        let expected = (u32::from(counter) + 1) % 65_536;
        assert_eq!(u32::from(timer.counter), expected);
        assert_eq!(timer.read_div(), (expected / 256) as u8);
    }
}

#[test]
fn divider_write_and_stop_reset_clear_every_counter_phase() {
    for counter in 0u16..=u16::MAX {
        let mut written = Timer {
            counter,
            tima: 0,
            tma: 0,
            tac: 0,
            phase: TimerPhase::Counting,
        };
        // Across the sweep, every byte value is written at every upper-byte
        // state. The write value must never become the stored DIV value.
        written.write_div(counter as u8);
        assert_eq!(written.counter, 0);
        assert_eq!(written.read_div(), 0);
        let mut stopped = Timer {
            counter,
            tima: 0,
            tma: 0,
            tac: 0,
            phase: TimerPhase::Counting,
        };
        stopped.on_stop_entered();
        assert_eq!(stopped.counter, 0);
        assert_eq!(stopped.read_div(), 0);
    }
}

#[test]
fn divider_reset_requires_256_new_base_ticks_before_visible_increment() {
    for counter in [1, 0xFF, 0x12FF, 0xABC8, 0xFFFF] {
        for value in 0u8..=255 {
            let mut timer = Timer {
                counter,
                tima: 0,
                tma: 0,
                tac: 0,
                phase: TimerPhase::Counting,
            };
            timer.write_div(value);
            for ticks in 1u16..=256 {
                timer.tick();
                assert_eq!(timer.counter, ticks);
                assert_eq!(timer.read_div(), u8::from(ticks == 256));
            }
        }
        let mut timer = Timer {
            counter,
            tima: 0,
            tma: 0,
            tac: 0,
            phase: TimerPhase::Counting,
        };
        timer.on_stop_entered();
        for ticks in 1u16..=256 {
            timer.tick();
            assert_eq!(timer.counter, ticks);
            assert_eq!(timer.read_div(), u8::from(ticks == 256));
        }
    }
}

#[test]
fn divider_startup_phase_matches_sourced_boot_read_vectors() {
    let mut timer = Timer::dmg_post_boot();
    assert_eq!(timer.counter, 0xABC8);
    assert_eq!(timer.read_div(), 0xAB);
    // Expected observations are from pinned Mooneye boot_div-dmgABCmgb;
    // cumulative sample times translate its header/instructions to our model.
    let samples = [
        (14, 0xAC),
        (78, 0xAD),
        (141, 0xAD),
        (205, 0xAE),
        (269, 0xAF),
        (334, 0xB1),
    ];
    let mut elapsed = 0;
    for (mcycles, expected_div) in samples {
        while elapsed < mcycles * 4 {
            timer.tick();
            elapsed += 1;
        }
        assert_eq!(
            timer.read_div(),
            expected_div,
            "sample at {mcycles} M-cycles"
        );
    }
}
