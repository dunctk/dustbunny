use std::path::PathBuf;

use crate::app::App;
use crate::model::{FileNode, FileTree, NodeId, NodeKind};

const GB: u64 = 1024 * 1024 * 1024;
const MB: u64 = 1024 * 1024;

pub fn app() -> App {
    let root_path = PathBuf::from("/Volumes/DustBunny Demo");
    let root = FileNode {
        id: NodeId(0),
        parent: None,
        name: "DustBunny Demo".to_string(),
        path: root_path.clone(),
        kind: NodeKind::Directory,
        size_bytes: 512 * GB,
        children: Vec::new(),
    };
    let mut tree = FileTree::new(root);
    let root_id = tree.root();

    let users = add_dir(&mut tree, root_id, "Users", 198 * GB);
    let projects = add_dir(&mut tree, root_id, "Projects", 96 * GB);
    let containers = add_dir(&mut tree, root_id, "Containers", 72 * GB);
    let apps = add_dir(&mut tree, root_id, "Applications", 48 * GB);
    let system = add_dir(&mut tree, root_id, "System", 42 * GB);
    let media = add_dir(&mut tree, root_id, "Media Cache", 31 * GB);
    let library = add_dir(&mut tree, root_id, "Library", 18 * GB);
    let hidden = add_dir(&mut tree, root_id, "hidden space", 7 * GB);

    let dunc = add_dir(&mut tree, users, "dunc", 122 * GB);
    add_dir(&mut tree, users, "Shared", 38 * GB);
    add_dir(&mut tree, users, "Downloads", 24 * GB);
    add_file(&mut tree, users, "photo-library.photoslibrary", 14 * GB);

    let rust = add_dir(&mut tree, projects, "rust", 35 * GB);
    add_dir(&mut tree, projects, "video-renders", 29 * GB);
    let node_modules = add_dir(&mut tree, projects, "node_modules", 21 * GB);
    add_file(&mut tree, projects, "archive.tar.zst", 11 * GB);

    add_dir(&mut tree, containers, "Docker.raw", 44 * GB);
    let postgres = add_dir(&mut tree, containers, "postgres-volumes", 15 * GB);
    add_dir(&mut tree, containers, "build-cache", 13 * GB);

    add_file(&mut tree, apps, "Xcode.app", 18 * GB);
    add_file(&mut tree, apps, "Blender.app", 9 * GB);
    add_file(&mut tree, apps, "Steam.app", 8 * GB);
    add_dir(&mut tree, apps, "Utilities", 13 * GB);

    add_dir(&mut tree, system, "Developer", 19 * GB);
    add_dir(&mut tree, system, "iOS Support", 11 * GB);
    add_dir(&mut tree, system, "Library", 8 * GB);
    add_file(&mut tree, system, "sleepimage", 4 * GB);

    add_file(&mut tree, media, "remotion-cache", 12 * GB);
    add_file(&mut tree, media, "podcast-stems", 9 * GB);
    add_file(&mut tree, media, "screen-recordings", 7 * GB);
    add_file(&mut tree, media, "waveforms", 3 * GB);

    let caches = add_dir(&mut tree, library, "Caches", 9 * GB);
    add_dir(&mut tree, library, "Application Support", 6 * GB);
    add_file(&mut tree, library, "Safari.db", 2 * GB);
    add_file(&mut tree, library, "tiny leftovers", 900 * MB);

    add_file(&mut tree, hidden, "purgeable snapshots", 4 * GB);
    add_file(&mut tree, hidden, "local time machine", 2 * GB);
    add_file(&mut tree, hidden, "smaller objects", 850 * MB);

    // A little extra nesting so the sunburst's outer rings aren't empty, but kept
    // chunky (few, large children per level) — this renderer only has ~140 columns
    // to work with, so many thin slivers alias into speckly noise instead of clean bands.
    let dunc_library = add_dir(&mut tree, dunc, "Library", 70 * GB);
    add_dir(&mut tree, dunc, "Documents", 52 * GB);

    add_file(&mut tree, dunc_library, "Caches", 45 * GB);
    add_file(&mut tree, dunc_library, "Application Support", 25 * GB);

    add_file(&mut tree, rust, "target", 20 * GB);
    add_file(&mut tree, rust, "other", 15 * GB);

    add_file(&mut tree, node_modules, "large packages", 16 * GB);
    add_file(&mut tree, node_modules, "other packages", 5 * GB);

    add_file(&mut tree, postgres, "pgdata", 10 * GB);
    add_file(&mut tree, postgres, "wal", 5 * GB);

    add_file(&mut tree, caches, "GPUCache", 4 * GB);
    add_file(&mut tree, caches, "Code Cache", 3 * GB);
    add_file(&mut tree, caches, "Others", 2 * GB);

    App::new(tree, root_path)
}

fn add_dir(tree: &mut FileTree, parent: NodeId, name: &str, size_bytes: u64) -> NodeId {
    add_node(tree, parent, name, NodeKind::Directory, size_bytes)
}

fn add_file(tree: &mut FileTree, parent: NodeId, name: &str, size_bytes: u64) -> NodeId {
    add_node(tree, parent, name, NodeKind::File, size_bytes)
}

fn add_node(
    tree: &mut FileTree,
    parent: NodeId,
    name: &str,
    kind: NodeKind,
    size_bytes: u64,
) -> NodeId {
    let path = tree.get(parent).path.join(name);
    let id = tree.add_node(FileNode {
        id: NodeId(0),
        parent: Some(parent),
        name: name.to_string(),
        path,
        kind,
        size_bytes,
        children: Vec::new(),
    });
    tree.get_mut(parent).children.push(id);
    id
}
