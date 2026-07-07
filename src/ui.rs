use std::f64::consts::PI;

use crate::app::{App, Focus};
use crate::layout::Rect;
use crate::model::{FileTree, NodeId, format_size};

type Rgb = (u8, u8, u8);

const BG: Rgb = (40, 43, 48);
const HUB_BG: Rgb = (15, 15, 19);
const DIVIDER: Rgb = (62, 65, 72);
const TEXT: Rgb = (230, 232, 236);
const MUTED: Rgb = (152, 156, 164);
const FAINT: Rgb = (98, 102, 110);
const WHITE: Rgb = (255, 255, 255);
const SELECTED: Rgb = (255, 199, 61);
const SELECTED_TEXT: Rgb = (48, 38, 8);
const SELECTED_EDGE: Rgb = (255, 255, 255);
const ACTIVE_PILL: Rgb = (64, 111, 220);
const INACTIVE_PILL: Rgb = (188, 193, 201);
const INACTIVE_PILL_TEXT: Rgb = (36, 39, 44);

const START_ANGLE: f64 = 0.62 * PI;
const END_ANGLE: f64 = 2.24 * PI;
const HUE_START: f64 = 118.0;
const HUE_SPAN: f64 = 300.0;
const RADIAL_GAP: f64 = 0.22;
const ANGULAR_SEAM_HALF: f64 = 0.16;

pub fn render(app: &App, width: u16, height: u16) -> String {
    let width = width.max(80);
    let height = height.max(24);
    let mut canvas = Canvas::new(width, height, Style::bg(BG));

    let header = Rect {
        x: 0,
        y: 0,
        width,
        height: 4,
    };
    let status = Rect {
        x: 0,
        y: height - 2,
        width,
        height: 2,
    };
    let body_height = height.saturating_sub(header.height + status.height);
    let side_width = ((width as f32 * 0.31) as u16).clamp(30, 44);
    let map = Rect {
        x: 0,
        y: header.height,
        width: width.saturating_sub(side_width),
        height: body_height,
    };
    let side = Rect {
        x: map.width,
        y: header.height,
        width: side_width,
        height: body_height,
    };

    let geometry = compute_geometry(map);
    let segments = sunburst_segments(&app.tree, app.view_root, geometry.max_depth);

    draw_header(&mut canvas, app, header);
    draw_sunburst(&mut canvas, app, map, &geometry, &segments);
    draw_sidebar(&mut canvas, app, side, &segments);
    draw_status(&mut canvas, app, status);

    if app.show_help {
        draw_help(&mut canvas, width, height);
    }

    canvas.finish()
}

fn draw_header(canvas: &mut Canvas, app: &App, area: Rect) {
    let root = app.tree.get(app.tree.root());
    let left = area.x + 3;
    canvas.write(left, area.y + 1, "Dustbunny", Style::fg(WHITE).bold());
    draw_pill(
        canvas,
        left + 12,
        area.y + 1,
        "Disks and Folders",
        Style::new(Some(WHITE), Some(ACTIVE_PILL), true, false),
    );
    draw_pill(
        canvas,
        left + 31,
        area.y + 1,
        &truncate(&app.tree.get(app.view_root).name, 24),
        Style::new(Some(INACTIVE_PILL_TEXT), Some(INACTIVE_PILL), false, false),
    );
    canvas.write(
        left,
        area.y + 2,
        &truncate(
            &format!(
                "{}   {}",
                app.tree.breadcrumb(app.view_root),
                root.path.display()
            ),
            area.width.saturating_sub(6) as usize,
        ),
        Style::fg(MUTED),
    );
}

#[derive(Debug, Clone, Copy)]
struct SunburstGeometry {
    cx: f64,
    cy_hi: f64,
    center_radius: f64,
    ring_width: f64,
    max_depth: usize,
}

fn compute_geometry(area: Rect) -> SunburstGeometry {
    let cx = area.x as f64 + area.width as f64 * 0.48;
    let cy_hi = (area.y as f64 + area.height as f64 * 0.52) * 2.0;
    let max_radius =
        ((area.width as f64 / 2.08).min(area.height as f64 * 2.0 / 2.2) - 1.0).max(5.0);
    let center_radius = (max_radius * 0.18).clamp(2.8, 5.5);
    let max_depth = if max_radius > 13.0 { 5 } else { 4 };
    let ring_width = ((max_radius - center_radius) / max_depth as f64).max(1.6);
    SunburstGeometry {
        cx,
        cy_hi,
        center_radius,
        ring_width,
        max_depth,
    }
}

