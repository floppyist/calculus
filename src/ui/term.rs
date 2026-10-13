use std::io::{self, Write};

#[cfg(target_os = "linux")]
mod unix;
#[cfg(windows)]
mod windows;

#[cfg(target_os = "linux")]
use unix::Backend;
#[cfg(windows)]
use windows::Backend;

// Fallback for platforms without a native console backend (e.g. macOS):
// environment-based size and line-buffered input, no raw mode
#[cfg(not(any(target_os = "linux", windows)))]
mod fallback {
    use std::io::{self, Read};

    pub struct Backend;

    impl Backend {
        pub fn enter() -> io::Result<Self> {
            Ok(Backend)
        }

        pub fn size(&self) -> (usize, usize) {
            super::env_size()
        }

        pub fn take_resized(&mut self) -> bool {
            false
        }

        pub fn read(&self, buffer: &mut [u8]) -> io::Result<usize> {
            io::stdin().read(buffer)
        }

        pub fn leave(&mut self) {}
    }
}

#[cfg(not(any(target_os = "linux", windows)))]
use fallback::Backend;

#[cfg(not(any(target_os = "linux", windows)))]
fn env_size() -> (usize, usize) {
    let columns = std::env::var("COLUMNS")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(80);
    let rows = std::env::var("LINES")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(24);

    (columns, rows)
}

pub struct Terminal {
    backend: Backend,
}

impl Terminal {
    // Enables raw mode, alternate screen, and the resize handler
    pub fn enter() -> io::Result<Self> {
        let backend = Backend::enter()?;

        let mut out = io::stdout();
        // Alternate screen, cursor off, autowrap off, clear screen
        write!(out, "\x1b[?1049h\x1b[?25l\x1b[?7l\x1b[2J")?;
        out.flush()?;

        Ok(Terminal { backend })
    }

    pub fn size(&self) -> (usize, usize) {
        self.backend.size()
    }

    // true if a resize was reported since the last call
    pub fn take_resized(&mut self) -> bool {
        self.backend.take_resized()
    }

    pub fn present(&mut self, frame: &str) -> io::Result<()> {
        let mut out = io::stdout();

        write!(out, "\x1b[H")?;
        out.write_all(frame.as_bytes())?;
        out.flush()
    }

    // Reads with a short timeout so resize events are seen promptly
    pub fn read(&self, buffer: &mut [u8]) -> io::Result<usize> {
        self.backend.read(buffer)
    }

    pub fn restore(&mut self) {
        let mut out = io::stdout();

        // Reset styles, cursor on, autowrap on, leave alternate screen
        let _ = write!(out, "\x1b[0m\x1b[?7h\x1b[?25h\x1b[?1049l");
        let _ = out.flush();

        self.backend.leave();
    }
}

impl Drop for Terminal {
    fn drop(&mut self) {
        self.restore();
    }
}
