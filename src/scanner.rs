use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::model::{FileNode, FileTree, NodeId, NodeKind, ScanError};

pub fn scan_path(path: impl AsRef<Path>) -> io::Result<FileTree> {
    let path = path.as_ref().canonicalize()?;
    let metadata = fs::symlink_metadata(&path)?;
    let root_kind = node_kind(&metadata);
    let root_name = path
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| path.display().to_string());

    let root = FileNode {
        id: NodeId(0),
        parent: None,
        name: root_name,
        path: path.clone(),
        kind: root_kind,
        size_bytes: 0,
        children: Vec::new(),
    };
    let mut tree = FileTree::new(root);
    let root_id = tree.root();
    let size = scan_node(&mut tree, root_id, &metadata);
    tree.get_mut(root_id).size_bytes = size;
    sort_all_children(&mut tree, root_id);
    Ok(tree)
}

fn scan_node(tree: &mut FileTree, id: NodeId, metadata: &fs::Metadata) -> u64 {
    let kind = node_kind(metadata);
    if kind != NodeKind::Directory {
        return metadata.len();
    }

    let path = tree.get(id).path.clone();
    let entries = match fs::read_dir(&path) {
        Ok(entries) => entries,
        Err(error) => {
            tree.add_error(ScanError {
                path,
                message: error.to_string(),
            });
            return 0;
        }
    };

    let mut total: u64 = 0;
    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(error) => {
                tree.add_error(ScanError {
                    path: path.clone(),
                    message: error.to_string(),
                });
                continue;
            }
        };

        let child_path = entry.path();
        let metadata = match fs::symlink_metadata(&child_path) {
            Ok(metadata) => metadata,
            Err(error) => {
                tree.add_error(ScanError {
                    path: child_path,
                    message: error.to_string(),
                });
                continue;
            }
        };

        let child_id = tree.add_node(FileNode {
            id: NodeId(0),
            parent: Some(id),
            name: display_name(&child_path),
            path: child_path,
            kind: node_kind(&metadata),
            size_bytes: 0,
            children: Vec::new(),
        });
        tree.get_mut(id).children.push(child_id);

        let child_size = scan_node(tree, child_id, &metadata);
        tree.get_mut(child_id).size_bytes = child_size;
        total = total.saturating_add(child_size);
    }

    total
}

fn display_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| PathBuf::from(path).display().to_string())
}

fn node_kind(metadata: &fs::Metadata) -> NodeKind {
    let file_type = metadata.file_type();
    if file_type.is_dir() {
        NodeKind::Directory
    } else if file_type.is_file() {
        NodeKind::File
    } else if file_type.is_symlink() {
        NodeKind::Symlink
    } else {
        NodeKind::Other
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
        assert_eq!(tree.get(tree.root()).size_bytes, 30);

        let a = tree
            .children_sorted(tree.root())
            .into_iter()
            .find(|id| tree.get(*id).name == "a")
            .unwrap();
        assert_eq!(tree.get(a).size_bytes, 25);

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn sorts_children_largest_first() {
        let root = test_dir("sorts_children_largest_first");
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("small.bin"), [0_u8; 1]).unwrap();
        fs::write(root.join("large.bin"), [0_u8; 9]).unwrap();

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
