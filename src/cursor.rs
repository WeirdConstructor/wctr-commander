use crate::fm_page::RenderFeedback;

pub struct PageCursor {
    pub cursor_idx:         usize,
    pub scroll_offset:      usize,
}

impl PageCursor {
    pub fn new() -> Self {
        PageCursor {
            cursor_idx:     0,
            scroll_offset:  0,
        }
    }

    pub fn do_control(&mut self, render_fb: &RenderFeedback, ctrl: PageControl) {
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
                let x1 = render_fb.start_rows.0;
                let x2 = render_fb.end_rows.0;
                let y1 = render_fb.start_rows.1;
                let y2 = render_fb.end_rows.1;

                if !(x >= x1 && x <= x2 && y >= y1 && y <= y2) {
                    return;
                }

                let y = y - y1;
                let row = y / render_fb.row_height;
                self.cursor_idx = render_fb.row_offset + row as usize;
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

                if self.len() <= render_fb.recent_line_count {
                    self.scroll_offset = 0;
                } else {
                    if self.scroll_offset > (self.len() - render_fb.recent_line_count) {
                        self.scroll_offset = self.len() - render_fb.recent_line_count;
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
                 render_fb.recent_line_count);

        if self.cursor_idx >= self.len() {
            self.cursor_idx = if self.len() > 0 { self.len() - 1 } else { 0 };
        }

        let recent_linecnt = render_fb.recent_line_count;

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

    pub fn is_cursor_idx(&self, idx: usize) -> bool { self.cursor_idx == idx }
}
