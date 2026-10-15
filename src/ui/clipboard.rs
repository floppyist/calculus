use std::io::Write;
use std::process::{Command, Stdio};

const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

// Puts text into the system clipboard; falls back to the OSC 52 sequence
pub fn copy(text: &str) -> bool {
    let tools: [(&str, &[&str]); 5] = [
        ("wl-copy", &[]),
        ("xclip", &["-selection", "clipboard", "-i"]),
        ("xsel", &["--clipboard", "--input"]),
        ("pbcopy", &[]),
        ("clip.exe", &[]),
    ];

    for (program, args) in tools {
        if feed(program, args, text) {
            return true;
        }
    }

    let payload = base64(text.as_bytes());
    let mut out = std::io::stdout();

    write!(out, "\x1b]52;c;{}\x07", payload).is_ok() && out.flush().is_ok()
}

// Reads the system clipboard through the first helper tool that works
pub fn paste() -> Option<String> {
    let tools: [(&str, &[&str]); 5] = [
        ("wl-paste", &["--no-newline"]),
        ("xclip", &["-selection", "clipboard", "-o"]),
        ("xsel", &["--clipboard", "--output"]),
        ("pbpaste", &[]),
        (
            "powershell.exe",
            &["-NoProfile", "-Command", "Get-Clipboard"],
        ),
    ];

    for (program, args) in tools {
        let Ok(output) = Command::new(program)
            .args(args)
            .stdin(Stdio::null())
            .stderr(Stdio::null())
            .output()
        else {
            continue;
        };

        if !output.status.success() {
            continue;
        }

        let Ok(text) = String::from_utf8(output.stdout) else {
            continue;
        };

        return Some(text.trim_end_matches(['\r', '\n']).to_string());
    }

    None
}

// Runs a helper tool with the text on its standard input
fn feed(program: &str, args: &[&str], text: &str) -> bool {
    let Ok(mut child) = Command::new(program)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    else {
        return false;
    };

    let Some(mut stdin) = child.stdin.take() else {
        return false;
    };

    if stdin.write_all(text.as_bytes()).is_err() {
        return false;
    }

    drop(stdin);

    child.wait().is_ok_and(|status| status.success())
}

// Standard base64 with padding, as required by OSC 52
pub fn base64(data: &[u8]) -> String {
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);

    for chunk in data.chunks(3) {
        let group = ((chunk[0] as u32) << 16)
            | ((chunk.get(1).copied().unwrap_or(0) as u32) << 8)
            | chunk.get(2).copied().unwrap_or(0) as u32;

        out.push(ALPHABET[(group >> 18) as usize & 63] as char);
        out.push(ALPHABET[(group >> 12) as usize & 63] as char);
        out.push(if chunk.len() > 1 {
            ALPHABET[(group >> 6) as usize & 63] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            ALPHABET[group as usize & 63] as char
        } else {
            '='
        });
    }

    out
}
