// Memory-preservation regressions here perform no DIV writes or STOP entries.
// Recompute only the clocked DIV byte; retain every other storage oracle.
pub(super) fn clocked_byte(expected: u8, address: u16, elapsed_ticks: u64) -> u8 {
    if address == 0xFF04 {
        ((43_976 + elapsed_ticks % 65_536) % 65_536 / 256) as u8
    } else {
        expected
    }
}
