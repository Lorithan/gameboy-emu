//! #81: the host-facing frame API is public (docs/tasks/RENDERING.md, host
//! interface). Behaviour is covered by system::tests::rendering.

use gameboy_emu::{
    memory::Cartridge,
    system::{MachineStep, System},
    video::{Frame, Shade},
};

#[test]
fn a_host_can_read_the_published_frame_through_the_public_api() {
    let mut rom = vec![0; 32_768];
    rom[0x14D] = 0xE7;
    rom[0x100] = 0x18; // JR -2
    rom[0x101] = 0xFE;
    let mut system = System::new(Cartridge::from_bytes(&rom).unwrap());
    assert_eq!(system.frames_completed(), 0);
    // The first VBlank after boot is under one frame (17,556 M-cycles) away.
    for _ in 0..17_556 {
        if system.frames_completed() != 0 {
            break;
        }
        assert!(matches!(
            system.step_machine_cycle().unwrap(),
            MachineStep::Cycle(_)
        ));
    }
    assert_eq!(system.frames_completed(), 1);
    // Post-boot VRAM is zero and BGP index 0 is White.
    let frame: &Frame = system.frame();
    assert_eq!(frame.pixels().len(), Frame::WIDTH * Frame::HEIGHT);
    assert_eq!(
        frame.pixel(Frame::WIDTH - 1, Frame::HEIGHT - 1),
        Shade::White
    );
    let shades = [
        Shade::White,
        Shade::LightGray,
        Shade::DarkGray,
        Shade::Black,
    ];
    assert_eq!(shades.len(), 4);
}
