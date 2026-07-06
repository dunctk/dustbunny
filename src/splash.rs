use std::path::Path;
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::{Duration, Instant};

use crate::model::FileTree;
use crate::scanner;
use crate::terminal::{Terminal, terminal_size};

const FRAMES: [&str; 6] = ["-", "\\", "|", "/", "-", "\\"];
const DUST: [&str; 4] = [".  ", " . ", "  .", " . "];

pub fn scan_with_splash(path: &Path, terminal: &mut Terminal) -> std::io::Result<FileTree> {
    let path = path.to_path_buf();
    let path_label = path.display().to_string();
    let (sender, receiver) = mpsc::channel();

    thread::spawn(move || {
        let result = scanner::scan_path(path);
        let _ = sender.send(result);
    });

    wait_for_scan(receiver, path_label, terminal)
}

fn wait_for_scan(
    receiver: Receiver<std::io::Result<FileTree>>,
    path_label: String,
    terminal: &mut Terminal,
) -> std::io::Result<FileTree> {
    let started = Instant::now();
    let mut frame = 0_usize;
    let mut first_frame = true;

    loop {
        let (width, height) = terminal_size();
        let body = render_scan_frame(width, height, frame, &path_label, started.elapsed());
        terminal.draw(&body)?;
        frame = frame.wrapping_add(1);

        match receiver.try_recv() {
            Ok(result) if !first_frame => {
                let elapsed = started.elapsed();
                if elapsed < Duration::from_millis(450) {
                    thread::sleep(Duration::from_millis(450) - elapsed);
                }
                return result;
            }
            Ok(result) => {
                thread::sleep(Duration::from_millis(220));
                return result;
            }
            Err(mpsc::TryRecvError::Disconnected) => {
                return Err(std::io::Error::other("scanner worker disconnected"));
            }
            Err(mpsc::TryRecvError::Empty) => {}
        }

        first_frame = false;
        thread::sleep(Duration::from_millis(110));
    }
}

fn render_scan_frame(
    width: u16,
    height: u16,
    frame: usize,
    path: &str,
    elapsed: Duration,
) -> String {
    let width = width.max(80) as usize;
    let height = height.max(24) as usize;
    let mut lines = vec![" ".repeat(width); height];
    let center_x = width / 2;
    let top = height.saturating_sub(22) / 2;
    let ring_frame = frame % 10;
    let dust = DUST[frame % DUST.len()];
    let spinner = FRAMES[frame % FRAMES.len()];

    let art = logo_art(ring_frame, dust);
    for (idx, line) in art.iter().enumerate() {
        write_centered(&mut lines, top + idx, center_x, line);
    }

    write_centered(
        &mut lines,
        top + art.len() + 1,
        center_x,
        &format!(
            "\x1b[38;5;244m{spinner} scanning {}\x1b[0m",
            truncate(path, 54)
        ),
    );
    write_centered(
        &mut lines,
        top + art.len() + 2,
        center_x,
        &format!(
            "\x1b[38;5;240mcollecting dust for {:.1}s\x1b[0m",
            elapsed.as_secs_f32()
        ),
    );

    lines.join("\r\n")
}

fn logo_art(frame: usize, dust: &str) -> Vec<String> {
    let twinkle = if frame.is_multiple_of(2) { "*" } else { "+" };
    let seg = |index| arc_segment(index, frame);

    vec![
        format!("                 {}   {}   {}", seg(0), seg(1), seg(2)),
        format!("           {}                         {}", seg(7), seg(3)),
        format!("       {}        .-\"\"\"\"\"\"\"-.          \x1b[38;5;238m+------+\x1b[0m", seg(6)),
        "                .'  /\\   /\\  '.        \x1b[38;5;238m| \x1b[1;38;5;255m>_\x1b[0;38;5;238m   |\x1b[0m".to_string(),
        format!("    {}         /   ( o   o )   \\       \x1b[38;5;238m+------+\x1b[0m", seg(5)),
        "              ;       \\_/       ;".to_string(),
        "              |   .-       -.   |".to_string(),
        "              |  /  .-----.  \\  |        /|".to_string(),
        "              ;  \\_(_____)_/  ;       / |".to_string(),
        format!("       {}      \\      ___      /      /  |     \x1b[38;5;244m{dust}\x1b[0m", seg(4)),
        "                '._         _.'      /  /".to_string(),
        "                   '-.___.-'       _/__/".to_string(),
        format!("                   _/ / \\ \\_      /___/   \x1b[38;5;255m{twinkle}\x1b[0m"),
        format!("             {}             {}    {}", seg(8), seg(9), seg(10)),
        "".to_string(),
        "\x1b[1;38;5;255m ____              _   ____                            \x1b[0m".to_string(),
        "\x1b[1;38;5;255m|  _ \\ _   _ ___ | |_| __ ) _   _ _ __  _ __  _   _   \x1b[0m".to_string(),
        "\x1b[1;38;5;255m| | | | | | / __|| __|  _ \\| | | | '_ \\| '_ \\| | | |  \x1b[0m".to_string(),
        "\x1b[1;38;5;255m| |_| | |_| \\__ \\| |_| |_) | |_| | | | | | | | |_| |  \x1b[0m".to_string(),
        "\x1b[1;38;5;255m|____/ \\__,_|___/ \\__|____/ \\__,_|_| |_|_| |_|\\__, |  \x1b[0m".to_string(),
        "\x1b[1;38;5;255m                                              |___/   \x1b[0m".to_string(),
    ]
}

fn arc_segment(index: usize, frame: usize) -> String {
    let colors = [81, 75, 69, 99, 238, 211, 215, 179, 150, 119, 114];
    let color = colors[(index + frame) % colors.len()];
    format!("\x1b[38;5;{color}m======\x1b[0m")
}

fn write_centered(lines: &mut [String], row: usize, center_x: usize, text: &str) {
    if row >= lines.len() {
        return;
    }
    let visible = visible_width(text);
    let start = center_x.saturating_sub(visible / 2);
    write_at(lines, row, start, text);
}

fn write_at(lines: &mut [String], row: usize, col: usize, text: &str) {
    if row >= lines.len() || col >= lines[row].len() {
        return;
    }

    let mut line = lines[row].clone();
    let visible = visible_width(text);
    let end = (col + visible).min(line.len());
    line.replace_range(col..end, &" ".repeat(end.saturating_sub(col)));
    line.insert_str(col, text);
    lines[row] = line;
}

fn visible_width(text: &str) -> usize {
    let mut width = 0;
    let mut escape = false;
    for ch in text.chars() {
        if ch == '\x1b' {
            escape = true;
        } else if escape && ch == 'm' {
            escape = false;
        } else if !escape {
            width += 1;
        }
    }
    width
}

fn truncate(value: &str, width: usize) -> String {
    let chars: Vec<_> = value.chars().collect();
    if chars.len() <= width {
        value.to_string()
    } else {
        format!(
            "{}…",
            chars[..width.saturating_sub(1)].iter().collect::<String>()
        )
    }
}
