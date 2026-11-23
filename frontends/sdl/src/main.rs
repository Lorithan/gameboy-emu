use std::{error::Error, time::Instant};

use gameboy_emu::{
    memory::Cartridge,
    system::{MachineStep, System},
    video::Frame,
};
use sdl3::{
    event::Event,
    keyboard::Keycode,
    pixels::{Color, PixelFormat},
    render::FRect,
};

use crate::{
    display::{RGBA_LEN, integer_viewport, write_rgba32},
    pacing::{MCYCLES_PER_FRAME, Pacer},
};

mod display;
mod pacing;

fn main() -> Result<(), Box<dyn Error>> {
    let rom_path = match std::env::args().nth(1) {
        Some(path) => path,
        None => {
            eprintln!("Usage: gameboy-sdl.exe <path>");
            std::process::exit(2);
        }
    };

    let cartridge = match Cartridge::from_filepath(rom_path) {
        Ok(cart) => cart,
        Err(err) => {
            eprintln!("{err}");
            std::process::exit(1);
        }
    };

    let mut system = System::new(cartridge);

    let sdl_context = sdl3::init()?;
    let video_subsystem = sdl_context.video()?;

    let window = video_subsystem
        .window("gameboy-emu", 640, 576)
        .position_centered()
        .resizable()
        .build()?;

    let mut canvas = window.into_canvas();

    let texture_creator = canvas.texture_creator();
    let mut texture = texture_creator.create_texture_streaming(PixelFormat::RGBA32, 160, 144)?;
    texture.set_scale_mode(sdl3::render::ScaleMode::Nearest);

    let mut rgba_buffer = [0u8; RGBA_LEN];
    let mut pacer = Pacer::new(Instant::now());
    let mut last_frame = system.frames_completed();

    let mut event_pump = sdl_context.event_pump()?;

    let start_up = Instant::now();
    let mut frame_count = 0;

    'running: loop {
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => break 'running,
                _ => {}
            }
        }

        let frames = pacer.frames_due(Instant::now());

        'frames: for _ in 0..frames {
            frame_count += 1;
            for _ in 0..MCYCLES_PER_FRAME {
                match system.step_machine_cycle() {
                    Ok(MachineStep::Cycle(_)) => {} // continue
                    Ok(MachineStep::Stopped) => {
                        break 'frames;
                    }
                    Err(e) => {
                        eprintln!("{e}");
                        std::process::exit(1);
                    }
                }
            }
        }

        if last_frame != system.frames_completed() {
            last_frame = system.frames_completed();
            write_rgba32(system.frame(), &mut rgba_buffer);
            texture.update(None, &rgba_buffer, Frame::WIDTH * 4)?;
        }

        canvas.set_draw_color(Color::RGB(0, 0, 0));
        canvas.clear();
        let (w, h) = canvas.output_size()?;

        let (x, y, vw, vh) = integer_viewport(w, h);
        canvas.copy(
            &texture,
            None,
            FRect::new(x as f32, y as f32, vw as f32, vh as f32),
        )?;
        canvas.present();

        std::thread::sleep(pacer.next_due().saturating_duration_since(Instant::now()));
    }

    let elapsed = start_up.elapsed();
    let secs = elapsed.as_secs_f64();
    eprintln!(
        "frames {frame_count} in {secs:.2} s ({:.2}/s)",
        frame_count as f64 / secs
    );
    Ok(())
}