fn draw_sunburst(
    canvas: &mut Canvas,
    app: &App,
    area: Rect,
    geometry: &SunburstGeometry,
    segments: &[Segment],
) {
    fill_rect(canvas, area, ' ', Style::bg(BG));
    if area.width < 32 || area.height < 14 {
        canvas.write(
            area.x + 2,
            area.y + 2,
            "terminal too small",
            Style::fg(MUTED),
        );
        return;
    }

    let focus = if app.focus == Focus::Map {
        "map *"
    } else {
        "map"
    };
    canvas.write(area.x + 3, area.y, focus, Style::fg(FAINT));

    let SunburstGeometry {
        cx,
        cy_hi,
        center_radius,
        ring_width,
        max_depth,
    } = *geometry;

    for y in area.y..area.y + area.height {
        for x in area.x..area.x + area.width {
            let upper = sample_sunburst_color(
                app,
                segments,
                x as f64 + 0.5,
                y as f64 * 2.0 + 0.5,
                cx,
                cy_hi,
                center_radius,
                ring_width,
                max_depth,
            );
            let lower = sample_sunburst_color(
                app,
                segments,
                x as f64 + 0.5,
                y as f64 * 2.0 + 1.5,
                cx,
                cy_hi,
                center_radius,
                ring_width,
                max_depth,
            );
            put_half_block(canvas, x, y, upper, lower);
        }
    }

    let root = app.tree.get(app.view_root);
    let size_str = format_size(root.size_bytes);
    let mut parts = size_str.splitn(2, ' ');
    let number = parts.next().unwrap_or_default();
    let unit = parts.next().unwrap_or_default();
    let cy = cy_hi / 2.0;
    canvas.write_centered(cx, cy - 0.5, number, Style::fg(WHITE).on_bg(HUB_BG).bold());
    canvas.write_centered(cx, cy + 0.6, unit, Style::fg(MUTED).on_bg(HUB_BG));

    let drop_y = area.y + area.height.saturating_sub(2);
    canvas.write(area.x + 4, drop_y, "◎", Style::fg(FAINT));
    canvas.write(
        area.x + 8,
        drop_y,
        "drag and drop files here to collect them",
        Style::fg(FAINT),
    );
}

