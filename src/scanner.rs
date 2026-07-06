use std::io;
use std::path::{Path, PathBuf};

use crate::dust_backend::{self, DustKind, DustNode};
use crate::model::{FileNode, FileTree, NodeId, NodeKind, ScanError};

pub fn scan_path(path: impl AsRef<Path>) -> io::Result<FileTree> {
    let scan = dust_backend::scan(path)?;
    let root = build_root(&scan.root);
    let mut tree = FileTree::new(root);
    let root_id = tree.root();

    for child in &scan.root.children {
        add_dust_node(&mut tree, root_id, child);
    }

    for error in scan.errors {
        tree.add_error(ScanError {
            path: error.path,
            message: error.message,
        });
    }

    sort_all_children(&mut tree, root_id);
    Ok(tree)
}

fn display_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| PathBuf::from(path).display().to_string())
}

fn build_root(node: &DustNode) -> FileNode {
    FileNode {
        id: NodeId(0),
        parent: None,
        name: display_name(&node.path),
        path: node.path.clone(),
        kind: node_kind(node.kind),
        size_bytes: node.size_bytes,
        children: Vec::new(),
    }
}

fn add_dust_node(tree: &mut FileTree, parent: NodeId, node: &DustNode) -> NodeId {
    let id = tree.add_node(FileNode {
        id: NodeId(0),
        parent: Some(parent),
        name: display_name(&node.path),
        path: node.path.clone(),
        kind: node_kind(node.kind),
        size_bytes: node.size_bytes,
        children: Vec::new(),
    });
    tree.get_mut(parent).children.push(id);

    for child in &node.children {
        add_dust_node(tree, id, child);
    }

    id
}

fn node_kind(kind: DustKind) -> NodeKind {
    match kind {
        DustKind::Directory => NodeKind::Directory,
        DustKind::File => NodeKind::File,
        DustKind::Symlink => NodeKind::Symlink,
        DustKind::Other => NodeKind::Other,
    }
}

fn sort_all_children(tree: &mut FileTree, id: NodeId) {
    let sorted = tree.children_sorted(id);
    tree.get_mut(id).children = sorted.clone();
    for child in sorted {
        sort_all_children(tree, child);
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;

    #[test]
    fn aggregates_nested_directory_sizes() {
        let root = test_dir("aggregates_nested_directory_sizes");
        fs::create_dir_all(root.join("a/b")).unwrap();
        fs::write(root.join("a/one.bin"), [0_u8; 10]).unwrap();
        fs::write(root.join("a/b/two.bin"), [0_u8; 15]).unwrap();
        fs::write(root.join("root.bin"), [0_u8; 5]).unwrap();

        let tree = scan_path(&root).unwrap();
        assert!(tree.get(tree.root()).size_bytes >= 30);

        let a = tree
            .children_sorted(tree.root())
            .into_iter()
            .find(|id| tree.get(*id).name == "a")
            .unwrap();
        assert!(tree.get(a).size_bytes >= 25);

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn sorts_children_largest_first() {
        let root = test_dir("sorts_children_largest_first");
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("small.bin"), [0_u8; 1]).unwrap();
        fs::write(root.join("large.bin"), [0_u8; 9000]).unwrap();

        let tree = scan_path(&root).unwrap();
        let names: Vec<_> = tree
            .children_sorted(tree.root())
            .iter()
            .map(|id| tree.get(*id).name.as_str())
            .collect();
        assert_eq!(names, ["large.bin", "small.bin"]);

        fs::remove_dir_all(root).unwrap();
    }

    fn test_dir(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("dustbunny-{name}-{nanos}"))
    }
}
