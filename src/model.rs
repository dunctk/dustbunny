use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeId(pub usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeKind {
    Directory,
    File,
    Symlink,
    Other,
}

impl NodeKind {
    pub fn icon(self) -> &'static str {
        match self {
            Self::Directory => "[D]",
            Self::File => "[F]",
            Self::Symlink => "[L]",
            Self::Other => "[?]",
        }
    }
}

#[derive(Debug, Clone)]
pub struct FileNode {
    pub id: NodeId,
    pub parent: Option<NodeId>,
    pub name: String,
    pub path: PathBuf,
    pub kind: NodeKind,
    pub size_bytes: u64,
    pub children: Vec<NodeId>,
}

#[derive(Debug, Clone)]
pub struct ScanError {
    pub path: PathBuf,
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct FileTree {
    nodes: Vec<FileNode>,
    root: NodeId,
    errors: Vec<ScanError>,
}

impl FileTree {
    pub fn new(root_node: FileNode) -> Self {
        Self {
            nodes: vec![root_node],
            root: NodeId(0),
            errors: Vec::new(),
        }
    }

    pub fn root(&self) -> NodeId {
        self.root
    }

    pub fn get(&self, id: NodeId) -> &FileNode {
        &self.nodes[id.0]
    }

    pub fn get_mut(&mut self, id: NodeId) -> &mut FileNode {
        &mut self.nodes[id.0]
    }

    pub fn add_node(&mut self, mut node: FileNode) -> NodeId {
        let id = NodeId(self.nodes.len());
        node.id = id;
        self.nodes.push(node);
        id
    }

    pub fn add_error(&mut self, error: ScanError) {
        self.errors.push(error);
    }

    pub fn errors(&self) -> &[ScanError] {
        &self.errors
    }

    pub fn children_sorted(&self, id: NodeId) -> Vec<NodeId> {
        let mut children = self.get(id).children.clone();
        children.sort_by(|left, right| {
            let left = self.get(*left);
            let right = self.get(*right);
            right
                .size_bytes
                .cmp(&left.size_bytes)
                .then_with(|| left.name.cmp(&right.name))
        });
        children
    }

    pub fn item_count(&self, id: NodeId) -> usize {
        self.get(id)
            .children
            .iter()
            .map(|child| 1 + self.item_count(*child))
            .sum()
    }

    pub fn breadcrumb(&self, id: NodeId) -> String {
        let mut parts = Vec::new();
        let mut current = Some(id);

        while let Some(node_id) = current {
            let node = self.get(node_id);
            parts.push(node.name.clone());
            current = node.parent;
        }

        parts.reverse();
        parts.join(" / ")
    }
}

pub fn format_size(bytes: u64) -> String {
    const UNITS: [&str; 6] = ["B", "KB", "MB", "GB", "TB", "PB"];
    let mut value = bytes as f64;
    let mut unit = 0;

    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }

    if unit == 0 {
        format!("{} {}", bytes, UNITS[unit])
    } else if value >= 10.0 {
        format!("{value:.0} {}", UNITS[unit])
    } else {
        format!("{value:.1} {}", UNITS[unit])
    }
}
