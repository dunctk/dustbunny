use std::fs;
use std::io;
use std::path::Path;

use crate::app::App;
use crate::model::{FileTree, NodeId, format_size};
use crate::ui;

const COLS: u16 = 140;
const ROWS: u16 = 42;
const CELL_WIDTH: u16 = 9;
const CELL_HEIGHT: u16 = 18;
const BG: (u8, u8, u8) = (40, 43, 48);
const HUB_BG: (u8, u8, u8) = (15, 15, 19);
const MUTED: (u8, u8, u8) = (152, 156, 164);
const FAINT: (u8, u8, u8) = (98, 102, 110);
const WHITE: (u8, u8, u8) = (255, 255, 255);
const SELECTED: (u8, u8, u8) = (255, 199, 61);
const SELECTED_EDGE: (u8, u8, u8) = (255, 255, 255);
const START_ANGLE: f64 = 0.62 * std::f64::consts::PI;
const END_ANGLE: f64 = 2.24 * std::f64::consts::PI;
const HUE_START: f64 = 118.0;
const HUE_SPAN: f64 = 300.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Style {
    fg: (u8, u8, u8),
    bg: (u8, u8, u8),
    bold: bool,
    dim: bool,
}

impl Default for Style {
    fn default() -> Self {
        Self {
            fg: (230, 232, 236),
            bg: (40, 43, 48),
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
    let svg = cells_to_svg(app, &cells);
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
                    let mut command = '\0';
                    for next in chars.by_ref() {
                        if next.is_ascii_alphabetic() {
                            command = next;
                            break;
                        }
                        code.push(next);
                    }
                    if command == 'm' {
                        apply_sgr(&mut style, &code);
                    }
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
            38 if codes.get(index + 1) == Some(&2) => {
                if let (Some(r), Some(g), Some(b)) = (
                    codes.get(index + 2),
                    codes.get(index + 3),
                    codes.get(index + 4),
                ) {
                    style.fg = (*r as u8, *g as u8, *b as u8);
                }
                index += 4;
            }
            48 if codes.get(index + 1) == Some(&2) => {
                if let (Some(r), Some(g), Some(b)) = (
                    codes.get(index + 2),
                    codes.get(index + 3),
                    codes.get(index + 4),
                ) {
                    style.bg = (*r as u8, *g as u8, *b as u8);
                }
                index += 4;
            }
            _ => {}
        }
        index += 1;
    }
}

