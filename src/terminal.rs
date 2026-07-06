use std::io::{self, Read, Write};
use std::process::{Command, Stdio};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    Char(char),
    Up,
    Down,
    Left,
    Right,
    Enter,
    Backspace,
    Tab,
    Esc,
    Unknown,
}

pub struct Terminal {
    original_mode: Option<String>,
}

impl Terminal {
    pub fn enter() -> io::Result<Self> {
        let original_mode = Command::new("stty")
            .arg("-g")
            .stderr(Stdio::null())
            .output()
            .ok()
            .and_then(|output| {
                if output.status.success() {
                    Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
                } else {
                    None
                }
            });

        let _ = Command::new("stty")
            .args(["raw", "-echo"])
            .stderr(Stdio::null())
            .status();
        print!("\x1b[?1049h\x1b[?25l\x1b[2J\x1b[H");
        io::stdout().flush()?;

        Ok(Self { original_mode })
    }

    pub fn draw(&mut self, frame: &str) -> io::Result<()> {
        print!("\x1b[H{frame}\x1b[J");
        io::stdout().flush()
    }
}

impl Drop for Terminal {
    fn drop(&mut self) {
        if let Some(mode) = &self.original_mode {
            let _ = Command::new("stty")
                .arg(mode)
                .stderr(Stdio::null())
                .status();
        } else {
            let _ = Command::new("stty")
                .args(["sane"])
                .stderr(Stdio::null())
                .status();
        }
        print!("\x1b[?25h\x1b[?1049l");
        let _ = io::stdout().flush();
    }
}

pub fn read_key() -> io::Result<Key> {
    let mut byte = [0_u8; 1];
    io::stdin().read_exact(&mut byte)?;

    match byte[0] {
        b'\r' | b'\n' => Ok(Key::Enter),
        b'\t' => Ok(Key::Tab),
        0x7f | 0x08 => Ok(Key::Backspace),
        0x1b => read_escape_sequence(),
        byte if byte.is_ascii() && !byte.is_ascii_control() => Ok(Key::Char(byte as char)),
        _ => Ok(Key::Unknown),
    }
}

pub fn terminal_size() -> (u16, u16) {
    let output = Command::new("stty")
        .arg("size")
        .stderr(Stdio::null())
        .output();
    if let Ok(output) = output
        && output.status.success()
    {
        let size = String::from_utf8_lossy(&output.stdout);
        let mut parts = size.split_whitespace();
        if let (Some(rows), Some(cols)) = (parts.next(), parts.next())
            && let (Ok(rows), Ok(cols)) = (rows.parse::<u16>(), cols.parse::<u16>())
        {
            return (cols.max(60), rows.max(20));
        }
    }

    let cols = std::env::var("COLUMNS")
        .ok()
        .and_then(|value| value.parse::<u16>().ok())
        .unwrap_or(100);
    let rows = std::env::var("LINES")
        .ok()
        .and_then(|value| value.parse::<u16>().ok())
        .unwrap_or(30);
    (cols.max(60), rows.max(20))
}

fn read_escape_sequence() -> io::Result<Key> {
    let mut sequence = [0_u8; 2];
    match io::stdin().read(&mut sequence)? {
        0 => Ok(Key::Esc),
        1 if sequence[0] == b'[' => Ok(Key::Esc),
        _ if sequence[0] == b'[' => match sequence[1] {
            b'A' => Ok(Key::Up),
            b'B' => Ok(Key::Down),
            b'C' => Ok(Key::Right),
            b'D' => Ok(Key::Left),
            _ => Ok(Key::Unknown),
        },
        _ => Ok(Key::Esc),
    }
}
