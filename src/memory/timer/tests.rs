use super::Timer;

#[test]
fn divider_reads_and_one_tick_exhaust_all_counter_states() {
    for counter in 0u16..=u16::MAX {
        let mut timer = Timer { counter };
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
        let mut written = Timer { counter };
        // Across the sweep, every byte value is written at every upper-byte
        // state. The write value must never become the stored DIV value.
        written.write_div(counter as u8);
        assert_eq!(written.counter, 0);
        assert_eq!(written.read_div(), 0);
        let mut stopped = Timer { counter };
        stopped.on_stop_entered();
        assert_eq!(stopped.counter, 0);
        assert_eq!(stopped.read_div(), 0);
    }
}

#[test]
fn divider_reset_requires_256_new_base_ticks_before_visible_increment() {
    for counter in [1, 0xFF, 0x12FF, 0xABC8, 0xFFFF] {
        for value in 0u8..=255 {
            let mut timer = Timer { counter };
            timer.write_div(value);
            for ticks in 1u16..=256 {
                timer.tick();
                assert_eq!(timer.counter, ticks);
                assert_eq!(timer.read_div(), u8::from(ticks == 256));
            }
        }
        let mut timer = Timer { counter };
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
