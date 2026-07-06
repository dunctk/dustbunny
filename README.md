# DustBunny

<p align="center">
  <img src="img/logo.png" alt="DustBunny logo" width="260">
</p>

DustBunny is a Rust terminal disk-usage explorer inspired by DaisyDisk. It scans a directory, shows the largest space consumers in a radial sunburst-style TUI, and lets you drill through the tree from the keyboard.

<p align="center">
  <img src="img/tui-screenshot.svg" alt="DustBunny TUI showing a simulated interesting disk">
</p>

The screenshot above is generated from a simulated disk tree and refreshed automatically on every push, so it tracks the latest TUI renderer.

## Features

- Radial, DaisyDisk-like disk map rendered directly in the terminal.
- High-resolution half-block drawing for smoother arcs.
- Right-side size legend and selected item details.
- Animated ASCII splash screen while scanning.
- Keyboard navigation for selecting, drilling into, and backing out of folders.
- Non-interactive summary mode for quick checks.

## Install

You need a recent Rust toolchain.

```bash
cargo build --release
```

The binary will be available at:

```bash
target/release/dustbunny
```

## Usage

Launch the TUI for the current directory:

```bash
cargo run -- .
```

Scan another path:

```bash
cargo run -- /path/to/folder
```

Print a plain summary without launching the TUI:

```bash
cargo run -- --summary .
```

Regenerate the README demo screenshot:

```bash
cargo run -- --demo-screenshot img/tui-screenshot.svg
```

## Controls

```text
q / Esc       quit
Enter         drill into selected directory
Backspace     go to parent
Up/Down       move selection
Left/Right    move selection
Tab           switch focus
r             rescan root path
o             open selected path
?             help overlay
```

Deletion is intentionally not implemented yet.

## Status

This is an early working prototype. It currently uses a standard-library scanner and terminal renderer, with no external runtime dependencies.

## License

MIT
