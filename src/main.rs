use sdl2::pixels::Color;
use sdl2::event::Event;
use sdl2::event::WindowEvent;
use sdl2::keyboard::Keycode;
use sdl2::rect::Rect;
use sdl2::rect::Point;
use std::rc::Rc;
use std::cell::RefCell;
use std::time::{Instant};

use wlambda::*;

mod defs;
mod cursor;
mod fm_page;
mod path_sheet;
mod text_line;
mod log_sheet;

use log_sheet::*;
use path_sheet::*;
use fm_page::*;
use defs::*;
use text_line::*;

use wlambda;
use wlambda::{VVal, GlobalEnv, EvalContext};

struct Page {
    fm_page:    Rc<dyn FmPage>,
    cache:      Option<Table>,
}

fn draw_fm_page(fm_page: &mut dyn FmPage, gp: &mut GUIPainter, x: i32, y: i32, w: u32, h: u32, is_active: bool) {
    gp.canvas.set_draw_color(NORM_BG_COLOR);
    gp.canvas.fill_rect(Rect::new(x, y, w, h))
        .expect("filling rectangle");

    let has_focus : bool =
        0 <
            gp.canvas.window().window_flags()
            & (  (sdl2::sys::SDL_WindowFlags::SDL_WINDOW_INPUT_FOCUS as u32)
               | (sdl2::sys::SDL_WindowFlags::SDL_WINDOW_MOUSE_FOCUS as u32));

    let render_feedback =
        gp.draw_table(
            fm_page, x + 2, y, w as i32 - 2, h as i32, has_focus, is_active);
    fm_page.set_render_feedback(render_feedback);
}

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum FileManagerSide {
    Left,
    Right,
}

pub struct FileManager {
    left:               std::vec::Vec<PathSheet>,
    right:              std::vec::Vec<PathSheet>,
    log:                LogSheet,
    active_side:        FileManagerSide,
    input_line:         TextInputLine,
    prompt:             String,
    show_input_line:    bool,
}

enum FileManagerAction {
    SetPrompt(String, bool),
    TextInput(TextInputAction),
}

enum PanePos {
    LeftTab,
    RightTab,
}

//fn wlambda_bind(genv: &mut GlobalEnv, fm: Rc<RefCell<FileManager>>) {
//}

struct WLCallbacks {
    ctx:        Rc<RefCell<EvalContext>>,
    cb_input:   VVal,
    fm_api:     VVal,
}

impl WLCallbacks {
    pub fn on_input(&mut self, keybind: String) {
        self.ctx.borrow_mut()
            .call(&self.cb_input, &[self.fm_api.clone(), VVal::new_str_mv(keybind)])
            .expect("No error in 'on_input' callback");
    }

    pub fn set_fm_api(&mut self, fm: VVal) {
        self.fm_api = fm;
    }
}

fn init_wlambda() -> WLCallbacks {
    let genv = GlobalEnv::new_default();

//    wlambda_bind(&mut genv.borrow_mut(), fm.clone());

//    let lfmr =
//        std::rc::Rc::new(std::cell::RefCell::new(
//            wlambda::compiler::LocalFileModuleResolver::new()));
//    genv.borrow_mut().set_resolver(lfmr);
    let mut wl_eval_ctx = EvalContext::new_default();

    match wl_eval_ctx.eval_file("main.wl") {
        Ok(v) => {
            if v.is_err() {
                panic!(format!("'main.wl' SCRIPT ERROR: {}", v.s()));
            }

            let cb_input =
                    wl_eval_ctx
                    .get_global_var("on_input")
                    .expect("'on_input' global callback in main.wl");
            WLCallbacks {
                ctx:        Rc::new(RefCell::new(wl_eval_ctx)),
                fm_api:     VVal::None,
                cb_input,
            }
        },
        Err(e) => { panic!(format!("'main.wl' SCRIPT ERROR: {}", e)); }
    }
}

impl FileManager {
    fn open_path_in(&mut self, path: &std::path::Path, pos: PanePos) {
        let mut ps = PathSheet::read(path).expect("No broken paths please");
        ps.sort_by_column(0);
        match pos {
            PanePos::LeftTab  => self.left.push(ps),
            PanePos::RightTab => self.right.push(ps),
        }
    }

