use std::path::PathBuf;
use std::process::Command;

use crate::model::{FileTree, NodeId, NodeKind};
use crate::scanner;
use crate::terminal::Key;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    Map,
    List,
}

pub struct App {
    pub tree: FileTree,
    pub root_path: PathBuf,
    pub view_root: NodeId,
    pub selected: NodeId,
    pub focus: Focus,
    pub show_help: bool,
    pub should_quit: bool,
    pub message: String,
}

impl App {
    pub fn new(tree: FileTree, root_path: PathBuf) -> Self {
        let root = tree.root();
        let selected = tree.children_sorted(root).first().copied().unwrap_or(root);
        Self {
            tree,
            root_path,
            view_root: root,
            selected,
            focus: Focus::List,
            show_help: false,
            should_quit: false,
            message: "ready".to_string(),
        }
    }

    pub fn handle_key(&mut self, key: Key) {
        match key {
            Key::Char('q') | Key::Esc => self.should_quit = true,
            Key::Char('?') => self.show_help = !self.show_help,
            Key::Tab => {
                self.focus = match self.focus {
                    Focus::Map => Focus::List,
                    Focus::List => Focus::Map,
                };
            }
            Key::Up => self.select_delta(-1),
            Key::Down => self.select_delta(1),
            Key::Left => self.select_delta(-1),
            Key::Right => self.select_delta(1),
            Key::Enter => self.drill_in(),
            Key::Backspace => self.go_parent(),
            Key::Char('r') => self.rescan(),
            Key::Char('o') => self.open_selected(),
            Key::Char('d') => {
                self.message = "delete is intentionally not implemented yet; use your file manager"
                    .to_string();
            }
            _ => {}
        }
    }

    pub fn visible_children(&self) -> Vec<NodeId> {
        self.tree.children_sorted(self.view_root)
    }

    pub fn selected_index(&self) -> Option<usize> {
        self.visible_children()
            .iter()
            .position(|child| *child == self.selected)
    }

    fn select_delta(&mut self, delta: isize) {
        let children = self.visible_children();
        if children.is_empty() {
            self.selected = self.view_root;
            return;
        }

        let current = self.selected_index().unwrap_or(0) as isize;
        let last = children.len() as isize - 1;
        let next = (current + delta).clamp(0, last) as usize;
        self.selected = children[next];
    }

    fn drill_in(&mut self) {
        let selected = self.tree.get(self.selected);
        if selected.kind != NodeKind::Directory {
            self.message = format!("{} is not a directory", selected.name);
            return;
        }

        self.view_root = self.selected;
        self.selected = self
            .tree
            .children_sorted(self.view_root)
            .first()
            .copied()
            .unwrap_or(self.view_root);
        self.message = "drilled in".to_string();
    }

    fn go_parent(&mut self) {
        if let Some(parent) = self.tree.get(self.view_root).parent {
            self.selected = self.view_root;
            self.view_root = parent;
            self.message = "went to parent".to_string();
        } else {
            self.message = "already at scan root".to_string();
        }
    }

    fn rescan(&mut self) {
        match scanner::scan_path(&self.root_path) {
            Ok(tree) => {
                *self = Self::new(tree, self.root_path.clone());
                self.message = "rescan complete".to_string();
            }
            Err(error) => {
                self.message = format!("rescan failed: {error}");
            }
        }
    }

    fn open_selected(&mut self) {
        let path = self.tree.get(self.selected).path.clone();
        let result = if cfg!(target_os = "macos") {
            Command::new("open").arg(&path).spawn()
        } else if cfg!(target_os = "windows") {
            Command::new("cmd").args(["/C", "start"]).arg(&path).spawn()
        } else {
            Command::new("xdg-open").arg(&path).spawn()
        };

        match result {
            Ok(_) => self.message = format!("opened {}", path.display()),
            Err(error) => self.message = format!("open failed: {error}"),
        }
    }
}
