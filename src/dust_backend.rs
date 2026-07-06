//! Dust-style filesystem scanner.
//!
//! This module is adapted from the scanner design used by `du-dust`
//! (https://github.com/bootandy/dust, Apache-2.0). The published `du-dust`
//! crate is binary-only, so DustBunny keeps a small local backend that follows
//! the same core approach: parallel traversal, allocated-size accounting, and
//! hard-link deduplication.

use std::cmp::Ordering;
use std::collections::HashSet;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use rayon::iter::ParallelBridge;
use rayon::prelude::ParallelIterator;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DustKind {
    Directory,
    File,
    Symlink,
    Other,
}

#[derive(Debug, Clone)]
pub struct DustNode {
    pub path: PathBuf,
    pub size_bytes: u64,
    pub kind: DustKind,
    pub children: Vec<DustNode>,
    inode_device: Option<(u64, u64)>,
}

#[derive(Debug, Clone)]
pub struct DustScan {
    pub root: DustNode,
    pub errors: Vec<DustError>,
}

#[derive(Debug, Clone)]
pub struct DustError {
    pub path: PathBuf,
    pub message: String,
}

pub fn scan(path: impl AsRef<Path>) -> io::Result<DustScan> {
    let path = path.as_ref().canonicalize()?;
    let mut errors = Vec::new();
    let root = walk(path, 0, &mut errors)?;
    let mut inodes = HashSet::new();
    let root = clean_inodes(root, &mut inodes)
        .ok_or_else(|| io::Error::other("scan root was filtered as a duplicate inode"))?;

    Ok(DustScan { root, errors })
}

fn walk(path: PathBuf, depth: usize, errors: &mut Vec<DustError>) -> io::Result<DustNode> {
    let metadata = fs::symlink_metadata(&path)?;
    let kind = kind_from_metadata(&metadata);
    let inode_device = inode_device(&metadata);

    if kind != DustKind::Directory {
        return Ok(DustNode {
            path,
            size_bytes: allocated_size(&metadata),
            kind,
            children: Vec::new(),
            inode_device,
        });
    }

    let entries = match fs::read_dir(&path) {
        Ok(entries) => entries,
        Err(error) => {
            errors.push(DustError {
                path: path.clone(),
                message: error.to_string(),
            });
            return Ok(DustNode {
                path,
                size_bytes: allocated_size(&metadata),
                kind,
                children: Vec::new(),
                inode_device,
            });
        }
    };

    let children_with_errors: Vec<_> = entries
        .par_bridge()
        .filter_map(|entry| match entry {
            Ok(entry) => Some(walk_child(entry.path(), depth + 1)),
            Err(error) => Some(Err(DustError {
                path: path.clone(),
                message: error.to_string(),
            })),
        })
        .collect();

    let mut children = Vec::new();
    for result in children_with_errors {
        match result {
            Ok((child, mut child_errors)) => {
                children.push(child);
                errors.append(&mut child_errors);
            }
            Err(error) => errors.push(error),
        }
    }

    let size_bytes = allocated_size(&metadata)
        .saturating_add(children.iter().map(|child| child.size_bytes).sum::<u64>());

    Ok(DustNode {
        path,
        size_bytes,
        kind,
        children,
        inode_device,
    })
}

fn walk_child(path: PathBuf, depth: usize) -> Result<(DustNode, Vec<DustError>), DustError> {
    let mut errors = Vec::new();
    match walk(path.clone(), depth, &mut errors) {
        Ok(node) => Ok((node, errors)),
        Err(error) => Err(DustError {
            path,
            message: error.to_string(),
        }),
    }
}

fn clean_inodes(node: DustNode, inodes: &mut HashSet<(u64, u64)>) -> Option<DustNode> {
    if let Some(id) = node.inode_device
        && !inodes.insert(id)
    {
        return None;
    }

    let DustNode {
        path,
        kind,
        children,
        inode_device,
        ..
    } = node;
    let mut children = children;
    children.sort_by(sort_by_inode);
    let children: Vec<_> = children
        .into_iter()
        .filter_map(|child| clean_inodes(child, inodes))
        .collect();

    let size_bytes = allocated_size_for_path(&path)
        .saturating_add(children.iter().map(|child| child.size_bytes).sum::<u64>());

    Some(DustNode {
        path,
        size_bytes,
        kind,
        children,
        inode_device,
    })
}

fn sort_by_inode(a: &DustNode, b: &DustNode) -> Ordering {
    match (a.inode_device, b.inode_device) {
        (Some(x), Some(y)) => x.1.cmp(&y.1).then_with(|| x.0.cmp(&y.0)),
        (Some(_), None) => Ordering::Greater,
        (None, Some(_)) => Ordering::Less,
        (None, None) => a.path.cmp(&b.path),
    }
}

fn allocated_size_for_path(path: &Path) -> u64 {
    fs::symlink_metadata(path)
        .map(|metadata| allocated_size(&metadata))
        .unwrap_or(0)
}

fn kind_from_metadata(metadata: &fs::Metadata) -> DustKind {
    let file_type = metadata.file_type();
    if file_type.is_dir() {
        DustKind::Directory
    } else if file_type.is_file() {
        DustKind::File
    } else if file_type.is_symlink() {
        DustKind::Symlink
    } else {
        DustKind::Other
    }
}

#[cfg(target_family = "unix")]
fn allocated_size(metadata: &fs::Metadata) -> u64 {
    use std::os::unix::fs::MetadataExt;

    let file_size = metadata.len();
    let block_size = metadata.blksize().max(1);
    let target_size = file_size.div_ceil(block_size) * block_size;
    let reported_size = metadata.blocks() * 512;
    let pre_allocation_buffer = block_size * 65_536;
    let max_size = target_size.saturating_add(pre_allocation_buffer);

    if reported_size > max_size {
        target_size
    } else {
        reported_size
    }
}

#[cfg(not(target_family = "unix"))]
fn allocated_size(metadata: &fs::Metadata) -> u64 {
    metadata.len()
}

#[cfg(target_family = "unix")]
fn inode_device(metadata: &fs::Metadata) -> Option<(u64, u64)> {
    use std::os::unix::fs::MetadataExt;
    Some((metadata.ino(), metadata.dev()))
}

#[cfg(not(target_family = "unix"))]
fn inode_device(_metadata: &fs::Metadata) -> Option<(u64, u64)> {
    None
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;

    #[cfg(target_family = "unix")]
    #[test]
    fn deduplicates_hard_links() {
        let root = test_dir("deduplicates_hard_links");
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("original.bin"), [0_u8; 9000]).unwrap();
        fs::hard_link(root.join("original.bin"), root.join("linked.bin")).unwrap();

        let scan = scan(&root).unwrap();
        assert_eq!(scan.root.children.len(), 1);

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