    fn toggle_active_side(&mut self) {
        self.active_side =
            match self.active_side {
                FileManagerSide::Left => FileManagerSide::Right,
                FileManagerSide::Right => FileManagerSide::Left,
            };
    }

    fn action(&mut self, fmact: FileManagerAction) {
        match fmact {
            FileManagerAction::SetPrompt(prompt, show) => {
                self.prompt          = prompt;
                self.show_input_line = show;
            },
            FileManagerAction::TextInput(txtact) => {
                self.input_line.handle_input(txtact);
            },
        }
    }

    fn process_page_control(&mut self, ctrl: PageControl, mouse: Option<(i32, i32)>) {
        if let Some((x, y)) = mouse {
            if !self.left.is_empty() {
                let fm_page : &mut dyn FmPage = self.left.get_mut(0).unwrap();
                if fm_page.is_inside_screen_rect(x, y) {
                    fm_page.do_control(ctrl);
                }
            }
            if !self.right.is_empty() {
                let fm_page : &mut dyn FmPage = self.right.get_mut(0).unwrap();
                if fm_page.is_inside_screen_rect(x, y) {
                    fm_page.do_control(ctrl);
                }
            }
            let fm_page : &mut dyn FmPage = &mut self.log;
            if fm_page.is_inside_screen_rect(x, y) {
                fm_page.do_control(ctrl);
            }

        } else {
            match self.active_side {
                FileManagerSide::Left => {
                    if self.left.is_empty() { return; }
                    self.left.get_mut(0).unwrap().do_control(ctrl);
                },
                FileManagerSide::Right => {
                    if self.right.is_empty() { return; }
                    self.right.get_mut(0).unwrap().do_control(ctrl);
                },
            };
        }
    }

    fn handle_resize(&mut self) {
        if !self.left.is_empty() {
            let fm_page : &mut dyn FmPage = self.left.get_mut(0).unwrap();
            fm_page.do_control(PageControl::Refresh);
        }

        if !self.right.is_empty() {
            let fm_page : &mut dyn FmPage = self.right.get_mut(0).unwrap();
            fm_page.do_control(PageControl::Refresh);
        }

        self.log.do_control(PageControl::Refresh);
    }

    fn redraw(&mut self, gui_painter: &mut GUIPainter) {
        let win_size = gui_painter.canvas.window().size();
        let half_width = win_size.0 / 2;

        let input_height = 20;
        let log_height = win_size.1 / 4;
        let tab_height = win_size.1 - log_height - input_height;
        let log_offs_y = tab_height as i32;

        if !self.left.is_empty() {
            let fm_page : &mut dyn FmPage = self.left.get_mut(0).unwrap();
            draw_fm_page(fm_page, gui_painter,
                0, 0, half_width, tab_height,
                self.active_side == FileManagerSide::Left);
        }

        gui_painter.canvas.set_draw_color(DIVIDER_COLOR);
        gui_painter.canvas.draw_line(
            Point::new(half_width as i32, 0),
            Point::new(half_width as i32, win_size.1 as i32))
            .expect("drawing a line");

        if !self.right.is_empty() {
            let fm_page : &mut dyn FmPage = self.right.get_mut(0).unwrap();
            draw_fm_page(fm_page, gui_painter,
                half_width as i32, 0, half_width, tab_height,
                self.active_side == FileManagerSide::Right);
        }

        let fm_page : &mut dyn FmPage = &mut self.log;
        draw_fm_page(fm_page, gui_painter,
            0, log_offs_y, win_size.0, log_height,
            true);

        let input_line_xoffs = {
            let (w, _h) =
                draw_bg_text(
                    &mut gui_painter.canvas,
                    &mut gui_painter.font.borrow_mut(),
                    NORM_FG_COLOR,
                    NORM_BG_COLOR,
                    0, log_offs_y + (log_height as i32),
                    win_size.0 as i32, 20, &self.prompt);
            w
        };

        if self.show_input_line {
            let (cursor_pos, scroll_offs, line_txt) =
                self.input_line.get_line_info();

            draw_bg_text_cursor(
                &mut gui_painter.canvas,
                &mut gui_painter.font.borrow_mut(),
                NORM_FG_COLOR,
                NORM_BG_COLOR,
                input_line_xoffs as i32,
                log_offs_y + (log_height as i32),
                win_size.0 as i32 - input_line_xoffs as i32,
                20, line_txt, cursor_pos);
        }
    }
}

