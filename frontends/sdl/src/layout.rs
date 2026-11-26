use std::borrow::Cow;

use sdl3::{
    pixels::Color,
    render::{Canvas, FRect},
    video::Window,
};

#[cfg(test)]
mod tests;

pub const CELL: f32 = 8.0; // the debug font's character size
pub const PAD_X: f32 = CELL; // one character either side
pub const PAD_Y: f32 = CELL / 2.0; // half a character above and below
pub const LINE: f32 = CELL + PAD_Y; // one line plus the gap below it

pub const OPEN_LABEL: &str = "Open ROM";
const FILE_HINT: &str = "or drop a ROM here";

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Area {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl From<Area> for FRect {
    fn from(value: Area) -> Self {
        FRect {
            x: value.x,
            y: value.y,
            w: value.w,
            h: value.h,
        }
    }
}

impl Area {
    pub fn contains(&self, x: f32, y: f32) -> bool {
        x >= self.x && x < self.x + self.w && y >= self.y && y < self.y + self.h
    }
}

pub fn text_size(text: &str, scale: f32) -> (f32, f32) {
    let len = text.chars().count() as f32;

    let w = CELL * len * scale;
    let h = CELL * scale;
    (w, h)
}

pub fn text_box(lines: &[Cow<'_, str>], position: (f32, f32), scale: f32) -> Area {
    let w = lines
        .iter()
        .map(|text| text_size(text, scale).0)
        .reduce(f32::max)
        .unwrap_or(0.0);

    let w = w + (2.0 * PAD_X * scale);
    let h = lines.len() as f32 * LINE * scale + PAD_Y * scale;

    let x = position.0 - (w / 2.0);
    let y = position.1 - (h / 2.0);

    Area { x, y, w, h }
}

pub struct Label<'a> {
    pub lines: Vec<Cow<'a, str>>,
    pub max_chars: usize,
    pub area: Area,
    pub text_at: (f32, f32),
    pub scale: f32,

    // colors
    pub text_color: Color,
    pub bg_color: Option<Color>,
    pub border_color: Option<Color>,
}

impl<'a> Label<'a> {
    pub fn new(text: &'a str, scale: f32, max_chars: usize, text_color: Color) -> Self {
        let lines = wrap_text(text, max_chars)
            .into_iter()
            .map(Cow::Borrowed)
            .collect::<Vec<Cow<'a, str>>>();
        let area = text_box(&lines, (0.0, 0.0), scale);

        Self {
            lines,
            max_chars,
            area,
            text_at: (0.0, 0.0),
            scale,
            text_color,
            bg_color: None,
            border_color: None,
        }
    }

    pub fn limited_to(mut self, max_lines: usize) -> Self {
        if self.lines.len() > max_lines {
            self.lines.truncate(max_lines);
            if let Some(last) = self.lines.pop() {
                let kept: String = last
                    .chars()
                    .take(self.max_chars.saturating_sub(3))
                    .collect();

                self.lines.push(Cow::Owned(format!("{kept}...")));
            }
            self.area = text_box(&self.lines, (0.0, 0.0), self.scale);
        }
        self
    }

    pub fn centered_on(mut self, center: (f32, f32)) -> Self {
        self.area.x = (center.0 - self.area.w / 2.0).floor();
        self.area.y = (center.1 - self.area.h / 2.0).floor();
        self.text_at = (
            self.area.x + PAD_X * self.scale,
            self.area.y + PAD_Y * self.scale,
        );
        self
    }

    pub fn with_background(mut self, bg_color: Color) -> Self {
        self.bg_color = Some(bg_color);
        self
    }

    pub fn with_border(mut self, border_color: Color) -> Self {
        self.border_color = Some(border_color);
        self
    }

    pub fn draw(&self, canvas: &mut Canvas<Window>) -> Result<(), sdl3::Error> {
        if let Some(color) = self.bg_color {
            canvas.set_draw_color(color);
            canvas.fill_rect(std::convert::Into::<FRect>::into(self.area))?;
        }

        if let Some(color) = self.border_color {
            canvas.set_draw_color(color);
            canvas.draw_rect(self.area)?;
        }

        canvas.set_draw_color(self.text_color);

        for (i, line) in self.lines.iter().enumerate() {
            let y = self.text_at.1 + (LINE * self.scale * i as f32);
            draw_text(canvas, line, (self.text_at.0, y), self.scale)?;
        }

        Ok(())
    }
}

pub struct PickerLayout<'a> {
    pub button: Label<'static>,
    pub hint: Label<'static>,
    pub error: Option<Label<'a>>,
}

