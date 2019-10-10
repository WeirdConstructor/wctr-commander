use sdl2::pixels::Color;
use sdl2::event::Event;
use sdl2::event::WindowEvent;
use sdl2::keyboard::Keycode;
use sdl2::rect::Rect;
use sdl2::rect::Point;
use std::rc::Rc;
use std::cell::RefCell;
use std::time::{Instant};

mod defs;
mod fm_page;
mod path_sheet;
mod text_line;

use path_sheet::*;
use fm_page::*;
use defs::*;

struct Page {
    fm_page:    Rc<dyn FmPage>,
    cache:      Option<Table>,
}

pub enum FileManagerSide {
    Left,
    Right,
}

pub struct FileManager<'a, 'b> {
    draw_state:     DrawState<'a, 'b>,
    left:           std::vec::Vec<Page>,
    right:          std::vec::Vec<Page>,
    active_side:    FileManagerSide,
}

enum PanePos {
    LeftTab,
    RightTab,
}

impl<'a, 'b> FileManager<'a, 'b> {
    fn open_path_in(&mut self, path: &std::path::Path, pos: PanePos) {
        let ps = PathSheet::read(path).expect("No broken paths please");
        let r = Rc::new(ps);
        let mut pg = Page {
            fm_page: r,
            cache: None,
        };
        Rc::get_mut(&mut pg.fm_page).unwrap().sort_by_column(0);
        match pos {
            PanePos::LeftTab  => self.left.push(pg),
            PanePos::RightTab => self.right.push(pg),
        }
    }

    fn process_page_control(&mut self, ctrl: PageControl) {
        match self.active_side {
            FileManagerSide::Left => {
                if self.left.is_empty() { return; }
                Rc::get_mut(&mut self.left.get_mut(0).unwrap().fm_page).unwrap()
                    .do_control(ctrl);
            },
            FileManagerSide::Right => {
                if self.right.is_empty() { return; }
//                Rc::get_mut(self.right.get_mut(0).unwrap().fm_page).unwrap().do_control(ctrl);
            },
        };
    }

    fn handle_resize(&mut self) {
        if !self.left.is_empty() {
            let pg : &mut Page = self.left.get_mut(0).unwrap();
            Rc::get_mut(&mut pg.fm_page).unwrap().do_control(PageControl::Refresh);
        }

        if !self.right.is_empty() {
            let pg : &mut Page = self.right.get_mut(0).unwrap();
            Rc::get_mut(&mut pg.fm_page).unwrap().do_control(PageControl::Refresh);
        }
    }

    fn redraw(&mut self) {
        if !self.left.is_empty() {
            let pg : &mut Page = self.left.get_mut(0).unwrap();

            if pg.cache.is_none() || pg.fm_page.needs_repage() {
                pg.cache = Some(pg.fm_page.as_draw_page());
            }

            let win_size = self.draw_state.canvas.window().size();
            let half_width = win_size.0 / 2;
            self.draw_state.canvas.set_draw_color(NORM_BG_COLOR);
            self.draw_state.canvas.fill_rect(Rect::new(0, 0, win_size.0, win_size.1));
            self.draw_state.canvas.set_draw_color(DIVIDER_COLOR);
            self.draw_state.canvas.draw_line(
                Point::new(half_width as i32 + 1, 0),
                Point::new(half_width as i32 + 1, win_size.1 as i32));

            let has_focus : bool =
                0 <
                self.draw_state.canvas.window().window_flags()
                & (  (sdl2::sys::SDL_WindowFlags::SDL_WINDOW_INPUT_FOCUS as u32)
                   | (sdl2::sys::SDL_WindowFlags::SDL_WINDOW_MOUSE_FOCUS as u32));

            if pg.cache.is_some() {
                let render_feedback =
                    self.draw_state.draw_table(
                        pg,
                        2, 0,
                        half_width as i32 - 2,
                        win_size.1 as i32,
                        has_focus);
                Rc::get_mut(&mut pg.fm_page)
                    .unwrap()
                    .set_render_feedback(render_feedback);
            }
        }
    }
}