struct GUIPainter<'a, 'b> {
    canvas: sdl2::render::Canvas<sdl2::video::Window>,
    font: Rc<RefCell<sdl2::ttf::Font<'a, 'b>>>,
}

impl<'a, 'b> GUIPainter<'a, 'b> {
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

    fn calc_table_width_chars(&self, table_width: i32) -> usize {
        let tsize = self.font.borrow().size_of("m");
        let mut char_width_in_px : usize = tsize.unwrap_or((0, 0)).0 as usize;
        if char_width_in_px == 0 {
            char_width_in_px = 5;
        }
        table_width as usize / char_width_in_px
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
                      is_active: bool,
                      fm_page: &mut dyn FmPage,
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

        let specially_marked_row =
            if has_focus && fm_page.is_cursor_idx(row_idx) {
                bg_color = CURS_BG_COLOR;
                fg_color = CURS_FG_COLOR;
                true

            } else if fm_page.is_selected(row_idx) {
                bg_color = SLCT_BG_COLOR;
                fg_color = SLCT_FG_COLOR;
                true

            } else if fm_page.is_highlighted(row_idx) {
                bg_color = HIGH_FG_COLOR;
                fg_color = HIGH_FG_COLOR;
                true
            } else {
                false
            };

        if specially_marked_row && !is_active {
            bg_color.r = (bg_color.r as f32 * 0.6) as u8;
            bg_color.g = (bg_color.g as f32 * 0.6) as u8;
            bg_color.b = (bg_color.b as f32 * 0.6) as u8;
        }

        self.canvas.set_draw_color(bg_color);
        self.canvas.fill_rect(Rect::new(x, y, width as u32, row_height as u32))
            .expect("filling rectangle");
        draw_bg_text(
            &mut self.canvas,
            &mut self.font.borrow_mut(),
            fg_color, bg_color,
            x, y, width - col_gap, row_height,
            &row.text);
    }

    fn draw_table(
        &mut self,
        fm_page: &mut dyn FmPage,
        x_offs: i32,
        y_offs: i32,
        table_width: i32,
        table_height: i32,
        has_focus: bool,
        is_active: bool) -> RenderFeedback {

        let table = fm_page.as_drawable_table();
        let mut table_ref = table.borrow_mut();

        self.calc_column_text_widths(&mut table_ref);
        let cols = self.calc_column_width(&mut table_ref, table_width, 0);
        let width_in_m_chars = self.calc_table_width_chars(table_width);

        let row_height = self.font.borrow().height() + table_ref.row_gap as i32;

        draw_bg_text(
            &mut self.canvas, &mut self.font.borrow_mut(),
            NORM_FG_COLOR, NORM_BG_COLOR,
            x_offs, y_offs, table_width, row_height,
            &table_ref.title);

        let y_offs = y_offs + row_height;
        let row_area_height = table_height - 2 * row_height;
        let row_count = (row_area_height / row_height) as usize;

        let mut x = x_offs;
        for width_and_col in cols.iter().enumerate().zip(table_ref.columns.iter()) {
            let col_idx = (width_and_col.0).0;
            let width   = (width_and_col.0).1;
            let column  = width_and_col.1;
            //d// println!("COL {}, w: {}, h: {}", col_idx, width, column.head);

            draw_bg_text(
                &mut self.canvas, &mut self.font.borrow_mut(),
                NORM_FG_COLOR, NORM_BG_COLOR,
                x, y_offs, *width - table_ref.col_gap as i32, row_height,
                &column.head);

            self.canvas.set_draw_color(NORM_FG_COLOR);
            self.canvas.draw_line(
                Point::new(x,         y_offs + (row_height - table_ref.row_gap as i32)),
                Point::new(x + width, y_offs + (row_height - table_ref.row_gap as i32)))
                .expect("drawing a line");

            let mut y = y_offs + row_height;

            for (row_idx, row) in column.rows.iter()
                                    .enumerate()
                                    .skip(fm_page.get_scroll_offs())
                                    .take(row_count) {
                self.draw_table_row(
                    row, col_idx as i32, row_idx, has_focus, is_active,
                    fm_page,
                    x, y,
                    *width, table_ref.col_gap as i32, row_height);

                y += row_height;
            }

            x += width;
            //d// println!("X= {}", x);
        }

        RenderFeedback {
            // substract 1 row_height for title bar
            screen_pos:  (x_offs, y_offs),
            screen_rect: (table_width as u32, table_height as u32),
            recent_line_count: row_count as usize,
            row_offset: fm_page.get_scroll_offs(),
            start_rows: (x_offs,
                         y_offs + row_height),
            row_height,
            end_rows:   (x_offs + table_width,
                         y_offs + row_height + row_count as i32 * row_height),
            width_in_m_chars,
        }
    }
}

