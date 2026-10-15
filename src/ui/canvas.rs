use crate::core::plot::{BRAILLE_BASE, DOT_BITS};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Color {
    Default,
    Black,
    White,
    BrightBlack,
    BrightRed,
    BrightGreen,
    BrightYellow,
    BrightBlue,
    BrightMagenta,
    BrightCyan,
    BrightWhite,
}

impl Color {
    fn fg_code(self) -> u8 {
        match self {
            Color::Default => 39,
            Color::Black => 30,
            Color::White => 37,
            Color::BrightBlack => 90,
            Color::BrightRed => 91,
            Color::BrightGreen => 92,
            Color::BrightYellow => 93,
            Color::BrightBlue => 94,
            Color::BrightMagenta => 95,
            Color::BrightCyan => 96,
            Color::BrightWhite => 97,
        }
    }

    fn bg_code(self) -> u8 {
        match self {
            Color::Default => 49,
            Color::Black => 40,
            Color::White => 47,
            Color::BrightBlack => 100,
            Color::BrightRed => 101,
            Color::BrightGreen => 102,
            Color::BrightYellow => 103,
            Color::BrightBlue => 104,
            Color::BrightMagenta => 105,
            Color::BrightCyan => 106,
            Color::BrightWhite => 107,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Style {
    pub fg: Color,
    pub bg: Color,
    pub bold: bool,
}

impl Style {
    pub const fn new() -> Self {
        Style {
            fg: Color::Default,
            bg: Color::Default,
            bold: false,
        }
    }

    pub const fn fg(color: Color) -> Self {
        Style {
            fg: color,
            ..Style::new()
        }
    }

    pub const fn on(mut self, background: Color) -> Self {
        self.bg = background;
        self
    }

    pub const fn bold(mut self) -> Self {
        self.bold = true;
        self
    }
}

#[derive(Clone, Copy)]
struct Cell {
    ch: char,
    style: Style,
}

pub struct Canvas {
    width: usize,
    height: usize,
    cells: Vec<Cell>,
}

impl Canvas {
    pub fn new(width: usize, height: usize) -> Self {
        let blank = Cell {
            ch: ' ',
            style: Style::new(),
        };

        Canvas {
            width,
            height,
            cells: vec![blank; width * height],
        }
    }

    pub fn put(&mut self, x: isize, y: isize, ch: char, style: Style) {
        if x < 0 || y < 0 {
            return;
        }

        let (x, y) = (x as usize, y as usize);

        if x >= self.width || y >= self.height {
            return;
        }

        self.cells[y * self.width + x] = Cell { ch, style };
    }

    pub fn print(&mut self, x: isize, y: isize, text: &str, style: Style) {
        let mut cursor = x;

        for ch in text.chars() {
            self.put(cursor, y, ch, style);
            cursor += 1;
        }
    }

    pub fn print_clipped(
        &mut self,
        x: isize,
        y: isize,
        max_width: usize,
        text: &str,
        style: Style,
    ) {
        if max_width == 0 {
            return;
        }

        let chars: Vec<char> = text.chars().collect();

        if chars.len() <= max_width {
            self.print(x, y, text, style);
            return;
        }

        let mut clipped: String = chars[..max_width - 1].iter().collect();

        clipped.push('…');
        self.print(x, y, &clipped, style);
    }

    pub fn draw_box(
        &mut self,
        x: isize,
        y: isize,
        width: usize,
        height: usize,
        style: Style,
        title: Option<&str>,
    ) {
        if width < 2 || height < 2 {
            return;
        }

        let right = x + width as isize - 1;
        let bottom = y + height as isize - 1;

        self.put(x, y, '┌', style);
        self.put(right, y, '┐', style);
        self.put(x, bottom, '└', style);
        self.put(right, bottom, '┘', style);

        for column in 1..width - 1 {
            self.put(x + column as isize, y, '─', style);
            self.put(x + column as isize, bottom, '─', style);
        }

        for row in 1..height - 1 {
            self.put(x, y + row as isize, '│', style);
            self.put(right, y + row as isize, '│', style);
        }

        if let Some(title) = title {
            let label = format!(" {} ", title);

            self.print_clipped(x + 2, y, width.saturating_sub(4), &label, style.bold());
        }
    }

    // True if the cell already holds a braille character (i.e. a curve point)
    pub fn has_braille(&self, x: usize, y: usize) -> bool {
        if x >= self.width || y >= self.height {
            return false;
        }

        let existing = self.cells[y * self.width + x].ch as u32;

        (BRAILLE_BASE..=BRAILLE_BASE + 0xFF).contains(&existing)
    }

    // Sets a dot in the 2x4 braille cell at (x, y)
    pub fn put_braille(
        &mut self,
        x: usize,
        y: usize,
        dot_col: usize,
        dot_row: usize,
        style: Style,
    ) {
        if x >= self.width || y >= self.height || dot_col > 1 || dot_row > 3 {
            return;
        }

        let index = y * self.width + x;
        let cell = &mut self.cells[index];

        let existing = cell.ch as u32;
        let bits = if (BRAILLE_BASE..=BRAILLE_BASE + 0xFF).contains(&existing) {
            existing - BRAILLE_BASE
        } else {
            0
        };

        cell.ch = char::from_u32((BRAILLE_BASE + bits) | DOT_BITS[dot_col][dot_row]).unwrap_or(' ');
        cell.style = style;
    }

    // Builds the full frame; every row is positioned absolutely
    pub fn to_string(&self) -> String {
        let mut out = String::with_capacity(self.width * self.height + self.height * 8);
        let mut current = Style::new();

        for row in 0..self.height {
            out.push_str(&format!("\x1b[{};1H", row + 1));

            for column in 0..self.width {
                let cell = self.cells[row * self.width + column];

                if cell.style != current {
                    out.push_str(&sgr(cell.style));
                    current = cell.style;
                }

                out.push(cell.ch);
            }

            if current != Style::new() {
                out.push_str("\x1b[0m");
                current = Style::new();
            }
        }

        out
    }
}

fn sgr(style: Style) -> String {
    let mut codes = vec!["0".to_string()];

    if style.bold {
        codes.push("1".to_string());
    }

    if style.fg != Color::Default {
        codes.push(style.fg.fg_code().to_string());
    }

    if style.bg != Color::Default {
        codes.push(style.bg.bg_code().to_string());
    }

    format!("\x1b[{}m", codes.join(";"))
}