struct DrawState<'a, 'b> {
    canvas: sdl2::render::Canvas<sdl2::video::Window>,
    font: Rc<RefCell<sdl2::ttf::Font<'a, 'b>>>,
}

impl<'a, 'b> DrawState<'a, 'b> {
    fn clear(&mut self) {
        self.canvas.set_draw_color(Color::RGB(255, 255, 255));
        self.canvas.clear();
    }

    fn done(&mut self) {
        self.canvas.present();
    }

    fn calc_column_text_widths(&mut self, table: &mut Table) {
        for col in table.columns.iter_mut() {
            if let ColumnSizing::TextWidth(txt) = &col.size {
                if col.calc_size.is_none() {
                    let tsize = self.font.borrow().size_of(&txt);
                    col.calc_size = Some(tsize.unwrap_or((0, 0)).0 as i32);
                }
            } else {
                col.calc_size = Some(0);
            }
        }
    }

    fn calc_column_width(&mut self, table: &Table, table_width: i32, skip_cols: u32) -> std::vec::Vec<i32> {
        if skip_cols >= table.columns.len() as u32 {
            return Vec::new();
        }

        let cols : std::vec::Vec<&Column> = table.columns.iter().rev().skip(skip_cols as usize).rev().collect();

        let fixed_width : i32 =
            cols.iter().map(|c| c.calc_size.unwrap() + table.col_gap as i32).sum();

        let expand_rest_width = table_width - fixed_width;

        if expand_rest_width < MIN_EXPAND_WIDTH {
            return self.calc_column_width(table, table_width, skip_cols + 1);
        }

        let fract_sum : u32 = cols.iter().map(|c|
            match c.size {
                ColumnSizing::ExpandFract(f) => f as u32,
                _ => 0u32,
            }).sum();

        cols.iter().map(|column|
            match column.size {
                ColumnSizing::TextWidth(_)   => column.calc_size.unwrap() + table.col_gap as i32,
                ColumnSizing::ExpandFract(f) => ((expand_rest_width * f) / fract_sum as i32) + table.col_gap as i32,
            }).collect()
    }

    fn draw_table_row(&mut self, row: &StyleString,
                      col_idx: i32,
                      row_idx: usize,
                      has_focus: bool,
                      fm_page: &Rc<dyn FmPage>,
                      x: i32,
                      y: i32,
                      width: i32,
                      col_gap: i32,
                      row_height: i32) {

        let mut fg_color = match row.style {
            Style::Dir     => DIR_FG_COLOR,
            Style::Special => LNK_FG_COLOR,
            _              => NORM_FG_COLOR,
        };

        let mut bg_color = if row_idx % 2 == 0 {
            if col_idx % 2 == 0 { NORM_BG_COLOR } else { NORM_BG2_COLOR }
        } else {
            if col_idx % 2 == 0 { NORM_BG2_COLOR } else { NORM_BG3_COLOR }
        };

        if has_focus && fm_page.is_cursor_idx(row_idx) {
            bg_color = CURS_BG_COLOR;
            fg_color = CURS_FG_COLOR;

        } else if fm_page.is_selected(row_idx) {
            bg_color = SLCT_BG_COLOR;
            fg_color = SLCT_FG_COLOR;

        } else if fm_page.is_highlighted(row_idx) {
            bg_color = HIGH_FG_COLOR;
            fg_color = HIGH_FG_COLOR;
        }

        self.canvas.set_draw_color(bg_color);
        self.canvas.fill_rect(Rect::new(x, y, width as u32, row_height as u32));
        draw_bg_text(
            &mut self.canvas,
            &mut self.font.borrow_mut(),
            fg_color, bg_color,
            x, y, width - col_gap, row_height,
            &row.text);
    }