fn with_text2texture<F>(font: &mut sdl2::ttf::Font,
                canvas: &mut sdl2::render::Canvas<sdl2::video::Window>,
                color: Color, txt: &str, mut f: F) -> (u32, u32)
    where F: FnMut(&mut sdl2::render::Canvas<sdl2::video::Window>, &sdl2::render::Texture) -> (u32, u32) {

    if txt.is_empty() {
        return (0, 0);
    }

    let txt_crt = canvas.texture_creator();
    let sf      = font.render(txt).blended(color).map_err(|e| e.to_string()).unwrap();
    let txt     = txt_crt.create_texture_from_surface(&sf).map_err(|e| e.to_string()).unwrap();
    let tq      = txt.query();

    f(canvas, &txt)
}

fn draw_text(font: &mut sdl2::ttf::Font, color: Color,
             canvas: &mut sdl2::render::Canvas<sdl2::video::Window>,
             x: i32, y: i32,
             max_w: i32, txt: &str) -> (u32, u32) {

    with_text2texture(font, canvas, color, txt, |canvas, t| {
        let tq = t.query();
        let w : i32 = if max_w < (tq.width as i32) { max_w } else { tq.width as i32 };

    //    t.set_color_mod(255, 0, 0);
        canvas.copy(
            &t,
            Some(Rect::new(0, 0, w as u32, tq.height)),
            Some(Rect::new(x, y, w as u32, tq.height))
        ).map_err(|e| e.to_string()).unwrap();

        (w as u32, tq.height)
    })
}

fn draw_bg_text(canvas: &mut sdl2::render::Canvas<sdl2::video::Window>,
                font: &mut sdl2::ttf::Font,
                color: Color,
                bg_color: Color,
                x: i32,
                y: i32,
                max_w: i32,
                h: i32,
                txt: &str) -> (u32, u32) {

    canvas.set_draw_color(bg_color);
    canvas.fill_rect(Rect::new(x, y, max_w as u32, h as u32))
        .expect("filling rectangle");
    draw_text(font, color, canvas, x, y, max_w, txt)
}

