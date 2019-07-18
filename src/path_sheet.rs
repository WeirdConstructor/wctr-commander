use chrono::DateTime;
use chrono::offset::Utc;
use crate::fm_page::*;
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
    pub cursor_idx:         usize,
    pub scroll_offset:      usize,
    pub selection:          std::collections::HashSet<usize>,
    pub highlight:          std::collections::HashSet<usize>,
    pub render_feedback:    RenderFeedback,
}

const SCROLL_PADDING : usize = 5;

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
            cursor_idx:     0,
            scroll_offset:  0,
            render_feedback: RenderFeedback {
                recent_line_count: 0,
                row_offset:        0,
                start_rows:        (0, 0),
                row_height:        0,
                end_rows:          (0, 0),
            },
            selection:      std::collections::HashSet::new(),
            highlight:      std::collections::HashSet::new(),
            paths_dirty:    false,
            state_dirty:    false,
        })
    }
}

impl FmPage for PathSheet {
    fn len(&self) -> usize { self.paths.len() }
    fn get_scroll_offs(&self) -> usize { self.scroll_offset }
    fn is_cursor_idx(&self, idx: usize) -> bool { self.cursor_idx == idx }
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

    fn do_control(&mut self, ctrl: PageControl) {
        match ctrl {
            PageControl::CursorDown => {
                self.cursor_idx += 1;
            },
            PageControl::CursorUp => {
                if self.cursor_idx > 0 {
                    self.cursor_idx -= 1;
                }
            },
            PageControl::Click((x, y)) => {
                println!("CLICK {}, {} | {:?}", x, y, self.render_feedback);
                let x1 = self.render_feedback.start_rows.0;
                let x2 = self.render_feedback.end_rows.0;
                let y1 = self.render_feedback.start_rows.1;
                let y2 = self.render_feedback.end_rows.1;

                if !(x >= x1 && x <= x2 && y >= y1 && y <= y2) {
                    return;
                }

                let y = y - y1;
                let row = y / self.render_feedback.row_height;
                self.cursor_idx = self.render_feedback.row_offset + row as usize;
            },
            PageControl::Scroll(amount) => {
                println!("SCROLL {}", amount);
                let amount = amount * SCROLL_PADDING as i32;
                if amount < 0 && self.scroll_offset < (-amount) as usize {
                    self.scroll_offset = 0;

                } else if amount < 0 {
                    self.scroll_offset -= (-amount) as usize;

                } else {
                    self.scroll_offset += amount as usize;
                }

                if self.len() <= self.render_feedback.recent_line_count {
                    self.scroll_offset = 0;
                } else {
                    if self.scroll_offset > (self.len() - self.render_feedback.recent_line_count) {
                        self.scroll_offset = self.len() - self.render_feedback.recent_line_count;
                    }
                }

                return;
            },
            _ => {},
        }

        println!("CURSOR CTRL {} len:{}, offs:{} disp:{}",
                 self.cursor_idx,
                 self.len(),
                 self.scroll_offset,
                 self.render_feedback.recent_line_count);

        if self.cursor_idx >= self.len() {
            self.cursor_idx = if self.len() > 0 { self.len() - 1 } else { 0 };
        }

        let recent_linecnt = self.render_feedback.recent_line_count;

        if recent_linecnt <= 2 * SCROLL_PADDING {
            if self.cursor_idx > 0 {
                self.scroll_offset = self.cursor_idx - 1;
            } else {
                self.scroll_offset = self.cursor_idx;
            }
        } else {
            if self.cursor_idx < (self.scroll_offset + SCROLL_PADDING) {
                let diff = (self.scroll_offset + SCROLL_PADDING) - self.cursor_idx;
                if self.scroll_offset > diff {
                    self.scroll_offset -= diff;
                } else {
                    self.scroll_offset = 0;
                }

            } else if (self.cursor_idx + SCROLL_PADDING + 1) > (self.scroll_offset + recent_linecnt) {
                self.scroll_offset += (self.cursor_idx + SCROLL_PADDING + 1) - (self.scroll_offset + recent_linecnt);
            }

            if (self.scroll_offset + recent_linecnt) > self.len() {
                if self.len() < recent_linecnt {
                    self.scroll_offset = 0;
                } else {
                    self.scroll_offset = self.len() - recent_linecnt;
                }
            }
        }

        println!("END CURSOR CTRL {} len:{}, offs:{} disp:{}", self.cursor_idx, self.len(), self.scroll_offset, recent_linecnt);
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
                    size: ColumnSizing::TextWidth(String::from("MMMMMM")),
                    calc_size: None,
                    rows: self.paths.iter().map(|_p| {
                        StyleString { text: String::from("{???}"), style: Style::Default }
                    }).collect(),
                },
            ],
        }
    }
}