    fn draw_table(
        &mut self,
        pg: &mut Page,
        x_offs: i32,
        y_offs: i32,
        table_width: i32,
        table_height: i32,
        has_focus: bool) -> RenderFeedback {

        let table = pg.cache.as_mut().unwrap();

        self.calc_column_text_widths(table);
        let cols = self.calc_column_width(table, table_width, 0);

        let row_height = self.font.borrow().height() + table.row_gap as i32;

        draw_bg_text(
            &mut self.canvas, &mut self.font.borrow_mut(),
            NORM_FG_COLOR, NORM_BG_COLOR,
            x_offs, y_offs, table_width, row_height,
            &table.title);

        let y_offs = y_offs + row_height;

        let mut x = x_offs;
        for width_and_col in cols.iter().enumerate().zip(table.columns.iter()) {
            let col_idx = (width_and_col.0).0;
            let width   = (width_and_col.0).1;
            let column  = width_and_col.1;
            //d// println!("COL {}, w: {}, h: {}", col_idx, width, column.head);

            draw_bg_text(
                &mut self.canvas, &mut self.font.borrow_mut(),
                NORM_FG_COLOR, NORM_BG_COLOR,
                x, y_offs, *width - table.col_gap as i32, row_height,
                &column.head);

            self.canvas.set_draw_color(NORM_FG_COLOR);
            self.canvas.draw_line(
                Point::new(x,         y_offs + (row_height - table.row_gap as i32)),
                Point::new(x + width, y_offs + (row_height - table.row_gap as i32)));

            let mut y = y_offs + row_height;

            for (row_idx, row) in column.rows.iter()
                                    .enumerate()
                                    .skip(pg.fm_page.get_scroll_offs()) {

                if (y - y_offs) + row_height > table_height {
                    break;
                }

                self.draw_table_row(
                    row, col_idx as i32, row_idx, has_focus,
                    &pg.fm_page,
                    x, y,
                    *width, table.col_gap as i32, row_height);

                y += row_height;
            }

            x += width;
            //d// println!("X= {}", x);
        }

        let line_count = ((table_height - row_height) / row_height) as i32;
        RenderFeedback {
            // substract 1 row_height for title bar
            recent_line_count: line_count as usize,
            row_offset: pg.fm_page.get_scroll_offs(),
            start_rows: (x_offs,
                         y_offs + row_height),
            row_height,
            end_rows:   (x_offs + table_width,
                         y_offs + row_height + line_count as i32 * row_height),
        }
    }
}

fn draw_text(font: &mut sdl2::ttf::Font, color: Color, canvas: &mut sdl2::render::Canvas<sdl2::video::Window>, x: i32, y: i32, max_w: i32, txt: &str) {
    let txt_crt = canvas.texture_creator();

    let sf = font.render(txt).blended(color).map_err(|e| e.to_string()).unwrap();
    let txt = txt_crt.create_texture_from_surface(&sf).map_err(|e| e.to_string()).unwrap();
    let tq = txt.query();

    let w : i32 = if max_w < (tq.width as i32) { max_w } else { tq.width as i32 };

//    txt.set_color_mod(255, 0, 0);
    canvas.copy(
        &txt,
        Some(Rect::new(0, 0, w as u32, tq.height)),
        Some(Rect::new(x, y, w as u32, tq.height))
    ).map_err(|e| e.to_string()).unwrap();
}

fn draw_bg_text(canvas: &mut sdl2::render::Canvas<sdl2::video::Window>,
                font: &mut sdl2::ttf::Font,
                color: Color,
                bg_color: Color,
                x: i32,
                y: i32,
                max_w: i32,
                h: i32,
                txt: &str) {

    canvas.set_draw_color(bg_color);
    canvas.fill_rect(Rect::new(x, y, max_w as u32, h as u32));
    draw_text(font, color, canvas, x, y, max_w, txt);
}

