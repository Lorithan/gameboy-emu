use super::Timer;

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