fn draw_sidebar(canvas: &mut Canvas, app: &App, area: Rect, segments: &[Segment]) {
    fill_rect(canvas, area, ' ', Style::bg(BG));
    draw_vline(canvas, area.x, area.y, area.height, Style::fg(DIVIDER));

    let selected = app.tree.get(app.selected);
    let title_y = area.y + 2;
    canvas.write(
        area.x + 3,
        title_y,
        &truncate(
            &app.tree.get(app.view_root).name,
            area.width.saturating_sub(15) as usize,
        ),
        Style::fg(WHITE),
    );
    let root_size = format_size(app.tree.get(app.view_root).size_bytes);
    canvas.write_right(
        area.x + area.width - 2,
        title_y,
        &root_size,
        Style::fg(WHITE),
    );

    let children = app.visible_children();
    let list_limit = area.height.saturating_sub(15) as usize;
    let selected_index = app.selected_index().unwrap_or(0);
    let (list_start, visible_children) = visible_list_window(&children, selected_index, list_limit);
    let mut row = title_y + 2;
    if list_start > 0 && row < area.y + area.height {
        canvas.write(
            area.x + 5,
            row,
            &format!("↑ {} above", list_start),
            Style::fg(FAINT),
        );
        row += 1;
    }
    for child in visible_children.iter() {
        let node = app.tree.get(*child);
        let color = depth0_color(segments, *child);
        let style = if *child == app.selected {
            Style::fg(SELECTED_TEXT).on_bg(SELECTED).bold()
        } else {
            Style::fg(TEXT)
        };
        if *child == app.selected {
            fill_rect(
                canvas,
                Rect {
                    x: area.x + 1,
                    y: row,
                    width: area.width.saturating_sub(2),
                    height: 1,
                },
                ' ',
                Style::bg(SELECTED),
            );
        }
        let marker_style = if *child == app.selected {
            Style::fg(SELECTED_TEXT).on_bg(SELECTED).bold()
        } else {
            Style::fg(color)
        };
        let marker = if *child == app.selected { '▶' } else { '●' };
        canvas.put(area.x + 3, row, marker, marker_style);
        canvas.write(
            area.x + 5,
            row,
            &truncate(&node.name, area.width.saturating_sub(19) as usize),
            style,
        );
        canvas.write_right(
            area.x + area.width - 2,
            row,
            &format_size(node.size_bytes),
            style,
        );
        row += 1;
    }

    let hidden_after = children
        .len()
        .saturating_sub(list_start + visible_children.len());
    if hidden_after > 0 {
        canvas.write(
            area.x + 5,
            row,
            &format!("↓ {} below", hidden_after),
            Style::fg(FAINT),
        );
        row += 1;
    }

    row += 1;
    draw_hline(
        canvas,
        area.x + 3,
        row,
        area.width.saturating_sub(6),
        Style::fg(DIVIDER),
    );

    row += 2;
    canvas.write(area.x + 3, row, "selected", Style::fg(FAINT));
    row += 1;
    canvas.write(
        area.x + 3,
        row,
        &truncate(&selected.name, area.width.saturating_sub(6) as usize),
        Style::fg(WHITE).bold(),
    );
    row += 1;
    canvas.write(area.x + 3, row, selected.kind.icon(), Style::fg(MUTED));
    canvas.write(
        area.x + 8,
        row,
        &format_size(selected.size_bytes),
        Style::fg(TEXT),
    );
    row += 1;
    canvas.write(
        area.x + 3,
        row,
        &format!("{} items", app.tree.item_count(app.selected)),
        Style::fg(MUTED),
    );
    row += 1;
    canvas.write(
        area.x + 3,
        row,
        &truncate(
            &selected.path.display().to_string(),
            area.width.saturating_sub(6) as usize,
        ),
        Style::fg(FAINT),
    );

    let footer = area.y + area.height.saturating_sub(2);
    let focus_label = if app.focus == Focus::List {
        "list focus"
    } else {
        "map focus"
    };
    canvas.write(area.x + 3, footer, focus_label, Style::fg(FAINT));
}

fn depth0_color(segments: &[Segment], node: NodeId) -> Rgb {
    segments
        .iter()
        .find(|segment| segment.depth == 0 && segment.node == node)
        .map(|segment| segment.color)
        .unwrap_or(FAINT)
}

fn draw_status(canvas: &mut Canvas, app: &App, area: Rect) {
    fill_rect(canvas, area, ' ', Style::bg(BG));
    canvas.write(
        area.x + 3,
        area.y,
        "q quit   enter drill   backspace parent   arrows select   tab focus   r rescan   o open   ? help",
        Style::fg(MUTED),
    );
    canvas.write(
        area.x + 3,
        area.y + 1,
        &truncate(&app.message, area.width.saturating_sub(6) as usize),
        Style::fg(TEXT),
    );
}

fn visible_list_window(
    children: &[NodeId],
    selected_index: usize,
    list_limit: usize,
) -> (usize, &[NodeId]) {
    if children.is_empty() || list_limit == 0 {
        return (0, &[]);
    }

    let reserve_top = usize::from(selected_index > 0 && children.len() > list_limit);
    let reserve_bottom =
        usize::from(selected_index + 1 < children.len() && children.len() > list_limit);
    let visible_limit = list_limit
        .saturating_sub(reserve_top + reserve_bottom)
        .max(1);
    let half_window = visible_limit / 2;
    let max_start = children.len().saturating_sub(visible_limit);
    let start = selected_index.saturating_sub(half_window).min(max_start);
    let end = (start + visible_limit).min(children.len());

    (start, &children[start..end])
}

