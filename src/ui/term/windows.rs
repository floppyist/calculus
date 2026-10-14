use std::ffi::c_void;
use std::io::{self, Read};

type Handle = *mut c_void;
type Dword = u32;

const STD_INPUT_HANDLE: Dword = -10i32 as Dword;
const STD_OUTPUT_HANDLE: Dword = -11i32 as Dword;

const ENABLE_PROCESSED_INPUT: Dword = 0x0001;
const ENABLE_LINE_INPUT: Dword = 0x0002;
const ENABLE_ECHO_INPUT: Dword = 0x0004;
const ENABLE_VIRTUAL_TERMINAL_PROCESSING: Dword = 0x0004;
const ENABLE_EXTENDED_FLAGS: Dword = 0x0080;
const ENABLE_VIRTUAL_TERMINAL_INPUT: Dword = 0x0200;

const WAIT_TIMEOUT: Dword = 258;

const INVALID_HANDLE_VALUE: Handle = -1isize as Handle;

#[repr(C)]
struct Coord {
    x: i16,
    y: i16,
}

#[repr(C)]
struct SmallRect {
    left: i16,
    top: i16,
    right: i16,
    bottom: i16,
}

#[repr(C)]
struct ConsoleScreenBufferInfo {
    size: Coord,
    cursor_position: Coord,
    attributes: u16,
    window: SmallRect,
    maximum_window_size: Coord,
}

unsafe extern "system" {
    fn GetStdHandle(which: Dword) -> Handle;
    fn GetConsoleMode(handle: Handle, mode: *mut Dword) -> i32;
    fn SetConsoleMode(handle: Handle, mode: Dword) -> i32;
    fn GetConsoleScreenBufferInfo(handle: Handle, info: *mut ConsoleScreenBufferInfo) -> i32;
    fn WaitForSingleObject(handle: Handle, milliseconds: Dword) -> Dword;
    fn ReadFile(
        handle: Handle,
        buffer: *mut u8,
        to_read: Dword,
        read: *mut Dword,
        overlapped: *mut c_void,
    ) -> i32;
}

// Windows console backend: virtual-terminal input, sized via the screen buffer
pub struct Backend {
    input: Handle,
    output: Handle,
    original_input_mode: Dword,
    original_output_mode: Dword,
    input_ok: bool,
    output_ok: bool,
    last_size: (usize, usize),
}

impl Backend {
    pub fn enter() -> io::Result<Self> {
        unsafe {
            let input = GetStdHandle(STD_INPUT_HANDLE);
            let output = GetStdHandle(STD_OUTPUT_HANDLE);

            let mut input_mode: Dword = 0;
            let input_ok =
                input != INVALID_HANDLE_VALUE && GetConsoleMode(input, &mut input_mode) != 0;

            let mut output_mode: Dword = 0;
            let output_ok =
                output != INVALID_HANDLE_VALUE && GetConsoleMode(output, &mut output_mode) != 0;

            if input_ok {
                let raw = (input_mode
                    & !(ENABLE_LINE_INPUT | ENABLE_ECHO_INPUT | ENABLE_PROCESSED_INPUT))
                    | ENABLE_VIRTUAL_TERMINAL_INPUT
                    | ENABLE_EXTENDED_FLAGS;

                SetConsoleMode(input, raw);
            }

            if output_ok {
                SetConsoleMode(output, output_mode | ENABLE_VIRTUAL_TERMINAL_PROCESSING);
            }

            let mut backend = Backend {
                input,
                output,
                original_input_mode: input_mode,
                original_output_mode: output_mode,
                input_ok,
                output_ok,
                last_size: (0, 0),
            };

            backend.last_size = backend.size();
            Ok(backend)
        }
    }

    pub fn size(&self) -> (usize, usize) {
        if self.output_ok {
            let mut info: ConsoleScreenBufferInfo = unsafe { std::mem::zeroed() };
            let ok = unsafe { GetConsoleScreenBufferInfo(self.output, &mut info) } != 0;

            if ok {
                let columns = (info.window.right - info.window.left + 1).max(1) as usize;
                let rows = (info.window.bottom - info.window.top + 1).max(1) as usize;

                return (columns, rows);
            }
        }

        env_size()
    }

    // Windows has no SIGWINCH; detect size changes by polling
    pub fn take_resized(&mut self) -> bool {
        let size = self.size();

        if size != self.last_size {
            self.last_size = size;
            true
        } else {
            false
        }
    }

    pub fn read(&self, buffer: &mut [u8]) -> io::Result<usize> {
        if !self.input_ok {
            return io::stdin().read(buffer);
        }

        // Wait briefly so resize changes are noticed between reads
        let waited = unsafe { WaitForSingleObject(self.input, 100) };

        if waited == WAIT_TIMEOUT {
            return Ok(0);
        }

        let mut read: Dword = 0;
        let ok = unsafe {
            ReadFile(
                self.input,
                buffer.as_mut_ptr(),
                buffer.len() as Dword,
                &mut read,
                std::ptr::null_mut(),
            )
        };

        if ok == 0 {
            return Err(io::Error::last_os_error());
        }

        // Ctrl-Z acts as EOF on Windows; treat it like Ctrl-D
        for byte in buffer[..read as usize].iter_mut() {
            if *byte == 0x1a {
                *byte = 0x04;
            }
        }

        Ok(read as usize)
    }

    pub fn leave(&mut self) {
        unsafe {
            if self.input_ok {
                SetConsoleMode(self.input, self.original_input_mode);
            }

            if self.output_ok {
                SetConsoleMode(self.output, self.original_output_mode);
            }
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
