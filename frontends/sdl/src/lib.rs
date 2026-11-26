use std::{
    error::Error,
    path::{Path, PathBuf},
    sync::mpsc::{self, Receiver, Sender},
    time::{Duration, Instant},
};

use gameboy_emu::{memory::Cartridge, video::Frame};
use sdl3::{
    EventPump, Sdl,
    dialog::{DialogError, DialogFileFilter, show_open_file_dialog},
    event::WindowEvent,
    keyboard::{Mod, Scancode},
    mouse::MouseButton,
    pixels::{Color, PixelFormat},
    render::{Canvas, FRect, Texture},
    video::Window,
};

use crate::{
    display::{RGBA_LEN, integer_viewport},
    input::Input,
    layout::{Label, PickerLayout, text_scale},
    session::Session,
};

mod display;
mod input;
mod layout;
mod pacing;
mod session;

pub use sdl3::event::Event as Sdl3Event;

enum Screen {
    Picker { error: Option<String> },
    Running(Box<Session>),
}

impl Screen {
    fn handle_input(
        &mut self,
        event: &sdl3::event::Event,
        input: &mut Input<Scancode>,
        size: (u32, u32),
    ) -> Option<Action> {
        match self {
            Screen::Picker { error } => match event {
                Sdl3Event::MouseButtonDown {
                    mouse_btn: MouseButton::Left,
                    x,
                    y,
                    ..
                } => {
                    let layout = PickerLayout::new(size.0, size.1, error.as_deref());
                    layout
                        .button
                        .area
                        .contains(*x, *y)
                        .then_some(Action::OpenDialog)
                }
                Sdl3Event::KeyDown {
                    scancode: Some(Scancode::Return | Scancode::O),
                    ..
                } => Some(Action::OpenDialog),
                _ => None,
            },
            Screen::Running(session) => match event {
                Sdl3Event::KeyDown {
                    scancode: Some(Scancode::Escape),
                    ..
                } => Some(Action::ToPicker),
                event => {
                    session.handle_input(event, input);
                    None
                }
            },
        }
    }

    fn update(&mut self) -> Result<(), Box<dyn Error>> {
        match self {
            Screen::Picker { .. } => Ok(()),
            Screen::Running(session) => session.update(),
        }
    }

