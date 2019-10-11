use chrono::DateTime;
use chrono::offset::Utc;
use crate::fm_page::*;
use crate::cursor::PageCursor;
use std::fs;

#[derive(Debug)]
pub enum FMError {
    IOError(std::io::Error),
}

impl std::convert::From<std::io::Error> for FMError {
    fn from(error: std::io::Error) -> Self {
        FMError::IOError(error)
    }
}


pub enum PathRecordType {
    File,
    Dir,
    SymLink,
}

pub struct PathRecord {
    pub path:       std::path::PathBuf,
    pub size:       u64,
    pub mtime:      std::time::SystemTime,
    pub path_type:  PathRecordType,
}

pub struct PathSheet {
    pub base:               std::path::PathBuf,
    pub paths:              std::vec::Vec<PathRecord>,
    pub paths_dirty:        bool,
    pub state_dirty:        bool,
    pub selection:          std::collections::HashSet<usize>,
    pub highlight:          std::collections::HashSet<usize>,
    pub render_feedback:    RenderFeedback,
    pub cursor:             PageCursor,
}

impl PathSheet {
    pub fn read(path: &std::path::Path) -> Result<PathSheet, FMError> {
        let mut sheet_paths = Vec::new();

        for e in fs::read_dir(path)? {
            let entry = e?;
            let path  = entry.path();
            let md    = path.symlink_metadata()?;
            let ft    = md.file_type();

            let pr = PathRecord {
                path,
                size:  md.len(),
                mtime: md.modified()?,
                path_type: if ft.is_symlink() {
                    PathRecordType::SymLink
                } else if ft.is_dir() {
                    PathRecordType::Dir
                } else {
                    PathRecordType::File
                },
            };

            sheet_paths.push(pr);
        }

        Ok(PathSheet {
            base:           path.to_path_buf(),
            paths:          sheet_paths,
            render_feedback: RenderFeedback::new(),
            cursor:         PageCursor::new(),
            selection:      std::collections::HashSet::new(),
            highlight:      std::collections::HashSet::new(),
            paths_dirty:    false,
            state_dirty:    false,
        })
    }
}

impl FmPage for PathSheet {
    fn len(&self) -> usize { self.paths.len() }
    fn get_scroll_offs(&self) -> usize { self.cursor.scroll_offset }
    fn is_cursor_idx(&self, idx: usize) -> bool { self.cursor.is_cursor_idx(idx) }
    fn is_selected(&self, idx: usize) -> bool { self.selection.get(&idx).is_some() }
    fn is_highlighted(&self, idx: usize) -> bool { self.highlight.get(&idx).is_some() }
    fn needs_repage(&self) -> bool { self.paths_dirty }
    fn needs_redraw(&self) -> bool { self.state_dirty }

    fn sort_by_column(&mut self, col_idx: usize) {
        if col_idx == 0 {
            self.paths.sort_by(|a, b| {
                let s1 = String::from(
                    a.path.file_name()
                    .unwrap_or(std::ffi::OsStr::new(""))
                    .to_string_lossy()).to_lowercase();
                let s2 = String::from(
                    b.path.file_name()
                    .unwrap_or(std::ffi::OsStr::new(""))
                    .to_string_lossy()).to_lowercase();

                if let PathRecordType::Dir = a.path_type {
                    if let PathRecordType::Dir = b.path_type {
                        s1.partial_cmp(&s2).unwrap()
                    } else {
                        std::cmp::Ordering::Less
                    }
                } else {
                    if let PathRecordType::Dir = b.path_type {
                        std::cmp::Ordering::Greater
                    } else {
                        s1.partial_cmp(&s2).unwrap()
                    }
                }
            });
        } else if col_idx == 1 {
            self.paths.sort_by(|a, b| a.mtime.partial_cmp(&b.mtime).unwrap());
        } else if col_idx == 2 {
            self.paths.sort_by(|a, b| a.size.partial_cmp(&b.size).unwrap());
        }

        self.paths_dirty = true;
    }

    fn set_render_feedback(&mut self, fb: RenderFeedback) {
        self.render_feedback = fb;
    }

    fn is_inside_screen_rect(&self, x: i32, y: i32) -> bool {
        self.render_feedback.is_inside_screen_rect(x, y)
    }

    fn do_control(&mut self, ctrl: PageControl) {
        self.cursor.do_control(self.len(), &self.render_feedback, ctrl);
    }

    fn as_draw_page(&self) -> Table {
        Table {
            title: String::from(self.base.to_string_lossy()),
            row_gap: 2,
            col_gap: 4,
            columns: vec![
                Column {
                    head: String::from("name"),
                    size: ColumnSizing::ExpandFract(1),
                    calc_size: None,
                    rows: self.paths.iter().map(|p| {
                        let mut path_postfix = String::from("");
                        if let PathRecordType::Dir = p.path_type {
                            path_postfix = std::path::MAIN_SEPARATOR.to_string();
                        };

                        StyleString {
                            text: String::from(p.path.file_name()
                                                .unwrap_or(std::ffi::OsStr::new(""))
                                                .to_string_lossy()) + &path_postfix,
                            style: match p.path_type {
                                PathRecordType::File    => Style::File,
                                PathRecordType::Dir     => Style::Dir,
                                PathRecordType::SymLink => Style::Special,
                            }
                        }
                    }).collect(),
                },
                Column {
                    head: String::from("time"),
                    size: ColumnSizing::TextWidth(String::from("MMMM-MM-MM MM:MM:MM")),
                    calc_size: None,
                    rows: self.paths.iter().map(|p| {
                        let dt : DateTime<Utc> = p.mtime.into();
                        StyleString { text: format!("{}", dt.format("%Y-%m-%d %H:%M:%S")), style: Style::Default }
                    }).collect(),
                },
                Column {
                    head: String::from("size"),
                    size: ColumnSizing::TextWidth(String::from("MMMMMMMM")),
                    calc_size: None,
                    rows: self.paths.iter().map(|p| {
                        let text =
                            if p.size >= 1024_u64.pow(4) {
                                let rnd = 1024_u64.pow(4) - 1;
                                format!("{:-4}  TB", (p.size + rnd) / 1024_u64.pow(4))
                            } else if p.size >= 1024_u64.pow(3) {
                                let rnd = 1024_u64.pow(3) - 1;
                                format!("{:-4}  GB", (p.size + rnd) / 1024_u64.pow(3))
                            } else if p.size >= 1024_u64.pow(2) {
                                let rnd = 1024_u64.pow(2) - 1;
                                format!("{:-4}  MB", (p.size + rnd) / 1024_u64.pow(2))
                            } else if p.size >= 1024_u64.pow(1) {
                                let rnd = 1024_u64.pow(1) - 1;
                                format!("{:-4}  kB", (p.size + rnd) / 1024_u64.pow(1))
                            } else {
                                format!("{:-4}  B", p.size)
                            };
                        StyleString { text, style: Style::Default }
                    }).collect(),
                },
            ],
        }
    }
}

