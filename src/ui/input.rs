#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Key {
    Char(char),
    Enter,
    Backspace,
    Delete,
    Left,
    Right,
    Up,
    Down,
    Home,
    End,
    Tab,
    PageUp,
    PageDown,
    CtrlC,
    CtrlD,
    CtrlL,
    Escape,
    Unknown,
}

pub struct LineEditor {
    chars: Vec<char>,
    cursor: usize,
    history: Vec<String>,
    history_pos: Option<usize>,
    stash: Vec<char>,
}

impl LineEditor {
    pub fn new() -> Self {
        LineEditor {
            chars: Vec::new(),
            cursor: 0,
            history: Vec::new(),
            history_pos: None,
            stash: Vec::new(),
        }
    }

    pub fn chars(&self) -> &[char] {
        &self.chars
    }

    pub fn cursor(&self) -> usize {
        self.cursor
    }

    pub fn is_empty(&self) -> bool {
        self.chars.is_empty()
    }

    pub fn text(&self) -> String {
        self.chars.iter().collect()
    }

    pub fn clear(&mut self) {
        self.chars.clear();
        self.cursor = 0;
        self.history_pos = None;
    }

    fn reset_history(&mut self) {
        self.history_pos = None;
    }

    pub fn insert(&mut self, ch: char) {
        self.chars.insert(self.cursor, ch);
        self.cursor += 1;
        self.reset_history();
    }

    pub fn backspace(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
            self.chars.remove(self.cursor);
            self.reset_history();
        }
    }

    pub fn delete(&mut self) {
        if self.cursor < self.chars.len() {
            self.chars.remove(self.cursor);
            self.reset_history();
        }
    }

    pub fn move_left(&mut self) {
        self.cursor = self.cursor.saturating_sub(1);
    }

    pub fn move_right(&mut self) {
        if self.cursor < self.chars.len() {
            self.cursor += 1;
        }
    }

    pub fn home(&mut self) {
        self.cursor = 0;
    }

    pub fn end(&mut self) {
        self.cursor = self.chars.len();
    }

    pub fn move_word_forward(&mut self) {
        let len = self.chars.len();

        while self.cursor < len && !self.chars[self.cursor].is_whitespace() {
            self.cursor += 1;
        }

        while self.cursor < len && self.chars[self.cursor].is_whitespace() {
            self.cursor += 1;
        }
    }

    pub fn move_word_back(&mut self) {
        while self.cursor > 0 && self.chars[self.cursor - 1].is_whitespace() {
            self.cursor -= 1;
        }

        while self.cursor > 0 && !self.chars[self.cursor - 1].is_whitespace() {
            self.cursor -= 1;
        }
    }

    pub fn delete_to_end(&mut self) {
        if self.cursor < self.chars.len() {
            self.chars.truncate(self.cursor);
            self.reset_history();
        }
    }

    // Keeps the cursor on a character, as vi normal mode expects
    pub fn clamp_normal(&mut self) {
        if !self.chars.is_empty() && self.cursor >= self.chars.len() {
            self.cursor = self.chars.len() - 1;
        }
    }

    pub fn history_prev(&mut self) {
        if self.history.is_empty() {
            return;
        }

        let position = match self.history_pos {
            None => {
                self.stash = self.chars.clone();
                self.history.len() - 1
            }
            Some(0) => return,
            Some(position) => position - 1,
        };

        self.history_pos = Some(position);
        self.chars = self.history[position].chars().collect();
        self.cursor = self.chars.len();
    }

    pub fn history_next(&mut self) {
        match self.history_pos {
            None => {}
            Some(position) if position + 1 < self.history.len() => {
                self.history_pos = Some(position + 1);
                self.chars = self.history[position + 1].chars().collect();
                self.cursor = self.chars.len();
            }
            Some(_) => {
                self.history_pos = None;
                self.chars = self.stash.clone();
                self.cursor = self.chars.len();
            }
        }
    }

    pub fn push_history(&mut self, entry: String) {
        if entry.is_empty() || self.history.last() == Some(&entry) {
            return;
        }

        self.history.push(entry);
    }
}

pub struct KeyReader {
    buffer: Vec<u8>,
}

impl KeyReader {
    pub fn new() -> Self {
        KeyReader { buffer: Vec::new() }
    }

    pub fn feed(&mut self, bytes: &[u8]) {
        self.buffer.extend_from_slice(bytes);
    }

    pub fn next_key(&mut self) -> Option<Key> {
        let first = *self.buffer.first()?;

        match first {
            0x1b => self.decode_escape(),
            b'\r' | b'\n' => {
                self.buffer.remove(0);
                Some(Key::Enter)
            }
            0x7f | 0x08 => {
                self.buffer.remove(0);
                Some(Key::Backspace)
            }
            0x09 => {
                self.buffer.remove(0);
                Some(Key::Tab)
            }
            0x03 => {
                self.buffer.remove(0);
                Some(Key::CtrlC)
            }
            0x04 => {
                self.buffer.remove(0);
                Some(Key::CtrlD)
            }
            0x0c => {
                self.buffer.remove(0);
                Some(Key::CtrlL)
            }
            _ if first < 0x20 => {
                self.buffer.remove(0);
                Some(Key::Unknown)
            }
            _ => self.decode_char(),
        }
    }

    fn decode_escape(&mut self) -> Option<Key> {
        if self.buffer.len() == 1 {
            self.buffer.remove(0);
            return Some(Key::Escape);
        }

        if self.buffer[1] == b'[' || self.buffer[1] == b'O' {
            let end = (2..self.buffer.len())
                .find(|index| (0x40..=0x7e).contains(&self.buffer[*index]));

            let Some(end) = end else {
                // Sequence still incomplete
                return None;
            };

            let sequence: Vec<u8> = self.buffer.drain(0..=end).collect();

            return Some(decode_csi(&sequence));
        }

        // Treat Alt+... like Escape
        self.buffer.remove(0);
        Some(Key::Escape)
    }

    fn decode_char(&mut self) -> Option<Key> {
        match std::str::from_utf8(&self.buffer) {
            Ok(valid) => {
                let ch = valid.chars().next()?;

                self.buffer.drain(0..ch.len_utf8());
                Some(Key::Char(ch))
            }
            Err(error) => {
                if error.valid_up_to() > 0 {
                    let valid = std::str::from_utf8(&self.buffer[..error.valid_up_to()]).unwrap();
                    let ch = valid.chars().next().unwrap();

                    self.buffer.drain(0..ch.len_utf8());
                    Some(Key::Char(ch))
                } else if error.error_len().is_none() {
                    // Incomplete UTF-8 sequence: wait for more bytes
                    None
                } else {
                    self.buffer.remove(0);
                    Some(Key::Unknown)
                }
            }
        }
    }
}

fn decode_csi(sequence: &[u8]) -> Key {
    match sequence.last().copied() {
        Some(b'A') => Key::Up,
        Some(b'B') => Key::Down,
        Some(b'C') => Key::Right,
        Some(b'D') => Key::Left,
        Some(b'H') => Key::Home,
        Some(b'F') => Key::End,
        Some(b'~') => {
            let body: String = sequence[2..sequence.len() - 1]
                .iter()
                .map(|byte| *byte as char)
                .collect();

            match body.as_str() {
                "1" | "7" => Key::Home,
                "4" | "8" => Key::End,
                "3" => Key::Delete,
                "5" => Key::PageUp,
                "6" => Key::PageDown,
                _ => Key::Unknown,
            }
        }
        _ => Key::Unknown,
    }
}


