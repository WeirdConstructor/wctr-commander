use crate::fm_page::*;
use crate::cursor::*;

pub struct LogSheet {
    pub max_messages:   usize,
    pub messages:       std::vec::Vec<String>,
    pub rows:           std::vec::Vec<String>,
    pub render_feedback: RenderFeedback,
    pub msgs_dirty:     bool,
    pub state_dirty:    bool,
    pub cursor:         PageCursor,
    pub last_width_in_m_chars: usize,
}

fn append_msg_rows(rows: &mut Vec<String>, msg: &str, chars_per_row: usize) {
    let mut row = String::new();
    let mut nothing_pushed = true;
    for c in msg.chars() {
        row.push(c);
        if row.len() >= chars_per_row {
            rows.push(row);
            nothing_pushed = false;
            row = String::new();
        }
    }
    if row.len() && nothing_pushed {
        rows.push(row);
    }
}

impl LogSheet {
    pub fn new() -> Self {
        LogSheet {
            max_messages:    1000,
            messages:        Vec::new(),
            rows:            Vec::new(),
            render_feedback: RenderFeedback::new(),
            msgs_dirty:      true,
            state_dirty:     true,
            cursor:          PageCursor::new(),
            last_width_in_m_chars: 0,
        }
    }

    pub fn append_msg(&mut self, msg: String) {
        self.messages.push(msg);
        append_msg_rows(
            &mut self.rows, &msg, self.render_feedback.width_in_m_chars);
        self.msgs_dirty = true;
        self.last_width_in_m_chars = self.render_feedback.width_in_m_chars;
    }

    pub fn rewrap(&mut self) {
        self.rows.clear();
        for m in self.messages.iter() {
            append_msg_rows(
                &mut self.rows, m, self.render_feedback.width_in_m_chars);
        }
        self.msgs_dirty = true;
        self.last_width_in_m_chars = self.render_feedback.width_in_m_chars;
    }
}

impl FmPage for LogSheet {
    fn len(&self) -> usize { self.rows.len() }
    fn get_scroll_offs(&self) -> usize           { self.cursor.scroll_offset }
    fn is_cursor_idx(&self, idx: usize) -> bool  { self.cursor.is_cursor_idx(idx) }
    fn is_selected(&self, idx: usize) -> bool    { false }
    fn is_highlighted(&self, idx: usize) -> bool { false }
    fn needs_repage(&self) -> bool               { self.msgs_dirty }
    fn needs_redraw(&self) -> bool               { self.state_dirty }
    fn sort_by_column(&mut self, col_idx: usize) { }

    fn set_render_feedback(&mut self, fb: RenderFeedback) {
        self.render_feedback = fb;
        if self.last_width_in_m_chars != fb.width_in_m_chars {
            self.rewrap();
        }
    }

    fn is_inside_screen_rect(&self, x: i32, y: i32) -> bool {
        self.render_feedback.is_inside_screen_rect(x, y)
    }

    fn do_control(&mut self, ctrl: PageControl) {
        self.cursor.do_control(self.len(), &self.render_feedback, ctrl);
    }

    fn as_draw_page(&self) -> Table {
        Table {
            title: String::from("Log"),
            row_gap: 2,
            col_gap: 0,
            columns: vec![
                Column {
                    head: String::from("msg"),
                    size: ColumnSizing::ExpandFract(1),
                    calc_size: None,
                    rows: self.rows.iter().map(|r| {
                        StyleString {
                            text: r.to_string(),
                            style: Style::Default,
                        }
                    }).collect(),
                },
            ],
        }
    }
}