fn draw_help(canvas: &mut Canvas, width: u16, height: u16) {
    let popup = Rect {
        x: width / 2 - 25,
        y: height / 2 - 6,
        width: 50,
        height: 12,
    };
    fill_rect(canvas, popup, ' ', Style::bg(DIVIDER));
    outline(canvas, popup, Style::fg(FAINT).on_bg(DIVIDER));
    canvas.write(
        popup.x + 2,
        popup.y,
        " help ",
        Style::fg(WHITE).on_bg(DIVIDER).bold(),
    );
    let lines = [
        "q / Esc       quit",
        "Enter         drill into selected directory",
        "Backspace     go to parent",
        "Up/Down       move selection",
        "Left/Right    move sibling selection",
        "Tab           switch focus",
        "r             rescan root path",
        "o             open selected path",
        "d             delete placeholder, no hard delete",
        "?             close this help",
    ];
    for (idx, line) in lines.iter().enumerate() {
        canvas.write(
            popup.x + 2,
            popup.y + 1 + idx as u16,
            line,
            Style::fg(TEXT).on_bg(DIVIDER),
        );
    }
}

fn sunburst_segments(tree: &FileTree, root: NodeId, max_depth: usize) -> Vec<Segment> {
    let mut segments = Vec::new();
    collect_segments(tree, root, START_ANGLE, END_ANGLE, 0, max_depth, &mut segments);
    segments
}

/// Hue sweeps continuously by angular position (like a color wheel segment),
/// not by category identity — this matches how a single large wedge can visibly
/// drift from green to orange across its own arc. Depth lightens/desaturates.
fn segment_color(mid_angle: f64, depth: usize) -> Rgb {
    let t = ((mid_angle - START_ANGLE) / (END_ANGLE - START_ANGLE)).clamp(0.0, 1.0);
    let hue = HUE_START + t * HUE_SPAN;
    let lightness = (0.50 + depth as f64 * 0.085).min(0.86);
    let saturation = (0.62 - depth as f64 * 0.05).max(0.30);
    hsl(hue, saturation, lightness)
}

fn hsl(hue: f64, saturation: f64, lightness: f64) -> Rgb {
    let h = hue.rem_euclid(360.0) / 360.0;
    let s = saturation.clamp(0.0, 1.0);
    let l = lightness.clamp(0.0, 1.0);
    if s == 0.0 {
        let v = (l * 255.0).round() as u8;
        return (v, v, v);
    }
    let q = if l < 0.5 { l * (1.0 + s) } else { l + s - l * s };
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

#[allow(clippy::too_many_arguments)]
fn sample_sunburst_color(
    app: &App,
    segments: &[Segment],
    x: f64,
    y_hi: f64,
    cx: f64,
    cy_hi: f64,
    center_radius: f64,
    ring_width: f64,
    max_depth: usize,
) -> Option<Rgb> {
    let dx = x - cx;
    let dy = cy_hi - y_hi;
    let distance = (dx * dx + dy * dy).sqrt();

    if distance <= center_radius {
        return Some(HUB_BG);
    }

    let depth = ((distance - center_radius) / ring_width).floor() as usize;
    if depth >= max_depth {
        return None;
    }

    let inner = center_radius + depth as f64 * ring_width + RADIAL_GAP;
    let outer = center_radius + (depth + 1) as f64 * ring_width - RADIAL_GAP;
    let angle = dy.atan2(dx);
    let selected_segment = segments.iter().rev().find(|segment| {
        segment.node == app.selected
            && segment.depth == depth
            && angle_in_segment(angle, segment.start, segment.end)
    });

    if let Some(segment) = selected_segment
        && distance >= inner - 0.85
        && distance <= outer + 0.85
    {
        if distance < inner || distance > outer {
            return Some(SELECTED_EDGE);
        }

        return Some(selected_segment_color(
            segment, angle, distance, inner, outer,
        ));
    }

    if distance < inner || distance > outer {
        return None;
    }

    let segment = segments.iter().rev().find(|segment| {
        segment.depth == depth && angle_in_segment(angle, segment.start, segment.end)
    })?;

    if angular_edge_distance(angle, segment.start, segment.end) * distance < ANGULAR_SEAM_HALF {
        return None;
    }

    Some(segment.color)
}

fn selected_segment_color(
    segment: &Segment,
    angle: f64,
    distance: f64,
    inner: f64,
    outer: f64,
) -> Rgb {
    let radial_edge = (distance - inner).min(outer - distance);
    let angular_edge = angular_edge_distance(angle, segment.start, segment.end) * distance;
    if radial_edge <= 0.68 || angular_edge <= 0.68 {
        SELECTED_EDGE
    } else if radial_edge <= 1.22 || angular_edge <= 1.22 {
        SELECTED_TEXT
    } else {
        SELECTED
    }
}

fn put_half_block(canvas: &mut Canvas, x: u16, y: u16, upper: Option<Rgb>, lower: Option<Rgb>) {
    match (upper, lower) {
        (None, None) => {}
        (Some(color), Some(other)) if color == other => {
            canvas.put(x, y, ' ', Style::bg(color));
        }
        (Some(upper), Some(lower)) => {
            canvas.put(x, y, '▀', Style::fg(upper).on_bg(lower));
        }
        (Some(upper), None) => {
            canvas.put(x, y, '▀', Style::fg(upper).on_bg(BG));
        }
        (None, Some(lower)) => {
            canvas.put(x, y, '▄', Style::fg(lower).on_bg(BG));
        }
    }
}

fn collect_segments(
    tree: &FileTree,
    node: NodeId,
    start: f64,
    end: f64,
    depth: usize,
    max_depth: usize,
    segments: &mut Vec<Segment>,
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
            segments.push(Segment {
                node: child,
                start: cursor,
                end: child_end,
                depth,
                color,
            });
            collect_segments(tree, child, cursor, child_end, depth + 1, max_depth, segments);
        }
        cursor = child_end;
    }
}

