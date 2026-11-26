#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use std::error::Error;

use gameboy_sdl::Application;

fn main() -> Result<(), Box<dyn Error>> {
    let mut app = match std::env::args_os().nth(1) {
        Some(path) => Application::with_rom_path(path),
        None => Application::new(),
    }?;

    app.run()
}