fn cells_to_svg(app: &App, cells: &[Vec<Cell>]) -> String {
    let width = COLS as u32 * CELL_WIDTH as u32;
    let height = ROWS as u32 * CELL_HEIGHT as u32;
    let map = map_rect();
    let mut svg = String::new();

    svg.push_str(&format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}">
<rect width="100%" height="100%" fill="#282b30"/>
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
            let color = hex_color(bg);
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

    svg.push_str(&format!(
        r#"<rect x="{}" y="{}" width="{}" height="{}" fill="{}"/>"#,
        map.x * CELL_WIDTH,
        map.y * CELL_HEIGHT,
        map.width * CELL_WIDTH,
        map.height * CELL_HEIGHT,
        hex_color(BG)
    ));
    svg.push('\n');
    draw_smooth_sunburst(&mut svg, app, map);

    for (row_index, row) in cells.iter().enumerate() {
        let mut start = 0_usize;
        while start < row.len() {
            if map.contains(start as u16, row_index as u16) {
                start += 1;
                continue;
            }

            let style = row[start].style;
            let mut end = start + 1;
            while end < row.len()
                && row[end].style == style
                && row[end].ch != ' '
                && row[start].ch != ' '
                && !map.contains(end as u16, row_index as u16)
            {
                end += 1;
            }

            if row[start].ch != ' ' {
                let text: String = row[start..end].iter().map(|cell| cell.ch).collect();
                svg.push_str(&format!(
                    r#"<text x="{}" y="{}" fill="{}"{}{}>{}</text>"#,
                    start as u16 * CELL_WIDTH,
                    row_index as u16 * CELL_HEIGHT + 2,
                    hex_color(style.fg),
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

fn draw_smooth_sunburst(svg: &mut String, app: &App, map: SvgRect) {
    let geometry = SvgGeometry::new(map);
    let segments = collect_svg_segments(&app.tree, app.view_root, geometry.max_depth);

    svg.push_str(&format!(
        r#"<text x="{}" y="{}" fill="{}">map</text>"#,
        (map.x + 3) * CELL_WIDTH,
        map.y * CELL_HEIGHT + 2,
        hex_color(FAINT)
    ));
    svg.push('\n');

    for segment in &segments {
        let inner = geometry.center_radius + segment.depth as f64 * geometry.ring_width + 1.5;
        let outer = geometry.center_radius + (segment.depth + 1) as f64 * geometry.ring_width - 1.5;
        let mut start = segment.start;
        let mut end = segment.end;
        let gap = ((end - start) * 0.045).min(0.010);
        start += gap;
        end -= gap;
        if end <= start || outer <= inner {
            continue;
        }

        let selected = segment.node == app.selected;
        let color = if selected { SELECTED } else { segment.color };
        svg.push_str(&format!(
            r#"<path d="{}" fill="{}" stroke="{}" stroke-width="{}" stroke-linejoin="round"/>"#,
            annular_sector_path(geometry.cx, geometry.cy, inner, outer, start, end),
            hex_color(color),
            hex_color(if selected { SELECTED_EDGE } else { BG }),
            if selected { 5.0 } else { 1.4 }
        ));
        svg.push('\n');
    }

    svg.push_str(&format!(
        r#"<circle cx="{:.2}" cy="{:.2}" r="{:.2}" fill="{}"/>"#,
        geometry.cx,
        geometry.cy,
        geometry.center_radius - 1.0,
        hex_color(HUB_BG)
    ));
    svg.push('\n');

    let root = app.tree.get(app.view_root);
    let size = format_size(root.size_bytes);
    let mut parts = size.splitn(2, ' ');
    let number = parts.next().unwrap_or_default();
    let unit = parts.next().unwrap_or_default();
    svg.push_str(&format!(
        r#"<text x="{:.2}" y="{:.2}" fill="{}" font-weight="700" text-anchor="middle">{}</text>"#,
        geometry.cx,
        geometry.cy - 13.0,
        hex_color(WHITE),
        escape_xml(number)
    ));
    svg.push('\n');
    svg.push_str(&format!(
        r#"<text x="{:.2}" y="{:.2}" fill="{}" text-anchor="middle">{}</text>"#,
        geometry.cx,
        geometry.cy + 5.0,
        hex_color(MUTED),
        escape_xml(unit)
    ));
    svg.push('\n');

    let drop_y = (map.y + map.height.saturating_sub(2)) * CELL_HEIGHT + 2;
    svg.push_str(&format!(
        r#"<text x="{}" y="{}" fill="{}">◎</text>"#,
        (map.x + 4) * CELL_WIDTH,
        drop_y,
        hex_color(FAINT)
    ));
    svg.push('\n');
    svg.push_str(&format!(
        r#"<text x="{}" y="{}" fill="{}">drag and drop files here to collect them</text>"#,
        (map.x + 8) * CELL_WIDTH,
        drop_y,
        hex_color(FAINT)
    ));
    svg.push('\n');
}

fn collect_svg_segments(tree: &FileTree, root: NodeId, max_depth: usize) -> Vec<SvgSegment> {
    let mut segments = Vec::new();
    collect_svg_segments_inner(
        tree,
        root,
        START_ANGLE,
        END_ANGLE,
        0,
        max_depth,
        &mut segments,
    );
    segments
}

fn collect_svg_segments_inner(
    tree: &FileTree,
    node: NodeId,
    start: f64,
    end: f64,
    depth: usize,
    max_depth: usize,
    segments: &mut Vec<SvgSegment>,
) {
    if depth >= max_depth {
        return;
    }

    let children: Vec<_> = tree
        .children_sorted(node)
        .into_iter()
        .filter(|child| tree.get(*child).size_bytes > 0)
        .collect();
    if children.is_empty() {
        return;
    }

    let total: u64 = children
        .iter()
        .map(|child| tree.get(*child).size_bytes.max(1))
        .sum();
    let mut cursor = start;

    for child in children {
        let size = tree.get(child).size_bytes.max(1);
        let span = (end - start) * (size as f64 / total as f64);
        let child_end = (cursor + span).min(end);
        if child_end - cursor > 0.0008 {
            let color = segment_color((cursor + child_end) / 2.0, depth);
            segments.push(SvgSegment {
                node: child,
                start: cursor,
                end: child_end,
                depth,
                color,
            });
            collect_svg_segments_inner(
                tree,
                child,
                cursor,
                child_end,
                depth + 1,
                max_depth,
                segments,
            );
        }
        cursor = child_end;
    }
}

fn annular_sector_path(cx: f64, cy: f64, inner: f64, outer: f64, start: f64, end: f64) -> String {
    let (outer_start_x, outer_start_y) = polar_point(cx, cy, outer, start);
    let (outer_end_x, outer_end_y) = polar_point(cx, cy, outer, end);
    let (inner_end_x, inner_end_y) = polar_point(cx, cy, inner, end);
    let (inner_start_x, inner_start_y) = polar_point(cx, cy, inner, start);
    let large_arc = i32::from(end - start > std::f64::consts::PI);

    format!(
        "M {:.2} {:.2} A {:.2} {:.2} 0 {large_arc} 0 {:.2} {:.2} L {:.2} {:.2} A {:.2} {:.2} 0 {large_arc} 1 {:.2} {:.2} Z",
        outer_start_x,
        outer_start_y,
        outer,
        outer,
        outer_end_x,
        outer_end_y,
        inner_end_x,
        inner_end_y,
        inner,
        inner,
        inner_start_x,
        inner_start_y,
    )
}

fn polar_point(cx: f64, cy: f64, radius: f64, angle: f64) -> (f64, f64) {
    (cx + angle.cos() * radius, cy - angle.sin() * radius)
}

fn segment_color(mid_angle: f64, depth: usize) -> (u8, u8, u8) {
    let t = ((mid_angle - START_ANGLE) / (END_ANGLE - START_ANGLE)).clamp(0.0, 1.0);
    let hue = HUE_START + t * HUE_SPAN;
    let lightness = (0.50 + depth as f64 * 0.085).min(0.86);
    let saturation = (0.62 - depth as f64 * 0.05).max(0.30);
    hsl(hue, saturation, lightness)
}

fn hsl(hue: f64, saturation: f64, lightness: f64) -> (u8, u8, u8) {
    let h = hue.rem_euclid(360.0) / 360.0;
    let s = saturation.clamp(0.0, 1.0);
    let l = lightness.clamp(0.0, 1.0);
    if s == 0.0 {
        let v = (l * 255.0).round() as u8;
        return (v, v, v);
    }
    let q = if l < 0.5 {
        l * (1.0 + s)
    } else {
        l + s - l * s
    };
    let p = 2.0 * l - q;
    let r = hue_channel(p, q, h + 1.0 / 3.0);
    let g = hue_channel(p, q, h);
    let b = hue_channel(p, q, h - 1.0 / 3.0);
    (
        (r * 255.0).round() as u8,
        (g * 255.0).round() as u8,
        (b * 255.0).round() as u8,
    )
}

fn hue_channel(p: f64, q: f64, t: f64) -> f64 {
    let t = t.rem_euclid(1.0);
    if t < 1.0 / 6.0 {
        p + (q - p) * 6.0 * t
    } else if t < 0.5 {
        q
    } else if t < 2.0 / 3.0 {
        p + (q - p) * (2.0 / 3.0 - t) * 6.0
    } else {
        p
    }
}

fn map_rect() -> SvgRect {
    let header_height = 4;
    let status_height = 2;
    let side_width = screenshot_sidebar_width(COLS);
    SvgRect {
        x: 0,
        y: header_height,
        width: COLS.saturating_sub(side_width),
        height: ROWS.saturating_sub(header_height + status_height),
    }
}

fn screenshot_sidebar_width(width: u16) -> u16 {
    if width < 124 {
        0
    } else if width < 144 {
        34
    } else {
        ((width as f32 * 0.31) as u16).clamp(34, 46)
    }
}

#[derive(Debug, Clone, Copy)]
struct SvgRect {
    x: u16,
    y: u16,
    width: u16,
    height: u16,
}

impl SvgRect {
    fn contains(self, x: u16, y: u16) -> bool {
        x >= self.x
            && x < self.x.saturating_add(self.width)
            && y >= self.y
            && y < self.y.saturating_add(self.height)
    }
}

#[derive(Debug, Clone, Copy)]
struct SvgGeometry {
    cx: f64,
    cy: f64,
    center_radius: f64,
    ring_width: f64,
    max_depth: usize,
}

impl SvgGeometry {
    fn new(map: SvgRect) -> Self {
        let cell = CELL_WIDTH as f64;
        let left = map.x as f64 * cell + 54.0;
        let right = (map.x + map.width) as f64 * cell - 54.0;
        let top = map.y as f64 * CELL_HEIGHT as f64 + 50.0;
        let bottom = (map.y + map.height) as f64 * CELL_HEIGHT as f64 - 82.0;
        let cx = (left + right) / 2.0;
        let cy = (top + bottom) / 2.0;
        let max_radius = ((right - left) / 2.0).min((bottom - top) / 2.0).max(80.0);
        let center_radius = (max_radius * 0.18).clamp(34.0, 52.0);
        let max_depth = if max_radius > 150.0 { 5 } else { 4 };
        let ring_width = (max_radius - center_radius) / max_depth as f64;

        Self {
            cx,
            cy,
            center_radius,
            ring_width,
            max_depth,
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct SvgSegment {
    node: NodeId,
    start: f64,
    end: f64,
    depth: usize,
    color: (u8, u8, u8),
}

fn hex_color((r, g, b): (u8, u8, u8)) -> String {
    format!("#{r:02x}{g:02x}{b:02x}")
}

fn escape_xml(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
