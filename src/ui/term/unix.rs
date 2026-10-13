use std::io::{self, Read};
use std::sync::atomic::{AtomicBool, Ordering};

// File descriptors of the standard streams
const STDIN_FILENO: i32 = 0;
const STDOUT_FILENO: i32 = 1;

// Linux constants for termios and ioctl
const TCSANOW: i32 = 0;
const TIOCGWINSZ: u64 = 0x5413;
const SIGWINCH: i32 = 28;

const ICANON: u32 = 0o0002;
const ECHO: u32 = 0o0010;
const ISIG: u32 = 0o0001;
const IEXTEN: u32 = 0o100000;
const IXON: u32 = 0o2000;
const ICRNL: u32 = 0o0400;
const OPOST: u32 = 0o0001;

// Indices into c_cc
const VMIN: usize = 6;
const VTIME: usize = 5;

#[repr(C)]
#[derive(Clone, Copy)]
struct Termios {
    c_iflag: u32,
    c_oflag: u32,
    c_cflag: u32,
    c_lflag: u32,
    c_line: u8,
    c_cc: [u8; 32],
    c_ispeed: u32,
    c_ospeed: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct Winsize {
    ws_row: u16,
    ws_col: u16,
    ws_xpixel: u16,
    ws_ypixel: u16,
}

unsafe extern "C" {
    fn ioctl(fd: i32, request: u64, ...) -> i32;
    fn tcgetattr(fd: i32, termios: *mut Termios) -> i32;
    fn tcsetattr(fd: i32, actions: i32, termios: *const Termios) -> i32;
    fn signal(signum: i32, handler: usize) -> usize;
}

// Set by the SIGWINCH handler
static RESIZED: AtomicBool = AtomicBool::new(false);

extern "C" fn handle_winch(_sig: i32) {
    RESIZED.store(true, Ordering::SeqCst);
}

// Linux console backend: raw mode, ioctl sizing, SIGWINCH resize
pub struct Backend {
    original: Termios,
    raw_ok: bool,
}

impl Backend {
    pub fn enter() -> io::Result<Self> {
        let mut original: Termios = unsafe { std::mem::zeroed() };
        let raw_ok = unsafe { tcgetattr(STDIN_FILENO, &mut original) } == 0;

        if raw_ok {
            let mut raw = original;

            raw.c_lflag &= !(ICANON | ECHO | ISIG | IEXTEN);
            raw.c_iflag &= !(IXON | ICRNL);
            raw.c_oflag &= !OPOST;
            raw.c_cc[VMIN] = 0;
            raw.c_cc[VTIME] = 1;

            unsafe { tcsetattr(STDIN_FILENO, TCSANOW, &raw) };
        }

        unsafe {
            signal(SIGWINCH, handle_winch as extern "C" fn(i32) as usize);
        }

        Ok(Backend { original, raw_ok })
    }

    pub fn size(&self) -> (usize, usize) {
        let mut ws = Winsize::default();

        let ok = unsafe { ioctl(STDOUT_FILENO, TIOCGWINSZ, &mut ws as *mut Winsize) } == 0;

        if ok && ws.ws_col > 0 && ws.ws_row > 0 {
            return (ws.ws_col as usize, ws.ws_row as usize);
        }

        env_size()
    }

    pub fn take_resized(&mut self) -> bool {
        RESIZED.swap(false, Ordering::SeqCst)
    }

    pub fn read(&self, buffer: &mut [u8]) -> io::Result<usize> {
        if !self.raw_ok {
            return io::stdin().read(buffer);
        }

        match io::stdin().read(buffer) {
            Ok(count) => Ok(count),
            Err(error) if error.kind() == io::ErrorKind::Interrupted => Ok(0),
            Err(error) => Err(error),
        }
    }

    pub fn leave(&mut self) {
        if self.raw_ok {
            unsafe { tcsetattr(STDIN_FILENO, TCSANOW, &self.original) };
        }
    }
}

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