    fn wake_at(&self) -> Option<Instant> {
        match self {
            Screen::Picker { .. } => None,
            Screen::Running(session) => session.wake_at(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Action {
    Quit,
    Load(PathBuf),
    ToPicker,
    OpenDialog,
}

enum ControlFlow {
    Quit,
    ChangeScreen(Screen),
    Continue,
}

enum DialogOutcome {
    Chosen(PathBuf),
    Cancelled,
    Failed(String),
}

pub struct Application {
    _sdl: Sdl,
    canvas: Canvas<Window>,
    event_pump: EventPump,
    input: Input<Scancode>,
    rgba: [u8; RGBA_LEN],
    screen: Screen,
    dialog_sx: Sender<DialogOutcome>,
    dialog_rx: Receiver<DialogOutcome>,
    dialog_open: bool,
    resume_after_dialog: bool,
}

impl Application {
    pub fn new() -> Result<Self, Box<dyn Error>> {
        let sdl_context = sdl3::init()?;
        let video_subsystem = sdl_context.video()?;

        let mut window = video_subsystem
            .window("gameboy-emu", 640, 576)
            .position_centered()
            .resizable()
            .build()?;

        window.set_minimum_size(160, 144)?;

        let canvas = window.into_canvas();

        let rgba = [0xFFu8; RGBA_LEN];

        let event_pump = sdl_context.event_pump()?;
        let input_handler = Input::new(crate::input::DEFAULT_BINDINGS);

        let (dialog_sx, dialog_rx) = mpsc::channel();

        Ok(Self {
            _sdl: sdl_context,
            canvas,
            event_pump,
            input: input_handler,
            rgba,
            screen: Screen::Picker { error: None },
            dialog_sx,
            dialog_rx,
            dialog_open: false,
            resume_after_dialog: false,
        })
    }

    pub fn with_rom_path(rom_path: impl AsRef<Path>) -> Result<Self, Box<dyn Error>> {
        let mut new_app = Self::new()?;
        let screen = Application::load(&rom_path);
        new_app.set_screen(screen);

        Ok(new_app)
    }

    pub fn run(&mut self) -> Result<(), Box<dyn Error>> {
        let creator = self.canvas.texture_creator();
        let mut texture = creator.create_texture_streaming(PixelFormat::RGBA32, 160, 144)?;
        texture.set_scale_mode(sdl3::render::ScaleMode::Nearest);

        'running: loop {
            while let Some(event) = self.event_pump.poll_event() {
                if !self.keep_running(event) {
                    break 'running;
                }
            }

            while let Ok(outcome) = self.dialog_rx.try_recv() {
                match outcome {
                    DialogOutcome::Chosen(path_buf) => {
                        let new_screen = Self::load(&path_buf);
                        self.set_screen(new_screen);
                    }
                    DialogOutcome::Cancelled => {
                        if let Screen::Running(session) = &mut self.screen
                            && self.resume_after_dialog
                        {
                            session.unpause();
                        }
                    }
                    DialogOutcome::Failed(error) => match &mut self.screen {
                        Screen::Picker { .. } => {
                            let screen = Screen::Picker { error: Some(error) };
                            self.set_screen(screen);
                        }
                        Screen::Running(session) => {
                            eprintln!("{error}");
                            if self.resume_after_dialog {
                                session.unpause();
                            }
                        }
                    },
                }

                self.dialog_open = false;
                self.resume_after_dialog = false;
            }

            match self.screen.update() {
                Ok(_) => {}
                Err(e) => {
                    let error = Some(format!("{e}"));
                    self.set_screen(Screen::Picker { error });
                }
            }

            self.draw(&mut texture)?;

            match self.screen.wake_at() {
                Some(time) => std::thread::sleep(time.saturating_duration_since(Instant::now())),
                None => {
                    if let Some(event) = self
                        .event_pump
                        .wait_event_timeout(Duration::from_millis(50))
                        && !self.keep_running(event)
                    {
                        break 'running;
                    }
                }
            }
        }

        Ok(())
    }

    fn handle_event(&mut self, mut event: Sdl3Event) -> ControlFlow {
        event.convert_coords(&self.canvas);
        let action = match event {
            Sdl3Event::Quit { .. } => Some(Action::Quit),
            Sdl3Event::DropFile { filename, .. } => Some(Action::Load(filename.into())),
            Sdl3Event::Window {
                win_event: WindowEvent::FocusLost,
                ..
            } => {
                self.release_all_buttons();
                None
            }
            Sdl3Event::KeyDown {
                scancode: Some(Scancode::O),
                keymod,
                repeat: false,
                ..
            } if keymod.intersects(Mod::LCTRLMOD | Mod::RCTRLMOD) => Some(Action::OpenDialog),
            other => self.screen.handle_input(
                &other,
                &mut self.input,
                self.canvas.output_size().unwrap_or((0, 0)),
            ),
        };

        if let Some(action) = action {
            match action {
                Action::Quit => ControlFlow::Quit,
                Action::Load(path_buf) => {
                    let screen = Application::load(&path_buf);
                    ControlFlow::ChangeScreen(screen)
                }
                Action::ToPicker => ControlFlow::ChangeScreen(Screen::Picker { error: None }),
                Action::OpenDialog => {
                    // open dialog.
                    if !self.dialog_open {
                        let sender = self.dialog_sx.clone();
                        if let Err(err) = show_open_file_dialog(
                            &[
                                DialogFileFilter {
                                    name: "Game Boy ROM",
                                    pattern: "gb;gbc",
                                },
                                DialogFileFilter {
                                    name: "All files",
                                    pattern: "*",
                                },
                            ],
                            None::<PathBuf>,
                            false,
                            self.canvas.window(),
                            Box::new(move |result, _| match result {
                                Ok(paths) => match paths.into_iter().next() {
                                    Some(path) => {
                                        let _ = sender.send(DialogOutcome::Chosen(path));
                                    }
                                    None => {
                                        let _ = sender.send(DialogOutcome::Cancelled);
                                    }
                                },
                                Err(DialogError::Canceled) => {
                                    let _ = sender.send(DialogOutcome::Cancelled);
                                }
                                Err(err) => {
                                    let error = format!("{err}");
                                    let _ = sender.send(DialogOutcome::Failed(error));
                                }
                            }),
                        ) {
                            let error = format!("{err}");
                            match self.screen {
                                Screen::Picker { .. } => {
                                    let screen = Screen::Picker { error: Some(error) };
                                    ControlFlow::ChangeScreen(screen)
                                }
                                Screen::Running(_) => {
                                    eprintln!("{error}");
                                    ControlFlow::Continue
                                }
                            }
                        } else {
                            self.dialog_open = true;
                            if let Screen::Running(session) = &mut self.screen
                                && !session.is_paused()
                            {
                                session.pause();
                                self.resume_after_dialog = true;
                            }
                            ControlFlow::Continue
                        }
                    } else {
                        ControlFlow::Continue
                    }
                }
            }
        } else {
            ControlFlow::Continue
        }
    }

    fn draw(&mut self, texture: &mut Texture<'_>) -> Result<(), Box<dyn Error>> {
        match &mut self.screen {
            Screen::Picker { error } => {
                self.canvas.set_draw_color(Color::RGB(255, 255, 255));
                self.canvas.clear();

                let (w, h) = self.canvas.output_size()?;
                let layout = PickerLayout::new(w, h, error.as_deref());
                layout.draw(&mut self.canvas)?;
            }
            Screen::Running(session) => {
                self.canvas.set_draw_color(Color::RGB(0, 0, 0));
                self.canvas.clear();

                if session.take_frame(&mut self.rgba) {
                    texture.update(None, &self.rgba, Frame::WIDTH * 4)?;
                }

                let (w, h) = self.canvas.output_size()?;

                let (x, y, vw, vh) = integer_viewport(w, h);
                self.canvas.copy(
                    texture,
                    None,
                    FRect::new(x as f32, y as f32, vw as f32, vh as f32),
                )?;

                if session.is_paused() {
                    const PAUSED: &str = "PAUSED";
                    let scale = text_scale(w, h);
                    let paused_box =
                        Label::new(PAUSED, scale, 20).centered_on((w as f32 / 2.0, h as f32 / 2.0));

                    self.canvas.set_draw_color(Color::WHITE);
                    self.canvas
                        .fill_rect(std::convert::Into::<FRect>::into(paused_box.area))?;

                    paused_box.draw(&mut self.canvas)?;
                }
            }
        }

        self.canvas.present();

        Ok(())
    }

    fn load(rom_path: &impl AsRef<Path>) -> Screen {
        let cartridge = match Cartridge::from_filepath(rom_path) {
            Ok(cart) => cart,
            Err(err) => {
                let error = format!("{err}");
                return Screen::Picker { error: Some(error) };
            }
        };

        let path = rom_path.as_ref();
        let name = path
            .file_stem()
            .unwrap_or(path.as_os_str())
            .to_string_lossy()
            .into_owned();

        let session = Session::new(cartridge, name);
        Screen::Running(Box::new(session))
    }

    fn set_screen(&mut self, screen: Screen) {
        match &screen {
            Screen::Picker { .. } => {
                let _ = self.canvas.window_mut().set_title("gameboy-emu");
            }
            Screen::Running(session) => {
                let _ = self
                    .canvas
                    .window_mut()
                    .set_title(&format!("{} - gameboy-emu", session.name()));
            }
        }
        self.release_all_buttons();
        self.screen = screen;
    }

    fn release_all_buttons(&mut self) {
        let buttons = self.input.release_all();
        if let Screen::Running(ref mut session) = self.screen {
            session.release_buttons(&buttons);
        }
    }

    fn keep_running(&mut self, event: Sdl3Event) -> bool {
        match self.handle_event(event) {
            ControlFlow::Quit => false,
            ControlFlow::ChangeScreen(screen) => {
                self.set_screen(screen);
                true
            }
            ControlFlow::Continue => true,
        }
    }
}