fn angle_in_segment(mut angle: f64, start: f64, end: f64) -> bool {
    while angle < start {
        angle += 2.0 * PI;
    }
    angle >= start && angle <= end
}

fn angular_edge_distance(angle: f64, start: f64, end: f64) -> f64 {
    let mut normalized = angle;
    while normalized < start {
        normalized += 2.0 * PI;
    }
    (normalized - start).min(end - normalized).max(0.0)
}

fn draw_pill(canvas: &mut Canvas, x: u16, y: u16, label: &str, style: Style) {
    let text = format!(" {label} ");
    canvas.write(x, y, &text, style);
}

fn draw_hline(canvas: &mut Canvas, x: u16, y: u16, width: u16, style: Style) {
    for offset in 0..width {
        canvas.put(x + offset, y, '─', style);
    }
}

fn draw_vline(canvas: &mut Canvas, x: u16, y: u16, height: u16, style: Style) {
    for offset in 0..height {
        canvas.put(x, y + offset, '│', style);
    }
}

fn outline(canvas: &mut Canvas, area: Rect, style: Style) {
    if area.width < 2 || area.height < 2 {
        return;
    }

    let right = area.x + area.width - 1;
    let bottom = area.y + area.height - 1;
    for x in area.x..=right {
        canvas.put(x, area.y, '─', style);
        canvas.put(x, bottom, '─', style);
    }
    for y in area.y..=bottom {
        canvas.put(area.x, y, '│', style);
        canvas.put(right, y, '│', style);
    }
    canvas.put(area.x, area.y, '┌', style);
    canvas.put(right, area.y, '┐', style);
    canvas.put(area.x, bottom, '└', style);
    canvas.put(right, bottom, '┘', style);
}

fn fill_rect(canvas: &mut Canvas, area: Rect, ch: char, style: Style) {
    for y in area.y..area.y.saturating_add(area.height) {
        for x in area.x..area.x.saturating_add(area.width) {
            canvas.put(x, y, ch, style);
        }
    }
}

fn truncate(value: &str, width: usize) -> String {
    if width == 0 {
        return String::new();
    }

    let chars: Vec<_> = value.chars().collect();
    if chars.len() <= width {
        value.to_string()
    } else if width <= 1 {
        ".".to_string()
    } else {
        format!("{}.", chars[..width - 1].iter().collect::<String>())
    }
}