fn draw_bg_text_cursor(
    canvas: &mut sdl2::render::Canvas<sdl2::video::Window>,
    font: &mut sdl2::ttf::Font,
    color: Color,
    bg_color: Color,
    mut x: i32,
    y: i32,
    max_w: i32,
    h: i32,
    txt: &str,
    cursor_idx: usize) -> (u32, u32) {

    let begin  : String = txt.chars().take(cursor_idx).collect();
    let cursor : String = txt.chars().skip(cursor_idx).take(1).collect();
    let end    : String = txt.chars().skip(cursor_idx + 1).collect();

    let mut xs = x;

    canvas.set_draw_color(bg_color);
    canvas.fill_rect(Rect::new(x, y, max_w as u32, h as u32))
        .expect("filling rectangle");

    let (w1, h1) = with_text2texture(font, canvas, color, &begin, |canvas, t| {
        let tq = t.query();
        let w : i32 = if max_w < (tq.width as i32) { max_w } else { tq.width as i32 };

    //    t.set_color_mod(255, 0, 0);
        canvas.copy(
            &t,
            Some(Rect::new(0, 0, w as u32, tq.height)),
            Some(Rect::new(xs, y, w as u32, tq.height))
        ).map_err(|e| e.to_string()).unwrap();

        xs += w;
        (w as u32, tq.height)
    });

    let (w2, h2) = with_text2texture(font, canvas, CURS_FG_COLOR, &cursor, |canvas, t| {
        let tq = t.query();
        let w : i32 = if max_w < (tq.width as i32) { max_w } else { tq.width as i32 };

        canvas.set_draw_color(CURS_BG_COLOR);
        canvas.fill_rect(Rect::new(xs, y, w as u32, h as u32))
            .expect("filling rectangle");
    //    t.set_color_mod(255, 0, 0);
        canvas.copy(
            &t,
            Some(Rect::new(0, 0, w as u32, tq.height)),
            Some(Rect::new(xs, y, w as u32, tq.height))
        ).map_err(|e| e.to_string()).unwrap();

        xs += w;
        (w as u32, tq.height)
    });

    let (w3, h3) = with_text2texture(font, canvas, color, &end, |canvas, t| {
        let tq = t.query();
        let w : i32 = if max_w < (tq.width as i32) { max_w } else { tq.width as i32 };

    //    t.set_color_mod(255, 0, 0);
        canvas.copy(
            &t,
            Some(Rect::new(0, 0, w as u32, tq.height)),
            Some(Rect::new(xs, y, w as u32, tq.height))
        ).map_err(|e| e.to_string()).unwrap();
        (w as u32, tq.height)
    });

    let mut max_h = h1;
    if max_h < h2 { max_h = h2; };
    if max_h < h3 { max_h = h3; };

    (w1 + w2 + w3, max_h)
}

