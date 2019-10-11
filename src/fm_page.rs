use std::rc::Rc;
use std::cell::RefCell;

#[derive(Debug)]
pub enum Style {
    Default,
    Dir,
    File,
    Special,
}

#[derive(Debug)]
pub struct StyleString {
    pub text: String,
    pub style: Style,
}

#[derive(Debug)]
pub enum ColumnSizing {
    TextWidth(String),
    ExpandFract(i32),
}

#[derive(Debug)]
pub struct Column {
    pub head:          String,
    pub rows:          std::vec::Vec<StyleString>,
    pub size:          ColumnSizing,
    pub calc_size:     Option<i32>,
}

#[derive(Debug)]
pub struct Table {
    pub title:      String,
    pub columns:    std::vec::Vec<Column>,
    pub row_gap:    u32,
    pub col_gap:    u32,
}

pub type TableRef = Rc<RefCell<Table>>;

impl Table {
    pub fn new() -> Self {
        Table {
            title: String::from(""),
            columns: Vec::new(),
            row_gap: 0,
            col_gap: 0,
        }
    }

    pub fn new_ref() -> Rc<RefCell<Self>> {
        Rc::new(RefCell::new(Self::new()))
    }
}

#[derive(Debug, Clone, Copy)]
pub enum PageControl {
    Refresh,
    Back,
    Access,
    CursorDown,
    CursorUp,
    Click((i32, i32)),
    Scroll(i32),
}

#[derive(Debug)]
pub struct RenderFeedback {
    pub recent_line_count: usize,
    pub row_offset:        usize,
    pub start_rows:        (i32, i32),
    pub row_height:        i32,
    pub end_rows:          (i32, i32),
    pub screen_pos:        (i32, i32),
    pub screen_rect:       (u32, u32),
    pub width_in_m_chars:  usize,
}

impl RenderFeedback {
    pub fn new() -> Self {
        RenderFeedback {
            screen_pos:        (0, 0),
            screen_rect:       (0, 0),
            recent_line_count: 0,
            row_offset:        0,
            start_rows:        (0, 0),
            row_height:        0,
            end_rows:          (0, 0),
            width_in_m_chars:  0,
        }
    }

    pub fn is_inside_screen_rect(&self, x: i32, y: i32) -> bool {
        let x1 = self.screen_pos.0;
        let y1 = self.screen_pos.1;
        let x2 = self.screen_rect.0 as i32 + x1;
        let y2 = self.screen_rect.1 as i32 + y1;
        return x >= x1 && y >= y1 && x < x2 && y < y2;
    }
}

pub trait FmPage {
    fn len(&self) -> usize;
    fn as_drawable_table(&mut self) -> Rc<RefCell<Table>>;
    fn get_scroll_offs(&self) -> usize;
    fn do_control(&mut self, ctrl: PageControl);
    fn is_inside_screen_rect(&self, x: i32, y: i32) -> bool;
    fn is_cursor_idx(&self, idx: usize) -> bool;
    fn is_selected(&self, idx: usize) -> bool;
    fn is_highlighted(&self, idx: usize) -> bool;
    fn needs_repage(&self) -> bool;
    fn needs_redraw(&self) -> bool;

    fn sort_by_column(&mut self, col_idx: usize);

    fn set_render_feedback(&mut self, fb: RenderFeedback);
}
