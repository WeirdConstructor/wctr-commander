use crate::fm_page::RenderFeedback;
use crate::fm_page::PageControl;

const SCROLL_PADDING : usize = 5;

pub struct PageCursor {
    pub cursor_scroll_bottom: bool,
    pub cursor_idx:         usize,
    pub scroll_offset:      usize,
}

impl PageCursor {
    pub fn new() -> Self {
        PageCursor {
            cursor_scroll_bottom:   false,
            cursor_idx:             0,
            scroll_offset:          0,
        }
    }

    pub fn enable_fixed_bottom_scroll(&mut self) {
        self.cursor_scroll_bottom = true;
    }

    pub fn do_control(&mut self,
                      row_count: usize,
                      render_fb: &RenderFeedback,
                      ctrl: PageControl) {
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

                if row_count <= render_fb.recent_line_count {
                    self.scroll_offset = 0;
                } else {
                    if self.scroll_offset > (row_count - render_fb.recent_line_count) {
                        self.scroll_offset = row_count - render_fb.recent_line_count;
                    }
                }

                return;
            },
            _ => {},
        }

        println!("CURSOR CTRL {} row_count:{}, offs:{} disp:{}",
                 self.cursor_idx,
                 row_count,
                 self.scroll_offset,
                 render_fb.recent_line_count);

        if self.cursor_idx >= row_count {
            self.cursor_idx = if row_count > 0 { row_count - 1 } else { 0 };
        }

        let recent_linecnt = render_fb.recent_line_count;

        if self.cursor_scroll_bottom {
            if self.cursor_idx > recent_linecnt {
                self.scroll_offset = self.cursor_idx - recent_linecnt;
            } else {
                self.scroll_offset = self.cursor_idx;
            }
        } else if recent_linecnt <= 2 * SCROLL_PADDING {
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

            if (self.scroll_offset + recent_linecnt) > row_count {
                if row_count < recent_linecnt {
                    self.scroll_offset = 0;
                } else {
                    self.scroll_offset = row_count - recent_linecnt;
                }
            }
        }

        println!("END CURSOR CTRL {} row_count:{}, offs:{} disp:{}", self.cursor_idx, row_count, self.scroll_offset, recent_linecnt);
    }

    pub fn is_cursor_idx(&self, idx: usize) -> bool { self.cursor_idx == idx }
}