fn sdl2keydown2str(event: &sdl2::event::Event) -> String {
    match event {
        Event::KeyDown { keycode, keymod, .. } => {
            let mut modstr = String::new();
            if keymod.contains(sdl2::keyboard::Mod::LSHIFTMOD) {
                modstr += "SHIFT+";
            } else if keymod.contains(sdl2::keyboard::Mod::RSHIFTMOD) {
                modstr += "SHIFT+";
            }

            if keymod.contains(sdl2::keyboard::Mod::LCTRLMOD) {
                modstr += "CTRL+";
            } else if keymod.contains(sdl2::keyboard::Mod::RCTRLMOD) {
                modstr += "CTRL+";
            }

            if keymod.contains(sdl2::keyboard::Mod::LALTMOD) {
                modstr += "ALT+";
            } else if keymod.contains(sdl2::keyboard::Mod::RALTMOD) {
                modstr += "ALT+";
            }

            if keymod.contains(sdl2::keyboard::Mod::LGUIMOD) {
                modstr += "GUI+";
            } else if keymod.contains(sdl2::keyboard::Mod::RGUIMOD) {
                modstr += "GUI+";
            }

            if keymod.contains(sdl2::keyboard::Mod::CAPSMOD) {
                modstr += "CAPS+";
            }

            if keymod.contains(sdl2::keyboard::Mod::MODEMOD) {
                modstr += "MODE+";
            }

            if let Some(keycode) = keycode {
                modstr += &keycode.to_string();
            } else {
                return String::new();
            }

            modstr
        },
        _ => String::new(),
    }
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

    let mut gui_painter = GUIPainter {
        canvas: canvas,
        font: Rc::new(RefCell::new(font)),
    };

    let mut wlcbs = init_wlambda();

    let fm = FileManager {
        active_side:        FileManagerSide::Left,
        left:               Vec::new(),
        right:              Vec::new(),
        log:                LogSheet::new(),
        input_line:         TextInputLine::new(),
        prompt:             String::from("[NORMAL]"),
        show_input_line:    false,
    };

    let fm = Rc::new(RefCell::new(fm));

    let fm_actions : Rc<RefCell<Vec<FileManagerAction>>> =
        Rc::new(RefCell::new(vec![]));

    let fm_api = VVal::map();

    wlcbs.set_fm_api(fm_api.clone());

    set_vval_method!(fm_api, fm_actions, set_prompt, Some(2), Some(2), env, _argc, {
        fm_actions.borrow_mut().push(
            FileManagerAction::SetPrompt(
                env.arg(0).s(),
                env.arg(1).b()));
        Ok(VVal::None)
    });

    fm.borrow_mut().log.append_msg(String::from("FOo bar foiwe jfowi fewoi fewoif jewof weof iewjo jfewo iwejf oiwejfo iwejf owiejf oweifj weoi fjweoi w 1"));
    fm.borrow_mut().log.append_msg(String::from("FOo bar foiwe jfowi fewoi fewoif jewof weof iewjo jfewo iwejf oiwejfo iwejf owiejf oweifj weoi fjweoi w 2"));
    fm.borrow_mut().log.append_msg(String::from("FOo bar foiwe jfowi fewoi fewoif jewof weof iewjo jfewo iwejf oiwejfo iwejf owiejf oweifj weoi fjweoi w 3"));
    fm.borrow_mut().log.append_msg(String::from("FOo bar foiwe jfowi fewoi fewoif jewof weof iewjo jfewo iwejf oiwejfo iwejf owiejf oweifj weoi fjweoi w 4"));
    fm.borrow_mut().log.append_msg(String::from("FOo bar foiwe jfowi fewoi fewoif jewof weof iewjo jfewo iwejf oiwejfo iwejf owiejf oweifj weoi fjweoi w 5"));
    fm.borrow_mut().log.append_msg(String::from("FOo bar foiwe jfowi fewoi fewoif jewof weof iewjo jfewo iwejf oiwejfo iwejf owiejf oweifj weoi fjweoi w 6"));
    fm.borrow_mut().log.append_msg(String::from("FOo bar foiwe jfowi fewoi fewoif jewof weof iewjo jfewo iwejf oiwejfo iwejf owiejf oweifj weoi fjweoi w 7"));
    fm.borrow_mut().log.append_msg(String::from("FOo bar foiwe jfowi fewoi fewoif jewof weof iewjo jfewo iwejf oiwejfo iwejf owiejf oweifj weoi fjweoi w 8"));
    fm.borrow_mut().log.append_msg(String::from("FOo bar foiwe jfowi fewoi fewoif jewof weof iewjo jfewo iwejf oiwejfo iwejf owiejf oweifj weoi fjweoi w 9"));
    fm.borrow_mut().log.append_msg(String::from("FOo bar foiwe jfowi fewoi fewoif jewof weof iewjo jfewo iwejf oiwejfo iwejf owiejf oweifj weoi fjweoi w 10"));
    fm.borrow_mut().log.append_msg(String::from("FOo bar foiwe jfowi fewoi fewoif jewof weof iewjo jfewo iwejf oiwejfo iwejf owiejf oweifj weoi fjweoi w 11"));
    fm.borrow_mut().log.append_msg(String::from("FOo bar foiwe jfowi fewoi fewoif jewof weof iewjo jfewo iwejf oiwejfo iwejf owiejf oweifj weoi fjweoi w 12"));
    fm.borrow_mut().log.append_msg(String::from("FOo bar foiwe jfowi fewoi fewoif jewof weof iewjo jfewo iwejf oiwejfo iwejf owiejf oweifj weoi fjweoi w 13"));
    fm.borrow_mut().log.append_msg(String::from("FOo bar foiwe jfowi fewoi fewoif jewof weof iewjo jfewo iwejf oiwejfo iwejf owiejf oweifj weoi fjweoi w 14"));
    fm.borrow_mut().log.append_msg(String::from("FOo bar foiwe jfowi fewoi fewoif jewof weof iewjo jfewo iwejf oiwejfo iwejf owiejf oweifj weoi fjweoi w 15"));
    fm.borrow_mut().log.append_msg(String::from("FOo bar foiwe jfowi fewoi fewoif jewof weof iewjo jfewo iwejf oiwejfo iwejf owiejf oweifj weoi fjweoi w 16"));
    fm.borrow_mut().log.append_msg(String::from("FOo bar foiwe jfowi fewoi fewoif jewof weof iewjo jfewo iwejf oiwejfo iwejf owiejf oweifj weoi fjweoi w 17"));
    fm.borrow_mut().log.append_msg(String::from("FOo bar foiwe jfowi fewoi fewoif jewof weof iewjo jfewo iwejf oiwejfo iwejf owiejf oweifj weoi fjweoi w 18"));
    fm.borrow_mut().log.append_msg(String::from("FOo bar foiwe jfowi fewoi fewoif jewof weof iewjo jfewo iwejf oiwejfo iwejf owiejf oweifj weoi fjweoi w 19"));

    let pth = std::path::Path::new(".");
    fm.borrow_mut().open_path_in(pth, PanePos::LeftTab);
    let pth = std::path::Path::new("..");
    fm.borrow_mut().open_path_in(pth, PanePos::RightTab);

    let mut textin = false;
    let mut ignore_next_text = false;

    let mut last_frame = Instant::now();
    let mut is_first = true;
    'running: loop {
        let mut force_redraw = false;
        let event = event_pump.wait_event_timeout(1000);
        let mouse_state = event_pump.mouse_state();
        if let Some(event) = event {
            let mut fm = fm.borrow_mut();
            println!("EV: {:?}", event);
            match event {
                Event::KeyDown { keycode, keymod, .. } => {
                    let keystr = sdl2keydown2str(&event);
                    println!("STR KEY: '{}'", keystr);
                    wlcbs.on_input(keystr);
                },
                _ => {},
            }

            let mut vecref = fm_actions.borrow_mut();
            while !vecref.is_empty() {
                let act = vecref.pop().expect("something in the action vec");
                fm.action(act);
            }

            match event {
                Event::Quit {..} => {
                    break 'running
                },
                Event::KeyDown { keycode: Some(Keycode::Escape), .. } => {
                    textin = false;
                    fm.action(
                        FileManagerAction::SetPrompt(
                            String::from("[NORMAL]"), false));
                },
                Event::KeyDown { keycode: Some(Keycode::I), .. } => {
                    if !textin {
                        textin           = true;
                        ignore_next_text = true;
                        fm.action(
                            FileManagerAction::SetPrompt(
                                String::from("> "), true));
                    }
                },
                Event::KeyDown { keycode: Some(Keycode::Tab), .. } => {
                    fm.toggle_active_side();
                },
                Event::KeyDown { keycode: Some(Keycode::H), .. } => {
                    fm.process_page_control(PageControl::Back, None);
                },
                Event::KeyDown { keycode: Some(Keycode::J), .. } => {
                    fm.process_page_control(PageControl::CursorDown, None);
                },
                Event::KeyDown { keycode: Some(Keycode::K), .. } => {
                    fm.process_page_control(PageControl::CursorUp, None);
                },
                Event::KeyDown { keycode: Some(Keycode::L), .. } => {
                    fm.process_page_control(PageControl::Access, None);
                },
                Event::MouseButtonDown { x, y, .. } => {
                    fm.process_page_control(PageControl::Click((x, y)), Some((x, y)));
                },
                Event::TextInput { text, .. } => {
                    println!("TextInput: {}", text);
                    if !ignore_next_text {
                        if textin {
                            fm.action(
                                FileManagerAction::TextInput(
                                    TextInputAction::Insert(text)));
                        }
                    } else {
                        ignore_next_text = false;
                    }
                },
                Event::MouseWheel { y, direction: dir, .. } => {
                    match dir {
                        sdl2::mouse::MouseWheelDirection::Normal => {
                            fm.process_page_control(PageControl::Scroll(-y),
                            Some((mouse_state.x(), mouse_state.y())));
                            println!("DIR NORMAL");
                        },
                        sdl2::mouse::MouseWheelDirection::Flipped => {
                            fm.process_page_control(PageControl::Scroll(y),
                            Some((mouse_state.x(), mouse_state.y())));
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

            let frame_time = last_frame.elapsed().as_millis();
            //d// println!("FO {},{},{}", frame_time, is_first, force_redraw);

            if is_first || force_redraw || frame_time >= 16 {
                gui_painter.clear();
                fm.redraw(&mut gui_painter);
                gui_painter.done();
                last_frame = Instant::now();
            }

            is_first = false;
        }
    }

    Ok(())
}
