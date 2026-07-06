use std::fs;
use std::io;
use std::path::Path;

use crate::app::App;
use crate::ui;

const COLS: u16 = 112;
const ROWS: u16 = 34;
const CELL_WIDTH: u16 = 9;
const CELL_HEIGHT: u16 = 18;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Style {
    fg: u8,
    bg: u8,
    bold: bool,
    dim: bool,
}

impl Default for Style {
    fn default() -> Self {
        Self {
            fg: 252,
            bg: 235,
            bold: false,
            dim: false,
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct Cell {
    ch: char,
    style: Style,
}

impl Default for Cell {
    fn default() -> Self {
        Self {
            ch: ' ',
            style: Style::default(),
        }
    }
}

pub fn write_demo_svg(app: &App, path: impl AsRef<Path>) -> io::Result<()> {
    let ansi = ui::render(app, COLS, ROWS);
    let cells = parse_ansi(&ansi);
    let svg = cells_to_svg(&cells);
    fs::write(path, svg)
}

fn parse_ansi(input: &str) -> Vec<Vec<Cell>> {
    let mut cells = vec![vec![Cell::default(); COLS as usize]; ROWS as usize];
    let mut style = Style::default();
    let mut row = 0_usize;
    let mut col = 0_usize;
    let mut chars = input.chars().peekable();

    while let Some(ch) = chars.next() {
        match ch {
            '\x1b' => {
                if chars.next() == Some('[') {
                    let mut code = String::new();
                    for next in chars.by_ref() {
                        if next == 'm' {
                            break;
                        }
                        code.push(next);
                    }
                    apply_sgr(&mut style, &code);
                }
            }
            '\r' => {}
            '\n' => {
                row += 1;
                col = 0;
                if row >= ROWS as usize {
                    break;
                }
            }
            ch => {
                if row < ROWS as usize && col < COLS as usize {
                    cells[row][col] = Cell { ch, style };
                }
                col += 1;
            }
        }
    }

    cells
}

fn apply_sgr(style: &mut Style, code: &str) {
    let codes: Vec<u16> = if code.is_empty() {
        vec![0]
    } else {
        code.split(';')
            .filter_map(|part| part.parse::<u16>().ok())
            .collect()
    };

    let mut index = 0;
    while index < codes.len() {
        match codes[index] {
            0 => *style = Style::default(),
            1 => style.bold = true,
            2 => style.dim = true,
            22 => {
                style.bold = false;
                style.dim = false;
            }
            38 if codes.get(index + 1) == Some(&5) => {
                if let Some(color) = codes.get(index + 2) {
                    style.fg = *color as u8;
                }
                index += 2;
            }
            48 if codes.get(index + 1) == Some(&5) => {
                if let Some(color) = codes.get(index + 2) {
                    style.bg = *color as u8;
                }
                index += 2;
            }
            _ => {}
        }
        index += 1;
    }
}

fn cells_to_svg(cells: &[Vec<Cell>]) -> String {
    let width = COLS as u32 * CELL_WIDTH as u32;
    let height = ROWS as u32 * CELL_HEIGHT as u32;
    let mut svg = String::new();

    svg.push_str(&format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}">
<rect width="100%" height="100%" fill="#0f1117"/>
<style>
text {{
  font-family: "SFMono-Regular", "Cascadia Mono", "Liberation Mono", Menlo, Consolas, monospace;
  font-size: 15px;
  dominant-baseline: text-before-edge;
  white-space: pre;
}}
</style>
"##
    ));

    for (row_index, row) in cells.iter().enumerate() {
        let mut start = 0_usize;
        while start < row.len() {
            let bg = row[start].style.bg;
            let mut end = start + 1;
            while end < row.len() && row[end].style.bg == bg {
                end += 1;
            }
            let color = xterm_color(bg);
            svg.push_str(&format!(
                r#"<rect x="{}" y="{}" width="{}" height="{}" fill="{}"/>"#,
                start as u16 * CELL_WIDTH,
                row_index as u16 * CELL_HEIGHT,
                (end - start) as u16 * CELL_WIDTH,
                CELL_HEIGHT,
                color
            ));
            svg.push('\n');
            start = end;
        }
    }

    for (row_index, row) in cells.iter().enumerate() {
        let mut start = 0_usize;
        while start < row.len() {
            let style = row[start].style;
            let mut end = start + 1;
            while end < row.len()
                && row[end].style == style
                && row[end].ch != ' '
                && row[start].ch != ' '
            {
                end += 1;
            }

            if row[start].ch != ' ' {
                let text: String = row[start..end].iter().map(|cell| cell.ch).collect();
                svg.push_str(&format!(
                    r#"<text x="{}" y="{}" fill="{}"{}{}>{}</text>"#,
                    start as u16 * CELL_WIDTH,
                    row_index as u16 * CELL_HEIGHT + 2,
                    xterm_color(style.fg),
                    if style.bold {
                        r#" font-weight="700""#
                    } else {
                        ""
                    },
                    if style.dim { r#" opacity="0.65""# } else { "" },
                    escape_xml(&text)
                ));
                svg.push('\n');
            }
            start = end;
        }
    }

    svg.push_str("</svg>\n");
    svg
}

fn xterm_color(color: u8) -> String {
    let (r, g, b) = match color {
        0..=15 => ANSI_16[color as usize],
        16..=231 => {
            let n = color - 16;
            let r = n / 36;
            let g = (n % 36) / 6;
            let b = n % 6;
            (cube(r), cube(g), cube(b))
        }
        232..=255 => {
            let value = 8 + (color - 232) * 10;
            (value, value, value)
        }
    };
    format!("#{r:02x}{g:02x}{b:02x}")
}

fn cube(value: u8) -> u8 {
    if value == 0 { 0 } else { 55 + value * 40 }
}

fn escape_xml(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

const ANSI_16: [(u8, u8, u8); 16] = [
    (0, 0, 0),
    (128, 0, 0),
    (0, 128, 0),
    (128, 128, 0),
    (0, 0, 128),
    (128, 0, 128),
    (0, 128, 128),
    (192, 192, 192),
    (128, 128, 128),
    (255, 0, 0),
    (0, 255, 0),
    (255, 255, 0),
    (0, 0, 255),
    (255, 0, 255),
    (0, 255, 255),
    (255, 255, 255),
];
