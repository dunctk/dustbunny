# Dustbunny

Dustbunny is a Rust terminal app for exploring disk usage with a visual, DaisyDisk-like workflow. It should feel closer to an interactive map than a table: scan a directory, see the biggest space consumers immediately, drill into folders, and delete or open targets only after a deliberate confirmation flow.

The implementation should start as a focused Ratatui application rather than a wrapper around a full existing UI. The scanner and layout engine should be independent from the terminal renderer so the core disk-usage model can be tested without running a TUI.

## Product Shape

The first usable version should answer three questions quickly:

1. What is using the most space under this path?
2. Which directories are worth drilling into?
3. What can I safely inspect, open, or delete next?

The primary screen should have:

- A visual map of disk usage, starting with a rectangular treemap because it maps cleanly to terminal cells.
- A directory tree or ranked list beside the map for keyboard-driven navigation.
- A details panel for the selected file or directory: name, full path, size, item count, modified time if cheap to gather, and scan state.
- A command/status bar with the current path, active mode, progress, and pending action.

Radial sunburst mode is a later visual mode, not the first milestone. A treemap will provide most of the value with less rendering risk.

## Recommended Stack

Use this as the initial Rust stack:

```toml
[dependencies]
ratatui = "0.30"
crossterm = "0.29"
jwalk = "0.8"
tui-tree-widget = "0.24"
color-eyre = "0.6"
humansize = "2"
```

Optional later dependencies:

```toml
ratatui-image = "11"
trash = "5"
open = "5"
```

Ratatui plus Crossterm is the right UI base because Dustbunny needs custom rendering, not form widgets. `jwalk` is a good scanner candidate because it supports parallel traversal and can feed progressive results into the UI. `tui-tree-widget` can handle the navigable directory pane while the project focuses custom work on the disk map.

Before creating `Cargo.toml`, check current crate versions with `cargo search` or `cargo add`; the versions above are planning anchors, not a lock.

## Architecture

Keep the project split into four layers:

```text
src/
  main.rs              # process setup and error handling
  app.rs               # app state, modes, commands, event dispatch
  scanner.rs           # filesystem traversal and size aggregation
  model.rs             # FileNode, ScanId, size summaries, selection state
  layout.rs            # treemap rectangle calculation
  ui/
    mod.rs             # root draw function
    treemap.rs         # custom Ratatui widget
    tree.rs            # directory tree/list pane
    details.rs         # selected item metadata
    status.rs          # footer/status bar
```

The scanner should emit updates instead of blocking until the whole tree is complete. The UI can initially show partial totals and mark incomplete branches as scanning. Keep filesystem side effects behind explicit commands so they are easy to audit and test.

## Core Data Model

The app needs a tree of scan nodes:

```rust
struct FileNode {
    id: NodeId,
    parent: Option<NodeId>,
    name: String,
    path: PathBuf,
    kind: NodeKind,
    size_bytes: u64,
    children: Vec<NodeId>,
    scan_state: ScanState,
}
```

Important derived views:

- Children sorted by `size_bytes` descending.
- Top-N largest files under the selected subtree.
- Breadcrumb path from root to selected node.
- Rectangle assignments for visible nodes in the treemap.

## Interaction Model

Start with predictable keyboard controls:

```text
q / Esc       quit or back out of transient mode
Enter         drill into selected directory
Backspace     go to parent
Up/Down       move selection in list/tree
Left/Right    move selection between visual siblings
Tab           switch focus between map and tree
r             rescan current root
o             open selected path with system handler
d             stage delete prompt for selected path
?             help overlay
```

Deletion should not be part of the first thin slice unless it uses a reversible trash flow. If hard delete is ever added, it should require a typed confirmation and be disabled by default.

## Milestones

### Milestone 1: Scanning CLI Skeleton

- Create a Cargo binary project.
- Accept a path argument, defaulting to the current directory.
- Traverse the directory tree and aggregate sizes.
- Print the largest immediate children and total size.
- Add unit tests for size aggregation on a temporary directory.

### Milestone 2: Static TUI

- Start Ratatui and Crossterm terminal setup.
- Render a static layout with map, list/tree, details, and status areas.
- Load a completed scan before entering the UI loop.
- Support quit, selection movement, and drill-down.

### Milestone 3: Treemap Widget

- Implement a deterministic squarified or slice-and-dice treemap layout.
- Render visible children with stable colors and size labels.
- Keep labels clipped to their rectangles.
- Add tests for rectangle coverage and no negative dimensions.

### Milestone 4: Progressive Scan

- Move traversal to a worker thread.
- Send scan updates over a channel.
- Show partial totals and progress in the status bar.
- Allow cancellation/rescan without corrupting app state.

### Milestone 5: Safe Actions

- Open selected path with the platform handler.
- Add trash-based delete with a confirmation prompt.
- Log or display completed actions.
- Never delete while scan state is incomplete for the selected node.

## First Build Slice

The next concrete step is to create the Rust app skeleton and make the scanner useful before chasing visuals:

1. Run `cargo init --bin --name dustbunny`.
2. Add `color-eyre`, `jwalk`, `humansize`, and `tempfile` for tests.
3. Implement `scanner::scan_path(path) -> Result<FileNodeTree>`.
4. Print a sorted summary from `main.rs`.
5. Add tests around nested directories, empty directories, and files that cannot be read.

Once that works, add Ratatui and build the first static UI around real scan data.

## Reference Links

- `dust` crate docs: https://docs.rs/lucamoller_dust/latest/dust/index.html
- Ratatui docs: https://docs.rs/ratatui/latest/ratatui/
- Custom Ratatui widgets: https://ratatui.rs/recipes/widgets/custom/
- Ratatui Canvas: https://docs.rs/ratatui/latest/ratatui/widgets/canvas/struct.Canvas.html
- ratatui-image: https://docs.rs/ratatui-image
- jwalk: https://docs.rs/jwalk/
- tui-tree-widget: https://ratatui.rs/showcase/third-party-widgets/
