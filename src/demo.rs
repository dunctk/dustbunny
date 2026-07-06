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

    add_dir(&mut tree, users, "dunc", 122 * GB);
    add_dir(&mut tree, users, "Shared", 38 * GB);
    add_dir(&mut tree, users, "Downloads", 24 * GB);
    add_file(&mut tree, users, "photo-library.photoslibrary", 14 * GB);

    add_dir(&mut tree, projects, "rust", 35 * GB);
    add_dir(&mut tree, projects, "video-renders", 29 * GB);
    add_dir(&mut tree, projects, "node_modules", 21 * GB);
    add_file(&mut tree, projects, "archive.tar.zst", 11 * GB);

    add_dir(&mut tree, containers, "Docker.raw", 44 * GB);
    add_dir(&mut tree, containers, "postgres-volumes", 15 * GB);
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

    add_dir(&mut tree, library, "Caches", 9 * GB);
    add_dir(&mut tree, library, "Application Support", 6 * GB);
    add_file(&mut tree, library, "Safari.db", 2 * GB);
    add_file(&mut tree, library, "tiny leftovers", 900 * MB);

    add_file(&mut tree, hidden, "purgeable snapshots", 4 * GB);
    add_file(&mut tree, hidden, "local time machine", 2 * GB);
    add_file(&mut tree, hidden, "smaller objects", 850 * MB);

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
