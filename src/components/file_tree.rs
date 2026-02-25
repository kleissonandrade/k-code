use std::collections::HashSet;
use std::path::{Path, PathBuf};

use ignore::WalkBuilder;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::icons::{icon_for_directory, icon_for_directory_special, icon_for_extension};
use crate::theme::Theme;

#[derive(Debug, Clone)]
pub struct FileTreeEntry {
    pub path: PathBuf,
    pub name: String,
    pub depth: usize,
    pub is_dir: bool,
}

pub struct FileTreeComponent {
    pub root: PathBuf,
    pub entries: Vec<FileTreeEntry>,
    pub selected: usize,
    pub scroll_offset: usize,
    pub expanded: HashSet<PathBuf>,
    pub visible: bool,
}

impl FileTreeComponent {
    pub fn new(root: PathBuf) -> Self {
        let mut tree = Self {
            root: root.clone(),
            entries: Vec::new(),
            selected: 0,
            scroll_offset: 0,
            expanded: HashSet::new(),
            visible: true,
        };
        tree.expanded.insert(root.clone());
        tree.refresh();
        tree
    }

    pub fn refresh(&mut self) {
        self.entries.clear();
        self.build_entries(&self.root.clone(), 0);
    }

    fn build_entries(&mut self, dir: &Path, depth: usize) {
        let mut children: Vec<PathBuf> = Vec::new();

        let walker = WalkBuilder::new(dir)
            .max_depth(Some(1))
            .hidden(true)
            .git_ignore(true)
            .sort_by_file_name(|a, b| {
                let a_is_dir = a.to_str().map(|s| !s.contains('.')).unwrap_or(true);
                let b_is_dir = b.to_str().map(|s| !s.contains('.')).unwrap_or(true);
                match (a_is_dir, b_is_dir) {
                    (true, false) => std::cmp::Ordering::Less,
                    (false, true) => std::cmp::Ordering::Greater,
                    _ => a.cmp(b),
                }
            })
            .build();

        for entry in walker.flatten() {
            let path = entry.path().to_path_buf();
            if path == dir {
                continue;
            }
            children.push(path);
        }

        // Sort: dirs first, then files, alphabetically
        children.sort_by(|a, b| {
            let a_is_dir = a.is_dir();
            let b_is_dir = b.is_dir();
            match (a_is_dir, b_is_dir) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.file_name().cmp(&b.file_name()),
            }
        });

        for child in children {
            let name = child
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string();

            // Skip hidden files
            if name.starts_with('.') && name != ".gitignore" && name != ".env" {
                continue;
            }

            let is_dir = child.is_dir();
            self.entries.push(FileTreeEntry {
                path: child.clone(),
                name,
                depth,
                is_dir,
            });

            if is_dir && self.expanded.contains(&child) {
                self.build_entries(&child, depth + 1);
            }
        }
    }

    pub fn toggle_expand(&mut self) {
        if let Some(entry) = self.entries.get(self.selected) {
            if entry.is_dir {
                let path = entry.path.clone();
                if self.expanded.contains(&path) {
                    self.expanded.remove(&path);
                } else {
                    self.expanded.insert(path);
                }
                self.refresh();
            }
        }
    }

    pub fn selected_path(&self) -> Option<&Path> {
        self.entries.get(self.selected).map(|e| e.path.as_path())
    }

    pub fn selected_entry(&self) -> Option<&FileTreeEntry> {
        self.entries.get(self.selected)
    }

    pub fn move_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    pub fn move_down(&mut self) {
        if self.selected + 1 < self.entries.len() {
            self.selected += 1;
        }
    }

    pub fn render(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let visible_height = area.height.saturating_sub(2) as usize;

        // Adjust scroll
        let scroll = if self.selected < self.scroll_offset {
            self.selected
        } else if self.selected >= self.scroll_offset + visible_height {
            self.selected.saturating_sub(visible_height - 1)
        } else {
            self.scroll_offset
        };

        let title = self
            .root
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("k-code");

        let block = Block::default()
            .title(format!(" {} ", title))
            .title_style(Style::default().fg(theme.ui.accent).add_modifier(Modifier::BOLD))
            .borders(Borders::RIGHT)
            .border_style(Style::default().fg(theme.ui.border))
            .style(Style::default().bg(theme.ui.file_tree_bg));

        let inner = block.inner(area);

        let mut lines: Vec<Line> = Vec::new();

        for (idx, entry) in self.entries.iter().enumerate().skip(scroll).take(visible_height) {
            let is_selected = idx == self.selected;
            let indent = "  ".repeat(entry.depth);

            let (icon_info, name_style) = if entry.is_dir {
                let expanded = self.expanded.contains(&entry.path);
                let icon = icon_for_directory_special(&entry.name, expanded)
                    .unwrap_or_else(|| icon_for_directory(expanded));
                let style = if is_selected {
                    Style::default()
                        .fg(theme.ui.file_tree_selected_fg)
                        .bg(theme.ui.file_tree_selected_bg)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                        .fg(theme.ui.file_tree_dir)
                        .add_modifier(Modifier::BOLD)
                };
                (icon, style)
            } else {
                let ext = entry
                    .path
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("");
                let icon = icon_for_extension(ext);
                let style = if is_selected {
                    Style::default()
                        .fg(theme.ui.file_tree_selected_fg)
                        .bg(theme.ui.file_tree_selected_bg)
                } else {
                    Style::default().fg(theme.ui.file_tree_fg)
                };
                (icon, style)
            };

            let bg = if is_selected {
                theme.ui.file_tree_selected_bg
            } else {
                theme.ui.file_tree_bg
            };

            let content_width = indent.len() + 2 + entry.name.len();
            let remaining = (inner.width as usize).saturating_sub(content_width);
            let padding = if remaining > 0 {
                " ".repeat(remaining)
            } else {
                String::new()
            };

            let spans = vec![
                Span::styled(indent, Style::default().bg(bg)),
                Span::styled(
                    format!("{} ", icon_info.icon),
                    Style::default().fg(icon_info.color).bg(bg),
                ),
                Span::styled(entry.name.clone(), name_style.bg(bg)),
                Span::styled(padding, Style::default().bg(bg)),
            ];

            lines.push(Line::from(spans));
        }

        // Fill remaining space
        let remaining_lines = visible_height.saturating_sub(lines.len());
        for _ in 0..remaining_lines {
            lines.push(Line::from(Span::styled(
                " ".repeat(inner.width as usize),
                Style::default().bg(theme.ui.file_tree_bg),
            )));
        }

        frame.render_widget(block, area);
        let paragraph = Paragraph::new(lines);
        frame.render_widget(paragraph, inner);
    }
}
