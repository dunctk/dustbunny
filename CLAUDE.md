# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

DustBunny is a Rust terminal disk-usage explorer inspired by DaisyDisk: scan a directory, see a
radial sunburst of what's using space, drill into folders from the keyboard. Everything —
terminal control, ANSI rendering, and the scanner — is hand-rolled; there is no `ratatui` /
`crossterm` dependency despite what `idea.md` (the original planning doc) proposes. Treat
`idea.md` as historical intent, not a description of the current implementation.

## Commands

```bash
cargo build                 # debug build
cargo build --release       # release build, binary at target/release/dustbunny
cargo run -- .              # launch the interactive TUI on the current directory
cargo run -- /path          # launch on another path
cargo run -- --summary .    # non-interactive plain-text summary (no TUI)
cargo run -- --demo-screenshot img/tui-screenshot.svg   # regenerate the README demo SVG
cargo test                  # unit tests (scanner.rs, dust_backend.rs)
cargo test aggregates_nested_directory_sizes   # run a single test by name
cargo clippy                # lint; keep this clean, CI/dev workflow assumes no warnings
```

There's no `rustfmt.toml`/custom lint config — defaults apply.

A GitHub Actions workflow (`.github/workflows/update-screenshot.yml`) runs
`cargo run -- --demo-screenshot img/tui-screenshot.svg` on every push to `main` and auto-commits
the result if it changed. So the committed `img/tui-screenshot.svg` should always match what
`demo.rs` + the current renderer produce — if you change rendering or the demo fixture, regenerate
it locally too so review diffs make sense before CI does it for you.

`dd-ref.jpg` (a DaisyDisk reference screenshot used for visual comparisons) is gitignored — it's a
local-only reference asset, not part of the repo.

## Architecture

Everything lives flat in `src/`, no submodules:

- `main.rs` — arg parsing and the three entry modes: interactive TUI, `--summary`, `--demo-screenshot`.
- `dust_backend.rs` — the actual filesystem walker. A local reimplementation of `du-dust`'s design
  (parallel traversal via `rayon`, block-size-aware allocated-size accounting, hard-link dedup via
  `(inode, device)` tracking) because the real `du-dust` crate is binary-only. Produces a
  `DustNode` tree.
- `scanner.rs` — converts a `DustNode` tree into the app's `model::FileTree`, sorts children by
  size descending, and is the thing the unit tests exercise (`scan_path`).
- `model.rs` — `FileTree` is a flat `Vec<FileNode>` arena addressed by `NodeId(usize)`, not a real
  pointer-based tree; `NodeId(0)` is always the root. `children_sorted` is the canonical
  size-desc/name-tiebreak ordering used everywhere (sidebar list, sunburst slice order).
- `app.rs` — `App` holds `tree`, `view_root` (current drill-down position), `selected`, and
  `focus` (Map/List). Key handling and drill-down/parent navigation live here.
- `terminal.rs` — raw-mode terminal control by shelling out to `stty` (no `crossterm`). Reads
  single bytes/escape sequences for input; `Terminal::drop` restores the original mode.
- `splash.rs` — spinner shown while `scanner::scan_path` runs on a background thread.
- `ui.rs` — the entire renderer. `render(app, width, height) -> String` builds a `Canvas` of
  `Cell { ch, style }` and serializes it straight to ANSI (`\x1b[38;2;r;g;b...`) — there is no
  ratatui widget tree. This is the file to read before touching anything visual.
- `screenshot.rs` — reuses `ui::render()` for a fixed `COLS`x`ROWS` grid, then **re-parses its own
  ANSI output** with a tiny SGR parser and rasterizes cells into an SVG for the README.
- `demo.rs` — builds a synthetic `FileTree` fixture used only by `--demo-screenshot`.

### Keeping the sunburst looking right

This renderer was tuned by rendering `img/tui-screenshot.svg` through headless Chrome and
comparing pixel-for-pixel against a DaisyDisk reference screenshot. The look depends on a few
non-obvious invariants in `ui.rs` — preserve them when touching sunburst code:

- **Hue is a continuous function of angle, not of category identity.** `segment_color()` picks
  hue from where a segment sits between `START_ANGLE` and `END_ANGLE`, then depth only adjusts
  HSL lightness/saturation. This deliberately lets one large wedge drift color across its own arc
  (e.g. green near the top, orange lower down), matching how DaisyDisk actually colors segments.
  Don't switch back to a fixed per-category palette — that was the original approach and looked
  wrong (index-based `PALETTE[i % n]` plus subtracting from an xterm-256 index produced
  hue-shifted, blotchy rings).
- **Colors are 24-bit truecolor** (`Style { fg: Option<(u8,u8,u8)>, bg: Option<(u8,u8,u8)> }`),
  not the 256-color palette. `screenshot.rs::apply_sgr` only understands `38;2;r;g;b` /
  `48;2;r;g;b` sequences — if `ui.rs::ansi_style` ever emits a different SGR form, the SVG
  generator needs a matching update or styling will silently disappear from the README image.
- **`RADIAL_GAP` and `ANGULAR_SEAM_HALF`** carve the dark seams between rings/slices at
  render-time (not baked into segment boundaries). They're tuned against the geometry in
  `compute_geometry` (cell resolution, `ring_width`, etc.) — pushing them up without checking a
  tree with many small leaf nodes will eat narrow segments entirely (this happened during
  development: a system folder's small children vanished because the seam width exceeded their
  angular width). If colors seem to be missing patches, check whether a segment is thinner than
  the seam margin before assuming it's a data bug.
- **No text labels are drawn inside the wheel.** DaisyDisk itself doesn't label segments in the
  wheel — only the side legend does. Identification happens entirely through the sidebar's
  colored bullets, which look up their color via `depth0_color()` matching a segment by
  `(depth == 0, node)`. Re-adding in-wheel labels previously caused unreadable overlapping text
  for adjacent slices; don't reintroduce them without real collision handling.
- `demo.rs`'s fixture tree is intentionally nested 4-5 levels deep in a few branches (e.g.
  `Users/dunc/Library/Caches/...`) so the outer rings in the demo screenshot actually have data —
  a shallow fixture leaves the outer rings empty even though the geometry reserves space for
  `max_depth` rings (see `compute_geometry`'s `max_depth`).

## Testing notes

Tests live inline (`#[cfg(test)] mod tests` in `scanner.rs` and `dust_backend.rs`) and operate on
real temp directories created under `std::env::temp_dir()` (no `tempfile` crate). There are no
tests for `ui.rs` — rendering correctness is verified visually (regenerate the demo SVG, render it
with headless Chrome, compare).
