
#[derive(Debug,Clone)]
pub struct TextInputLine {
    history:        std::vec::Vec<String>,
    history_pos:    Option<u32>,
    cursor_pos:     usize,
    text:           String,
}

#[derive(Debug,Clone)]
pub enum TextInputAction {
    Insert(String),
    Replace(String),
    Clear,
    CursorRight,
    CursorLeft,
    CursorBegin,
    CursorEnd,
    HistoryUp,
    HistoryDown,
}

impl TextInputLine {
    pub fn new() -> TextInputLine {
        TextInputLine {
            history:        Vec::new(),
            history_pos:    None,
            cursor_pos:     0,
            text:           String::from(""),
        }
    }

    pub fn get_line_info(&self) -> (usize, usize, &str) {
        (self.cursor_pos, 0, &self.text)
    }

    fn add_history(&mut self, text: String) {
        self.history_pos = None;
        if !self.history.is_empty()
           && self.history[self.history.len() - 1] == text {
            return;
        }
        self.history.push(text);
    }

    pub fn handle_input(&mut self, action: TextInputAction) {
        match action {
            TextInputAction::Insert(s) => {
                let left : String = self.text.chars().take(self.cursor_pos).collect();
                let right : String = self.text.chars().skip(self.cursor_pos).collect();
                self.text = left + &s + &right;
                self.cursor_pos += s.len();
                self.history_pos = None;
            },
            TextInputAction::Replace(s) => {
                self.text = s;
                self.cursor_pos = 0;
                self.history_pos = None;
            },
            TextInputAction::Clear => {
                self.text = String::from("");
                self.cursor_pos = 0;
                self.history_pos = None;
            },
            TextInputAction::CursorLeft => {
                if self.cursor_pos > 0 {
                    self.cursor_pos -= 1;
                }
            },
            TextInputAction::CursorRight => {
                self.cursor_pos += 1;
                if self.cursor_pos > self.text.len() {
                    self.cursor_pos = self.text.len();
                }
            },
            TextInputAction::CursorBegin => {
                self.cursor_pos = 0;
            },
            TextInputAction::CursorEnd => {
                self.cursor_pos = self.text.len();
            },
            TextInputAction::HistoryUp => {
            },
            TextInputAction::HistoryDown => {
            },
        }
    }
}