pub fn summary(tree: &FileTree) -> String {
    let root = tree.get(tree.root());
    let mut output = format!(
        "{}\nTotal: {} across {} items\n\n",
        root.path.display(),
        format_size(root.size_bytes),
        tree.item_count(tree.root())
    );
    output.push_str("Largest children:\n");
    for child in tree.children_sorted(tree.root()).into_iter().take(20) {
        let node = tree.get(child);
        output.push_str(&format!(
            "{:>10}  {} {}\n",
            format_size(node.size_bytes),
            node.kind.icon(),
            node.name
        ));
    }
    if !tree.errors().is_empty() {
        output.push_str("\nScan errors:\n");
        for error in tree.errors().iter().take(10) {
            output.push_str(&format!("{}: {}\n", error.path.display(), error.message));
        }
    }
    output
}

#[derive(Debug, Clone, Copy)]
struct Segment {
    node: NodeId,
    start: f64,
    end: f64,
    depth: usize,
    color: Rgb,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Style {
    fg: Option<Rgb>,
    bg: Option<Rgb>,
    bold: bool,
    dim: bool,
}

impl Style {
    const fn new(fg: Option<Rgb>, bg: Option<Rgb>, bold: bool, dim: bool) -> Self {
        Self { fg, bg, bold, dim }
    }

    const fn fg(color: Rgb) -> Self {
        Self::new(Some(color), None, false, false)
    }

    const fn bg(color: Rgb) -> Self {
        Self::new(None, Some(color), false, false)
    }

    const fn bold(mut self) -> Self {
        self.bold = true;
        self
    }

    const fn on_bg(mut self, color: Rgb) -> Self {
        self.bg = Some(color);
        self
    }
}

#[derive(Debug, Clone, Copy)]
struct Cell {
    ch: char,
    style: Style,
}

struct Canvas {
    width: u16,
    height: u16,
    cells: Vec<Cell>,
    default_style: Style,
}

impl Canvas {
    fn new(width: u16, height: u16, default_style: Style) -> Self {
        Self {
            width,
            height,
            cells: vec![
                Cell {
                    ch: ' ',
                    style: default_style,
                };
                width as usize * height as usize
            ],
            default_style,
        }
    }

    fn put(&mut self, x: u16, y: u16, ch: char, style: Style) {
        if x >= self.width || y >= self.height {
            return;
        }
        let index = y as usize * self.width as usize + x as usize;
        self.cells[index] = Cell { ch, style };
    }

    fn write(&mut self, x: u16, y: u16, text: &str, style: Style) {
        if y >= self.height || x >= self.width {
            return;
        }
        for (offset, ch) in text
            .chars()
            .take(self.width.saturating_sub(x) as usize)
            .enumerate()
        {
            self.put(x + offset as u16, y, ch, style);
        }
    }

    fn write_right(&mut self, right_x: u16, y: u16, text: &str, style: Style) {
        let width = text.chars().count() as u16;
        let x = right_x.saturating_sub(width);
        self.write(x, y, text, style);
    }

    fn write_centered(&mut self, cx: f64, cy: f64, text: &str, style: Style) {
        let width = text.chars().count() as f64;
        let x = (cx - width / 2.0).round().max(0.0) as u16;
        let y = cy.round().max(0.0) as u16;
        self.write(x, y, text, style);
    }

    fn finish(self) -> String {
        let mut output = String::new();
        let mut current = self.default_style;
        output.push_str(&ansi_style(current));

        for y in 0..self.height {
            for x in 0..self.width {
                let cell = self.cells[y as usize * self.width as usize + x as usize];
                if cell.style != current {
                    output.push_str(&ansi_style(cell.style));
                    current = cell.style;
                }
                output.push(cell.ch);
            }
            output.push_str("\x1b[0m");
            current = self.default_style;
            if y + 1 < self.height {
                output.push_str("\r\n");
                output.push_str(&ansi_style(current));
            }
        }
        output.push_str("\x1b[0m");
        output
    }
}

fn ansi_style(style: Style) -> String {
    let mut codes = vec!["0".to_string()];
    if style.bold {
        codes.push("1".to_string());
    }
    if style.dim {
        codes.push("2".to_string());
    }
    if let Some((r, g, b)) = style.fg {
        codes.push(format!("38;2;{r};{g};{b}"));
    }
    if let Some((r, g, b)) = style.bg {
        codes.push(format!("48;2;{r};{g};{b}"));
    }
    format!("\x1b[{}m", codes.join(";"))
}
