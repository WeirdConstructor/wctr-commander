
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

#[derive(Debug)]
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
}

pub trait FmPage {
    fn len(&self) -> usize;
    fn as_draw_page(&self) -> Table;
    fn get_scroll_offs(&self) -> usize;
    fn do_control(&mut self, ctrl: PageControl);
    fn is_cursor_idx(&self, idx: usize) -> bool;
    fn is_selected(&self, idx: usize) -> bool;
    fn is_highlighted(&self, idx: usize) -> bool;
    fn needs_repage(&self) -> bool;
    fn needs_redraw(&self) -> bool;

    fn sort_by_column(&mut self, col_idx: usize);

    fn set_render_feedback(&mut self, fb: RenderFeedback);
}