impl<'a> PickerLayout<'a> {
    pub fn new(width: u32, height: u32, error: Option<&'a str>) -> PickerLayout<'a> {
        let scale = text_scale(width, height);
        let (width, height) = (width as f32, height as f32);
        let max_chars = ((width - 2.0 * PAD_X * scale) / (CELL * scale)).floor() as usize;
        let center_x = width / 2.0;
        let gap = CELL * scale;

        let button = Label::new(OPEN_LABEL, scale, max_chars, Color::BLACK)
            .with_background(Color::GRAY)
            .with_border(Color::BLACK)
            .centered_on((center_x, height / 2.0));

        let hint = Label::new(FILE_HINT, scale, max_chars, Color::GRAY);
        let hint_y = button.area.y + button.area.h + gap + hint.area.h / 2.0;
        let hint = hint.centered_on((center_x, hint_y));

        let error = error.map(|text| {
            let available = button.area.y - gap;
            let line = LINE * scale;
            let max_lines = (((available - PAD_Y * scale) / line).floor() as usize).max(1);

            let label = Label::new(text, scale, max_chars, Color::BLACK).limited_to(max_lines);
            let centre_y = button.area.y - gap - label.area.h / 2.0;
            label.centered_on((center_x, centre_y))
        });

        Self {
            button,
            hint,
            error,
        }
    }

    pub fn draw(&self, canvas: &mut Canvas<Window>) -> Result<(), sdl3::Error> {
        self.button.draw(canvas)?;

        self.hint.draw(canvas)?;

        if let Some(ref error_label) = self.error {
            error_label.draw(canvas)?;
        }

        Ok(())
    }
}

pub fn text_scale(width: u32, height: u32) -> f32 {
    let game = (width / 160).min(height / 144); // the game's integer scale
    (game / 2).max(1) as f32
}

fn draw_text(
    canvas: &mut Canvas<Window>,
    text: &str,
    at: (f32, f32),
    scale: f32,
) -> Result<(), sdl3::Error> {
    canvas.set_scale(scale, scale)?;
    let drawn = canvas.draw_debug_text(text, (at.0 / scale, at.1 / scale));
    canvas.set_scale(1.0, 1.0)?;
    drawn
}

fn wrap_text(text: &str, max_chars: usize) -> Vec<&str> {
    if max_chars == 0 || text.is_empty() {
        return Vec::new();
    }

    let mut lines = Vec::new();
    let mut rest = text.trim_start();

    while !rest.is_empty() {
        let mut line_end = 0;
        let mut last_space_end = None;

        for (char_count, (idx, ch)) in rest.char_indices().enumerate() {
            if ch.is_whitespace() {
                last_space_end = Some(idx);
            }

            if char_count == max_chars {
                break;
            }

            line_end = idx + ch.len_utf8();
        }

        // If we hit max_chars and there was a whitespace boundary before the cutoff,
        // wrap at the whitespace unless the next character is already whitespace.
        let next_is_ws = rest[line_end..].starts_with(char::is_whitespace);
        let more_after = line_end < rest.len();
        let split_at = if more_after && !next_is_ws {
            last_space_end.unwrap_or(line_end)
        } else {
            line_end
        };

        let line = rest[..split_at].trim_end();
        if !line.is_empty() {
            lines.push(line);
        }

        rest = rest[split_at..].trim_start();
    }

    lines
}