pub fn main() -> Result<(), String> {
    let sdl_context = sdl2::init()?;
    let video_subsystem = sdl_context.video()?;

    let window = video_subsystem.window("rust-sdl2 demo: Video", 800, 600)
        .position_centered()
        .resizable()
//        .opengl()
        .build()
        .map_err(|e| e.to_string())?;

    let canvas = window.into_canvas().build().map_err(|e| e.to_string())?;

    let mut event_pump = sdl_context.event_pump()?;


    let ttf_ctx = sdl2::ttf::init().map_err(|e| e.to_string())?;

    let mut font = ttf_ctx.load_font("DejaVuSansMono.ttf", 14).map_err(|e| e.to_string())?;
//    font.set_style(sdl2::ttf::FontStyle::BOLD | sdl2::ttf::FontStyle::UNDERLINE);
    font.set_hinting(sdl2::ttf::Hinting::Normal);
//    font.set_outline_width(0.1);
    font.set_kerning(true);

    let mut fm = FileManager {
        draw_state: DrawState {
            canvas: canvas,
            font: Rc::new(RefCell::new(font)),
        },
        active_side: FileManagerSide::Left,
        left: Vec::new(),
        right: Vec::new(),
    };

    let pth = std::path::Path::new(".");
    fm.open_path_in(pth, PanePos::LeftTab);

    let mut last_frame = Instant::now();
    let mut is_first = true;
    'running: loop {
        let mut force_redraw = false;
        let event = event_pump.wait_event_timeout(1000);
        if let Some(event) = event {
            match event {
                Event::Quit {..} | Event::KeyDown { keycode: Some(Keycode::Escape), .. } => {
                    break 'running
                },
                Event::KeyDown { keycode: Some(Keycode::H), .. } => {
                    fm.process_page_control(PageControl::Back);
                },
                Event::KeyDown { keycode: Some(Keycode::J), .. } => {
                    fm.process_page_control(PageControl::CursorDown);
                },
                Event::KeyDown { keycode: Some(Keycode::K), .. } => {
                    fm.process_page_control(PageControl::CursorUp);
                },
                Event::KeyDown { keycode: Some(Keycode::L), .. } => {
                    fm.process_page_control(PageControl::Access);
                },
                Event::MouseButtonDown { x: x, y: y, .. } => {
                    fm.process_page_control(PageControl::Click((x, y)));
                },
                Event::TextInput { text: text, .. } => {
                    println!("TEXT: {}", text);
                },
                Event::MouseWheel { y: y, direction: dir, .. } => {
                    match dir {
                        sdl2::mouse::MouseWheelDirection::Normal => {
                            fm.process_page_control(PageControl::Scroll(-y));
                            println!("DIR NORMAL");
                        },
                        sdl2::mouse::MouseWheelDirection::Flipped => {
                            fm.process_page_control(PageControl::Scroll(y));
                            println!("DIR FLOP");
                        },
                        _ => {}
                    }
                },
                Event::Window { win_event: w, timestamp: _, window_id: _ } => {
                    match w {
                        WindowEvent::Resized(w, h) => {
                            println!("XHX {},{}", w, h);
                            fm.handle_resize();
                            force_redraw = true;
                        },
                        WindowEvent::SizeChanged(w, h) => {
                            println!("XHXSC {},{}", w, h);
                            fm.handle_resize();
                            force_redraw = true;
                        },
                        WindowEvent::FocusGained => {
                            force_redraw = true;
                        },
                        WindowEvent::FocusLost => {
                            force_redraw = true;
                        },
                        _ => {}
                    }
                },
                _ => {}
            }
        }

        let frame_time = last_frame.elapsed().as_millis();
        println!("FO {},{},{}", frame_time, is_first, force_redraw);

        if is_first || force_redraw || frame_time >= 16 {
            fm.draw_state.clear();
            fm.redraw();
            fm.draw_state.done();
            last_frame = Instant::now();
        }

        is_first = false;
    }

    Ok(())
}
