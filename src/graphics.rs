// Graphics commands for jlb: windows, drawing, text, images and input. The drawing itself
// is the shared C core in native/gfx (the same code jlb-c uses, built by build.rs), so
// this file only turns .jlb values into its inputs. It mirrors jlb-c's graphics.c.

use std::ffi::{c_char, c_void, CStr, CString};
use std::rc::Rc;
use std::time::Duration;

use crate::interp::{Frame, Interp};
use crate::number::Number;
use crate::value::*;

// ---------- the C core ----------

#[repr(C)]
#[derive(Clone, Copy)]
pub struct GfxStyle {
    set: u32,
    anchor: i32,
    angle: f64,
    pivot_anchor: i32,
    pivot_x: f64,
    pivot_y: f64,
    outline: f64,
    alpha: i32,
    scale_x: f64,
    scale_y: f64,
    tint: u32,
    smooth: bool,
    radius: f64,
    font: *mut c_void,
    size: f64,
    align: i32,
    wrap: f64,
    flip_x: bool,
    flip_y: bool,
}

impl GfxStyle {
    fn none() -> Self {
        // SAFETY: all zeros is a valid GfxStyle (nothing set, no font).
        unsafe { std::mem::zeroed() }
    }
}

const GS_ANCHOR: u32 = 1 << 0;
const GS_ANGLE: u32 = 1 << 1;
const GS_PIVOT: u32 = 1 << 2;
const GS_OUTLINE: u32 = 1 << 3;
const GS_ALPHA: u32 = 1 << 4;
const GS_SCALE_X: u32 = 1 << 5;
const GS_SCALE_Y: u32 = 1 << 6;
const GS_TINT: u32 = 1 << 7;
const GS_SMOOTH: u32 = 1 << 8;
const GS_RADIUS: u32 = 1 << 9;
const GS_FONT: u32 = 1 << 10;
const GS_SIZE: u32 = 1 << 11;
const GS_ALIGN: u32 = 1 << 12;
const GS_WRAP: u32 = 1 << 13;
const GS_FLIP_X: u32 = 1 << 14;
const GS_FLIP_Y: u32 = 1 << 15;

#[repr(C)]
#[derive(Clone, Copy)]
struct GfxPt {
    x: f64,
    y: f64,
}

#[repr(C)]
struct GfxTarget {
    img: *mut c_void,
    cam: *const c_void,
}

// GfxKind
const GK_PIXEL: i32 = 0;
const GK_LINE: i32 = 1;
const GK_ARROW: i32 = 2;
const GK_LINES: i32 = 3;
const GK_CURVE: i32 = 4;
const GK_RECT: i32 = 5;
const GK_CIRCLE: i32 = 6;
const GK_ELLIPSE: i32 = 7;
const GK_TRIANGLE: i32 = 8;
const GK_POLYGON: i32 = 9;
const GK_NGON: i32 = 10;
const GK_STAR: i32 = 11;
const GK_ARC: i32 = 12;
const GK_PIE: i32 = 13;

// GfxEventKind
const GE_KEY_DOWN: i32 = 0;
const GE_KEY_UP: i32 = 1;
const GE_TYPE: i32 = 2;
const GE_MOUSE_DOWN: i32 = 3;
const GE_MOUSE_MOVE: i32 = 5;
const GE_RESIZE: i32 = 7;
const GE_CLOSE: i32 = 8;

const GFX_ERR: usize = 200;

extern "C" {
    fn gfx_image_new(w: i32, h: i32, fill: u32) -> *mut c_void;
    fn gfx_image_load(path: *const c_char, err: *mut c_char) -> *mut c_void;
    fn gfx_image_copy(src: *const c_void) -> *mut c_void;
    fn gfx_image_part(src: *const c_void, x: i32, y: i32, w: i32, h: i32, err: *mut c_char) -> *mut c_void;
    fn gfx_image_save(img: *const c_void, path: *const c_char, err: *mut c_char) -> bool;
    fn gfx_image_unref(img: *mut c_void);
    fn gfx_image_width(img: *const c_void) -> i32;
    fn gfx_image_height(img: *const c_void) -> i32;
    fn gfx_color_from_number(v: f64, out: *mut u32) -> bool;
    fn gfx_color_to_number(argb: u32) -> f64;
    fn gfx_color_from_name(s: *const c_char, n: usize, out: *mut u32) -> bool;
    fn gfx_font_load(path: *const c_char, err: *mut c_char) -> *mut c_void;
    fn gfx_font_ref(f: *mut c_void);
    fn gfx_font_unref(f: *mut c_void);
    fn gfx_font_is_pixel(f: *const c_void) -> bool;
    fn gfx_style_current() -> *const GfxStyle;
    fn gfx_style_set(s: *const GfxStyle);
    #[allow(dead_code)]
    fn gfx_style_size() -> usize;
    fn gfx_shape(t: *mut GfxTarget, k: i32, n: *const f64, nn: i32, pts: *const GfxPt, npts: i32, color: u32, call: *const GfxStyle, err: *mut c_char) -> bool;
    fn gfx_clear(img: *mut c_void, color: u32, opaque: bool);
    fn gfx_get_pixel(t: *mut GfxTarget, x: f64, y: f64, out: *mut u32) -> bool;
    fn gfx_flood(t: *mut GfxTarget, x: f64, y: f64, color: u32);
    fn gfx_draw_image(t: *mut GfxTarget, img: *const c_void, x: f64, y: f64, call: *const GfxStyle, err: *mut c_char) -> bool;
    fn gfx_text(t: *mut GfxTarget, x: f64, y: f64, s: *const c_char, len: usize, color: u32, call: *const GfxStyle, err: *mut c_char) -> bool;
    fn gfx_text_size(s: *const c_char, len: usize, call: *const GfxStyle, w: *mut f64, h: *mut f64);
    fn gfx_screen_to_world(cam: *const c_void, w: i32, h: i32, sx: f64, sy: f64, wx: *mut f64, wy: *mut f64);
    fn gfx_key_code(name: *const c_char, n: usize) -> i32;
    fn gfx_key_name(code: i32) -> *const c_char;
    fn gfx_headless() -> bool;
    fn gfx_window_open(w: i32, h: i32, scale: f64, title: *const c_char, err: *mut c_char) -> *mut c_void;
    fn gfx_window_frame(win: *mut c_void) -> *mut c_void;
    fn gfx_window_camera(win: *mut c_void) -> *mut c_void;
    fn gfx_window_is_open(win: *const c_void) -> bool;
    fn gfx_window_title(win: *const c_void) -> *const c_char;
    fn gfx_window_close(win: *mut c_void);
    fn gfx_window_set_title(win: *mut c_void, title: *const c_char);
    fn gfx_window_resize(win: *mut c_void, w: i32, h: i32, err: *mut c_char) -> bool;
    fn gfx_window_set_resizable(win: *mut c_void, on: bool);
    fn gfx_window_set_fullscreen_allowed(win: *mut c_void, on: bool);
    fn gfx_window_set_fullscreen(win: *mut c_void, on: bool);
    fn gfx_window_set_cursor(win: *mut c_void, cursor: i32);
    fn gfx_window_set_icon(win: *mut c_void, icon: *const c_void);
    fn gfx_window_ask_before_close(win: *mut c_void, on: bool);
    fn gfx_window_show(win: *mut c_void);
    fn gfx_window_poll(win: *mut c_void, events: *mut *const c_void) -> i32;
    fn gfx_frame_delay(fps: f64) -> f64;
    fn gfx_wait_events(secs: f64);
    fn gfx_headless_frame_done();
    fn gfx_input_key(win: *const c_void, which: i32, key: i32) -> i32;
    fn gfx_input_mouse(win: *const c_void, which: i32, button: i32) -> i32;
    fn gfx_input_pointer(win: *const c_void, x: *mut f64, y: *mut f64, wheel: *mut f64, inside: *mut i32);
    fn gfx_input_typed(win: *const c_void) -> *const c_char;
    fn gfx_keys_count() -> i32;
    fn gfx_event_get(evs: *const c_void, i: i32, kind: *mut i32, key: *mut i32, repeat: *mut i32, x: *mut f64, y: *mut f64, wheel: *mut f64, text: *mut *const c_char, w: *mut i32, h: *mut i32);
    fn gfx_camera_set(win: *mut c_void, on: i32, x: f64, y: f64, zoom: f64, angle: f64);
}

fn err_text(buf: &[c_char; GFX_ERR]) -> String {
    // SAFETY: the core always writes a terminated string (or leaves the zeros).
    unsafe { CStr::from_ptr(buf.as_ptr()) }.to_string_lossy().into_owned()
}

fn cstr(s: &str) -> CString {
    CString::new(s.replace('\0', "")).unwrap_or_default()
}

// ---------- values ----------

/// An image: one reference to the core's picture. Copies share it until one is drawn into.
pub struct ImageBox(*mut c_void);

impl ImageBox {
    pub fn width(&self) -> i32 {
        // SAFETY: the pointer is a live image for as long as this box exists.
        unsafe { gfx_image_width(self.0) }
    }
    pub fn height(&self) -> i32 {
        // SAFETY: as above.
        unsafe { gfx_image_height(self.0) }
    }
}

impl Drop for ImageBox {
    fn drop(&mut self) {
        // SAFETY: this box owns one reference.
        unsafe { gfx_image_unref(self.0) }
    }
}

pub struct FontBox(*mut c_void);

impl FontBox {
    pub fn is_pixel(&self) -> bool {
        // SAFETY: a live font.
        unsafe { gfx_font_is_pixel(self.0) }
    }
}

impl Drop for FontBox {
    fn drop(&mut self) {
        // SAFETY: this box owns one reference.
        unsafe { gfx_font_unref(self.0) }
    }
}

fn image_val(p: *mut c_void) -> Value {
    Value::Image(Rc::new(ImageBox(p)))
}

// ---------- state ----------

const H_KEY: i32 = 0;
const H_KEY_UP: i32 = 1;
const H_CLICK: i32 = 2;
const H_MOVE: i32 = 3;
const H_TYPE: i32 = 4;
const H_RESIZE: i32 = 5;
const H_CLOSE: i32 = 6;
const ANY_KEY: i32 = -2;

struct Handler {
    kind: i32,
    key: i32,
    func: Rc<Function>,
}

struct Win {
    w: *mut c_void,
    handlers: Vec<Handler>,
    closed_by_program: bool,
}

pub struct Gfx {
    wins: Vec<Win>,
    sel_win: Option<usize>,
    draw_img: Option<Slot>,
    fps: f64,
    asked_open: bool,
}

impl Default for Gfx {
    fn default() -> Self {
        Gfx { wins: Vec::new(), sel_win: None, draw_img: None, fps: 60.0, asked_open: false }
    }
}

fn is_open(w: &Win) -> bool {
    // SAFETY: windows live for the whole run.
    unsafe { gfx_window_is_open(w.w) }
}

impl Gfx {
    fn pick_win(&self) -> Option<usize> {
        if let Some(i) = self.sel_win {
            if i < self.wins.len() && is_open(&self.wins[i]) {
                return Some(i);
            }
        }
        if let Some(i) = self.wins.iter().position(is_open) {
            return Some(i);
        }
        if let Some(i) = self.sel_win {
            if i < self.wins.len() {
                return Some(i);
            }
        }
        if self.wins.is_empty() {
            None
        } else {
            Some(0)
        }
    }

    fn any_open(&self) -> bool {
        self.wins.iter().any(is_open)
    }

    fn show_all(&self) {
        for w in &self.wins {
            if is_open(w) {
                // SAFETY: a live window.
                unsafe { gfx_window_show(w.w) }
            }
        }
    }
}

fn no_window<T>() -> R<T> {
    err("there's no window yet: open one with WINDOW(width, height)")
}

// ---------- inputs ----------

fn argc(name: &str, n: usize, lo: usize, hi: usize) -> R<()> {
    if n >= lo && n <= hi {
        return Ok(());
    }
    let need = if lo == hi { format!("{}", lo) } else { format!("{} to {}", lo, hi) };
    err(format!("{} takes {} input{}, but got {}", name, need, if lo == 1 && hi == 1 { "" } else { "s" }, n))
}

fn num(name: &str, v: &Value) -> R<f64> {
    let d = match v {
        Value::Num(n) => n.to_f64(),
        Value::Null => 0.0,
        Value::Text(s) => return err(format!("{} needs a number, not the text \"{}\" (use NUM() to convert)", name, s)),
        o => return err(format!("{} needs a number, not a {}", name, o.type_name())),
    };
    if !d.is_finite() {
        return err(format!("{} needs an ordinary number", name));
    }
    Ok(d)
}

fn whole(name: &str, what: &str, v: &Value, lo: i32, hi: i32) -> R<i32> {
    let d = num(name, v)?;
    if d != d.floor() || d < lo as f64 || d > hi as f64 {
        return err(format!("{}: {} must be a whole number from {} to {}", name, what, lo, hi));
    }
    Ok(d as i32)
}

fn flag(name: &str, v: &Value) -> R<bool> {
    match v {
        Value::Bool(b) => Ok(*b),
        o => err(format!("{} needs TRUE or FALSE, not a {}", name, o.type_name())),
    }
}

fn item(t: &Table, i: usize) -> Value {
    t.items.get(i).map(|s| s.borrow().clone()).unwrap_or(Value::Null)
}

fn named(t: &Table, k: &str) -> Option<Value> {
    t.named.get(k).map(|s| s.borrow().clone())
}

fn color(name: &str, v: &Value) -> R<u32> {
    let mut out = 0u32;
    match v {
        Value::Num(n) => {
            // SAFETY: plain values in, one out.
            if unsafe { gfx_color_from_number(n.to_f64(), &mut out) } {
                return Ok(out);
            }
            err(format!("{}: that number isn't a color (colors go from 0x000000 to 0xFFFFFF, or use RGB())", name))
        }
        Value::Text(s) => {
            // SAFETY: the text and its length.
            if unsafe { gfx_color_from_name(s.as_ptr() as *const c_char, s.len(), &mut out) } {
                return Ok(out);
            }
            err(format!(
                "{}: there's no color called \"{}\" (try \"red\", \"skyblue\", \"#ff8800\", or a number like 0xFF8800)",
                name, s
            ))
        }
        Value::Table(t, _) => {
            let keys = ["r", "g", "b", "alpha"];
            let mut c = [0i32, 0, 0, 255];
            for i in 0..4 {
                let part = named(t, keys[i]).unwrap_or_else(|| item(t, i));
                if i == 3 && matches!(part, Value::Null) {
                    break;
                }
                c[i] = whole(name, "each part of a color", &part, 0, 255)?;
            }
            Ok((c[3] as u32) << 24 | (c[0] as u32) << 16 | (c[1] as u32) << 8 | c[2] as u32)
        }
        o => err(format!("{} needs a color (like 0xFF8800, RGB(255, 136, 0) or \"orange\"), not a {}", name, o.type_name())),
    }
}

fn point(name: &str, v: &Value) -> R<GfxPt> {
    let Value::Table(t, _) = v else { return err(format!("{} needs points like [x, y], not a {}", name, v.type_name())) };
    let sx = named(t, "x");
    let sy = named(t, "y");
    if (sx.is_none() && t.items.is_empty()) || (sy.is_none() && t.items.len() < 2) {
        return err(format!("{} needs points like [x, y] or [x: 1, y: 2]", name));
    }
    let x = sx.unwrap_or_else(|| item(t, 0));
    let y = sy.unwrap_or_else(|| item(t, 1));
    Ok(GfxPt { x: num(name, &x)?, y: num(name, &y)? })
}

fn points(name: &str, v: &Value, min: usize) -> R<Vec<GfxPt>> {
    let Value::Table(t, _) = v else {
        return err(format!("{} needs a table of points like [[1, 1], [10, 5]], not a {}", name, v.type_name()));
    };
    if t.items.len() < min {
        return err(format!("{} needs at least {} point{}", name, min, if min == 1 { "" } else { "s" }));
    }
    t.items.iter().map(|s| point(name, &s.borrow())).collect()
}

const OPTIONS: &str = "anchor, angle, pivot, outline, alpha, scale, scale_x, scale_y, flip_x, flip_y, tint, smooth, radius, font, size, align and wrap";

/// An options table, like [angle: 45, anchor: 7]. The fonts it names are kept alive in `keep`.
fn options(name: &str, v: &Value, keep: &mut Vec<Rc<FontBox>>) -> R<GfxStyle> {
    let mut st = GfxStyle::none();
    let t = match v {
        Value::Null => return Ok(st),
        Value::Table(t, _) => t,
        o => return err(format!("{}'s options go in a table, like [angle: 45], not a {}", name, o.type_name())),
    };
    if !t.items.is_empty() {
        return err(format!("{}'s options need names, like [angle: 45]", name));
    }
    for (k, s) in t.named.iter() {
        let x = s.borrow().clone();
        match &**k {
            "anchor" => {
                st.anchor = whole(name, "anchor", &x, 1, 9)?;
                st.set |= GS_ANCHOR;
            }
            "angle" => {
                st.angle = num(name, &x)?;
                st.set |= GS_ANGLE;
            }
            "pivot" => {
                st.set |= GS_PIVOT;
                if matches!(x, Value::Num(_)) {
                    st.pivot_anchor = whole(name, "pivot (an anchor number)", &x, 1, 9)?;
                } else {
                    let p = point(name, &x)?;
                    st.pivot_anchor = 0;
                    st.pivot_x = p.x;
                    st.pivot_y = p.y;
                }
            }
            "outline" => {
                st.outline = num(name, &x)?;
                st.set |= GS_OUTLINE;
            }
            "alpha" => {
                st.alpha = whole(name, "alpha", &x, 0, 255)?;
                st.set |= GS_ALPHA;
            }
            "scale" => {
                st.scale_x = num(name, &x)?;
                st.scale_y = st.scale_x;
                st.set |= GS_SCALE_X | GS_SCALE_Y;
            }
            "scale_x" => {
                st.scale_x = num(name, &x)?;
                st.set |= GS_SCALE_X;
            }
            "scale_y" => {
                st.scale_y = num(name, &x)?;
                st.set |= GS_SCALE_Y;
            }
            "flip_x" => {
                st.flip_x = flag(name, &x)?;
                st.set |= GS_FLIP_X;
            }
            "flip_y" => {
                st.flip_y = flag(name, &x)?;
                st.set |= GS_FLIP_Y;
            }
            "tint" => {
                st.tint = color(name, &x)?;
                st.set |= GS_TINT;
            }
            "smooth" => {
                st.smooth = flag(name, &x)?;
                st.set |= GS_SMOOTH;
            }
            "radius" => {
                st.radius = num(name, &x)?;
                st.set |= GS_RADIUS;
            }
            "size" => {
                st.size = num(name, &x)?;
                st.set |= GS_SIZE;
            }
            "wrap" => {
                st.wrap = num(name, &x)?;
                st.set |= GS_WRAP;
            }
            "font" => match &x {
                Value::Font(f) => {
                    st.font = f.0;
                    st.set |= GS_FONT;
                    keep.push(f.clone());
                }
                o => return err(format!("{}: font needs a FONT (from FONT()), not a {}", name, o.type_name())),
            },
            "align" => {
                st.align = match &x {
                    Value::Text(s) if &**s == "left" => 0,
                    Value::Text(s) if &**s == "center" => 1,
                    Value::Text(s) if &**s == "right" => 2,
                    _ => return err(format!("{}: align is \"left\", \"center\" or \"right\"", name)),
                };
                st.set |= GS_ALIGN;
            }
            other => return err(format!("{} doesn't have an option called {} (its options are {})", name, other, OPTIONS)),
        }
    }
    Ok(st)
}

/// Takes an options table off the end of the inputs, when there is one.
fn take_options(name: &str, a: &mut Vec<Value>, required: usize, keep: &mut Vec<Rc<FontBox>>) -> R<GfxStyle> {
    if a.len() > required && matches!(a.last(), Some(Value::Table(..))) {
        let v = a.pop().unwrap();
        return options(name, &v, keep);
    }
    Ok(GfxStyle::none())
}

fn num_val(d: f64) -> Value {
    if d == d.floor() && d.abs() < 9e15 {
        return Value::int(d as i64);
    }
    Number::from_f64(d).map(Value::Num).unwrap_or(Value::int(0))
}

fn named_table(entries: Vec<(&str, Value)>) -> Value {
    let mut t = Table::new();
    for (k, v) in entries {
        t.named.insert(Rc::from(k), new_slot(v));
    }
    Value::new_table(t)
}

fn image_arg<'a>(name: &str, v: &'a Value) -> R<&'a Rc<ImageBox>> {
    match v {
        Value::Image(i) => Ok(i),
        o => err(format!("{} needs an IMAGE (from IMAGE(\"file.png\") or IMAGE_NEW()), not a {}", name, o.type_name())),
    }
}

// ---------- drawing targets ----------

/// Where drawing goes now. Keeps the image alive (and unshared) while it's used.
fn target(it: &mut Interp) -> R<GfxTarget> {
    if let Some(slot) = &it.gfx.draw_img {
        let mut v = slot.borrow_mut();
        if let Value::Image(img) = &mut *v {
            // Images copy like tables: drawing into one that's shared gives this name its own.
            if Rc::strong_count(img) > 1 {
                // SAFETY: a live image in, a new one out.
                let c = unsafe { gfx_image_copy(img.0) };
                if c.is_null() {
                    return err("out of memory for the image");
                }
                *img = Rc::new(ImageBox(c));
            }
            return Ok(GfxTarget { img: img.0, cam: std::ptr::null() });
        }
    }
    let Some(i) = it.gfx.pick_win() else { return err("there's no window to draw in yet: open one with WINDOW(width, height)") };
    let w = it.gfx.wins[i].w;
    // SAFETY: a live window.
    Ok(unsafe { GfxTarget { img: gfx_window_frame(w), cam: gfx_window_camera(w) } })
}

/// DRAW_TO(name) of an image: drawing goes into that variable's image.
pub fn draw_to_slot(it: &mut Interp, s: Slot) -> R<()> {
    if s.is_const() {
        return err("DRAW_TO can't draw into a constant (CONST) image");
    }
    it.gfx.draw_img = Some(s);
    Ok(())
}

fn window_set(name: &str, w: *mut c_void, v: &Value) -> R<()> {
    let t = match v {
        Value::Table(t, _) if t.items.is_empty() => t,
        _ => return err(format!("{}'s settings go in a table, like [resizable: FALSE]", name)),
    };
    for (k, s) in t.named.iter() {
        let x = s.borrow().clone();
        // SAFETY (all of these): a live window and plain values.
        match &**k {
            "resizable" => unsafe { gfx_window_set_resizable(w, flag(name, &x)?) },
            "fullscreen_allowed" => unsafe { gfx_window_set_fullscreen_allowed(w, flag(name, &x)?) },
            "fullscreen" => unsafe { gfx_window_set_fullscreen(w, flag(name, &x)?) },
            "title" => unsafe { gfx_window_set_title(w, cstr(&display(&x)).as_ptr()) },
            "cursor" => {
                let names = ["arrow", "hand", "text", "crosshair", "hidden"];
                let c = match &x {
                    Value::Text(s) => names.iter().position(|n| **s == **n),
                    _ => None,
                };
                let Some(c) = c else {
                    return err(format!("{}: cursor is \"arrow\", \"hand\", \"text\", \"crosshair\" or \"hidden\"", name));
                };
                unsafe { gfx_window_set_cursor(w, c as i32) }
            }
            "icon" => match &x {
                Value::Text(path) => {
                    let mut e = [0 as c_char; GFX_ERR];
                    let img = unsafe { gfx_image_load(cstr(path).as_ptr(), e.as_mut_ptr()) };
                    if img.is_null() {
                        return err(err_text(&e));
                    }
                    unsafe {
                        gfx_window_set_icon(w, img);
                        gfx_image_unref(img);
                    }
                }
                other => {
                    let img = image_arg(name, other)?;
                    unsafe { gfx_window_set_icon(w, img.0) }
                }
            },
            other => {
                return err(format!(
                    "{} doesn't have a setting called {} (its settings are resizable, fullscreen_allowed, fullscreen, title, cursor and icon)",
                    name, other
                ))
            }
        }
    }
    Ok(())
}

// ---------- events ----------

fn set_handler(it: &mut Interp, name: &str, kind: i32, key: i32, f: &Value) -> R<()> {
    let func = match f {
        Value::Func(f) => Some(f.clone()),
        Value::Null => None,
        o => return err(format!("{} needs a function (or NULL to stop), not a {}", name, o.type_name())),
    };
    let Some(i) = it.gfx.pick_win() else {
        return err(format!("there's no window yet: open one with WINDOW(width, height) before {}", name));
    };
    let w = &mut it.gfx.wins[i];
    match (w.handlers.iter().position(|h| h.kind == kind && h.key == key), func) {
        (Some(p), Some(f)) => w.handlers[p].func = f,
        (Some(p), None) => {
            w.handlers.swap_remove(p);
        }
        (None, Some(f)) => w.handlers.push(Handler { kind, key, func: f }),
        (None, None) => {}
    }
    if kind == H_CLOSE {
        let ask = w.handlers.iter().any(|h| h.kind == H_CLOSE);
        // SAFETY: a live window.
        unsafe { gfx_window_ask_before_close(w.w, ask) }
    }
    Ok(())
}

/// Calls a handler with up to these inputs (fewer if the function takes fewer).
fn call_handler(it: &mut Interp, f: &Rc<Function>, args: Vec<Value>) -> R<Value> {
    let d = f.decl.clone();
    let rest = d.params.last().is_some_and(|p| p.rest);
    let take = if rest || args.len() < d.params.len() { args.len() } else { d.params.len() };
    let vals: Vec<Slot> = args.into_iter().take(take).map(new_slot).collect();
    // After the program's code has ended there's no frame left to call from.
    let pushed = it.frames.is_empty();
    if pushed {
        it.frames.push(Frame { func: None, scopes: vec![OrdMap::new()], env: f.env.clone(), line: 0, src: f.src.clone() });
    }
    let r = it.enter_call(f.clone(), d, vals);
    if pushed {
        it.frames.pop();
    }
    r
}

const BUTTONS: [&str; 3] = ["left", "right", "middle"];

fn key_name(code: i32) -> Option<String> {
    // SAFETY: the core gives back a static string, or NULL.
    let p = unsafe { gfx_key_name(code) };
    if p.is_null() {
        None
    } else {
        Some(unsafe { CStr::from_ptr(p) }.to_string_lossy().into_owned())
    }
}

struct Event {
    kind: i32,
    key: i32,
    repeat: bool,
    x: f64,
    y: f64,
    text: String,
    w: i32,
    h: i32,
}

fn find(it: &Interp, wi: usize, kind: i32, key: i32) -> Option<Rc<Function>> {
    it.gfx.wins[wi].handlers.iter().find(|h| h.kind == kind && h.key == key).map(|h| h.func.clone())
}

fn dispatch(it: &mut Interp, wi: usize, e: &Event) -> R<()> {
    match e.kind {
        GE_KEY_DOWN | GE_KEY_UP => {
            if e.repeat {
                return Ok(());
            }
            let Some(k) = key_name(e.key) else { return Ok(()) };
            let kind = if e.kind == GE_KEY_DOWN { H_KEY } else { H_KEY_UP };
            if let Some(f) = find(it, wi, kind, e.key) {
                call_handler(it, &f, vec![Value::text(&k)])?;
            }
            if let Some(f) = find(it, wi, kind, ANY_KEY) {
                call_handler(it, &f, vec![Value::text(&k)])?;
            }
        }
        GE_MOUSE_DOWN => {
            if let Some(f) = find(it, wi, H_CLICK, 0) {
                let b = BUTTONS[if (0..3).contains(&e.key) { e.key as usize } else { 0 }];
                call_handler(it, &f, vec![num_val(e.x), num_val(e.y), Value::text(b)])?;
            }
        }
        GE_MOUSE_MOVE => {
            if let Some(f) = find(it, wi, H_MOVE, 0) {
                call_handler(it, &f, vec![num_val(e.x), num_val(e.y)])?;
            }
        }
        GE_TYPE => {
            if let Some(f) = find(it, wi, H_TYPE, 0) {
                call_handler(it, &f, vec![Value::text(&e.text)])?;
            }
        }
        GE_RESIZE => {
            if let Some(f) = find(it, wi, H_RESIZE, 0) {
                call_handler(it, &f, vec![Value::int(e.w as i64), Value::int(e.h as i64)])?;
            }
        }
        GE_CLOSE => {
            let close = match find(it, wi, H_CLOSE, 0) {
                Some(f) => !matches!(call_handler(it, &f, vec![])?, Value::Bool(false)),
                None => true,
            };
            if close {
                it.gfx.wins[wi].closed_by_program = true;
                // SAFETY: a live window.
                unsafe { gfx_window_close(it.gfx.wins[wi].w) }
            }
        }
        _ => {}
    }
    Ok(())
}

/// A new input frame for every window, and their events to the handlers.
fn pump(it: &mut Interp) -> R<()> {
    let mut i = 0;
    while i < it.gfx.wins.len() {
        let w = it.gfx.wins[i].w;
        let mut evs: *const c_void = std::ptr::null();
        // SAFETY: a live window; the events stay valid until its next poll, and are
        // copied out before any handler runs.
        let n = unsafe { gfx_window_poll(w, &mut evs) };
        let mut list = Vec::with_capacity(n.max(0) as usize);
        for k in 0..n {
            let (mut kind, mut key, mut repeat, mut x, mut y, mut wheel, mut text, mut ww, mut hh) =
                (0, 0, 0, 0.0, 0.0, 0.0, std::ptr::null::<c_char>(), 0, 0);
            unsafe { gfx_event_get(evs, k, &mut kind, &mut key, &mut repeat, &mut x, &mut y, &mut wheel, &mut text, &mut ww, &mut hh) };
            let text = if text.is_null() { String::new() } else { unsafe { CStr::from_ptr(text) }.to_string_lossy().into_owned() };
            list.push(Event { kind, key, repeat: repeat != 0, x, y, text, w: ww, h: hh });
        }
        for e in &list {
            dispatch(it, i, e)?;
        }
        i += 1;
    }
    Ok(())
}

/// After the program's code: its windows stay up, and their events still work.
pub fn finish(it: &mut Interp) -> R<()> {
    if it.gfx.wins.is_empty() {
        return Ok(());
    }
    // SAFETY: a plain query.
    if unsafe { gfx_headless() } {
        it.gfx.show_all();
        return Ok(());
    }
    it.gfx.show_all();
    while it.gfx.any_open() {
        // SAFETY: waits on the core's own lock.
        unsafe { gfx_wait_events(0.25) };
        pump(it)?;
        it.gfx.show_all();
    }
    Ok(())
}

// ---------- values ----------

thread_local! {
    // Window titles for PRINT (which has no access to the interpreter).
    static TITLES: std::cell::RefCell<Vec<String>> = const { std::cell::RefCell::new(Vec::new()) };
}

pub fn window_title(id: usize) -> String {
    TITLES.with(|t| t.borrow().get(id).cloned().unwrap_or_default())
}

fn remember_titles(g: &Gfx) {
    let titles: Vec<String> = g
        .wins
        .iter()
        // SAFETY: live windows; the title is copied out.
        .map(|w| unsafe { CStr::from_ptr(gfx_window_title(w.w)) }.to_string_lossy().into_owned())
        .collect();
    TITLES.with(|t| *t.borrow_mut() = titles);
}

pub fn field(it: &Interp, base: &Value, name: &str) -> R<Value> {
    let (w, h) = match base {
        Value::Image(img) => (img.width(), img.height()),
        Value::Window(id) if *id < it.gfx.wins.len() => {
            let win = it.gfx.wins[*id].w;
            if name == "title" {
                // SAFETY: a live window.
                return Ok(Value::text(&unsafe { CStr::from_ptr(gfx_window_title(win)) }.to_string_lossy()));
            }
            // SAFETY: a live window and its frame.
            unsafe {
                let f = gfx_window_frame(win);
                (gfx_image_width(f), gfx_image_height(f))
            }
        }
        _ => return err(format!("a FONT has no .{}", name)),
    };
    match name {
        "width" => Ok(Value::int(w as i64)),
        "height" => Ok(Value::int(h as i64)),
        _ => err(format!(
            "a{} {} has .width and .height{}, not .{}",
            if matches!(base, Value::Image(_)) { "n" } else { "" },
            base.type_name(),
            if matches!(base, Value::Window(_)) { " and .title" } else { "" },
            name
        )),
    }
}

fn key_arg(name: &str, v: &Value) -> R<i32> {
    let Value::Text(s) = v else { return err(format!("{} needs a key's name, like \"a\", \"space\" or \"left\", not a {}", name, v.type_name())) };
    if &**s == "any" {
        return Ok(ANY_KEY);
    }
    // SAFETY: the text and its length.
    let k = unsafe { gfx_key_code(s.as_ptr() as *const c_char, s.len()) };
    if k < 0 {
        return err(format!(
            "{}: there's no key called \"{}\" (keys are letters and numbers like \"a\" and \"1\", \"space\", \"enter\", \"escape\", \"tab\", \"backspace\", \"delete\", \"left\", \"right\", \"up\", \"down\", \"shift\", \"ctrl\", \"alt\", \"f1\" to \"f12\", \"home\", \"end\", \"pageup\", \"pagedown\", punctuation like \"-\", or \"any\")",
            name, s
        ));
    }
    Ok(k)
}

fn button_arg(name: &str, a: &[Value]) -> R<i32> {
    match a.first() {
        None | Some(Value::Null) => Ok(0),
        Some(Value::Text(s)) if BUTTONS.contains(&&**s) => Ok(BUTTONS.iter().position(|b| **b == **s).unwrap() as i32),
        _ => err(format!("{}: the mouse buttons are \"left\", \"right\" and \"middle\"", name)),
    }
}

fn style_table() -> Value {
    // SAFETY: the core's current style, read right away.
    let s = unsafe { *gfx_style_current() };
    let mut e: Vec<(&str, Value)> = Vec::new();
    if s.set & GS_ANCHOR != 0 {
        e.push(("anchor", Value::int(s.anchor as i64)));
    }
    if s.set & GS_ANGLE != 0 {
        e.push(("angle", num_val(s.angle)));
    }
    if s.set & GS_PIVOT != 0 {
        if s.pivot_anchor != 0 {
            e.push(("pivot", Value::int(s.pivot_anchor as i64)));
        } else {
            e.push(("pivot", Value::new_table(Table::from_values(vec![num_val(s.pivot_x), num_val(s.pivot_y)]))));
        }
    }
    if s.set & GS_OUTLINE != 0 {
        e.push(("outline", num_val(s.outline)));
    }
    if s.set & GS_ALPHA != 0 {
        e.push(("alpha", Value::int(s.alpha as i64)));
    }
    if s.set & GS_SCALE_X != 0 {
        e.push(("scale_x", num_val(s.scale_x)));
    }
    if s.set & GS_SCALE_Y != 0 {
        e.push(("scale_y", num_val(s.scale_y)));
    }
    if s.set & GS_FLIP_X != 0 {
        e.push(("flip_x", Value::Bool(s.flip_x)));
    }
    if s.set & GS_FLIP_Y != 0 {
        e.push(("flip_y", Value::Bool(s.flip_y)));
    }
    if s.set & GS_TINT != 0 {
        // SAFETY: plain value.
        e.push(("tint", num_val(unsafe { gfx_color_to_number(s.tint) })));
    }
    if s.set & GS_SMOOTH != 0 {
        e.push(("smooth", Value::Bool(s.smooth)));
    }
    if s.set & GS_RADIUS != 0 {
        e.push(("radius", num_val(s.radius)));
    }
    if s.set & GS_FONT != 0 && !s.font.is_null() {
        // SAFETY: the style holds a reference; this value takes one of its own.
        unsafe { gfx_font_ref(s.font) };
        e.push(("font", Value::Font(Rc::new(FontBox(s.font)))));
    }
    if s.set & GS_SIZE != 0 {
        e.push(("size", num_val(s.size)));
    }
    if s.set & GS_ALIGN != 0 {
        e.push(("align", Value::text(["left", "center", "right"][s.align.clamp(0, 2) as usize])));
    }
    if s.set & GS_WRAP != 0 {
        e.push(("wrap", num_val(s.wrap)));
    }
    named_table(e)
}

// ---------- the commands ----------

pub const NAMES: &[&str] = &[
    "WINDOW", "DRAW_TO", "SHOW", "FPS", "WINDOW_OPEN", "WINDOW_CLOSE", "WINDOW_TITLE", "WINDOW_RESIZE", "WINDOW_SET",
    "STYLE", "RESET_STYLE", "RGB", "CLEAR", "PIXEL", "GET_PIXEL", "LINE", "LINES", "ARROW", "CURVE", "RECT", "CIRCLE",
    "ELLIPSE", "TRIANGLE", "POLYGON", "NGON", "STAR", "ARC", "PIE", "FLOOD", "TEXT", "TEXT_SIZE", "FONT", "IMAGE",
    "IMAGE_NEW", "PART", "DRAW", "SAVE_IMAGE", "SCREENSHOT", "CAMERA", "KEY_DOWN", "KEY_PRESSED", "KEY_RELEASED",
    "KEYS_DOWN", "TYPED", "MOUSE", "MOUSE_PRESSED", "MOUSE_RELEASED", "ON_KEY", "ON_KEY_UP", "ON_CLICK",
    "ON_MOUSE_MOVE", "ON_TYPE", "ON_RESIZE", "ON_CLOSE",
];

fn draw_shape(it: &mut Interp, name: &str, kind: i32, nnums: usize, has_extra: bool, mut a: Vec<Value>) -> R<Value> {
    let mut keep = Vec::new();
    let mut st = take_options(name, &mut a, nnums + 1, &mut keep)?;
    argc(name, a.len(), nnums + 1, nnums + 1 + has_extra as usize)?;
    let mut v = [0f64; 8];
    for i in 0..nnums {
        v[i] = num(name, &a[i])?;
    }
    let c = color(name, &a[nnums])?;
    if a.len() > nnums + 1 && !matches!(a[nnums + 1], Value::Null) {
        let extra = num(name, &a[nnums + 1])?;
        if kind == GK_LINE || kind == GK_ARROW || kind == GK_ARC {
            v[nnums] = extra;
        } else {
            st.outline = extra;
            st.set |= GS_OUTLINE;
        }
    }
    let mut t = target(it)?;
    let mut e = [0 as c_char; GFX_ERR];
    // SAFETY: a live target and plain values.
    if !unsafe { gfx_shape(&mut t, kind, v.as_ptr(), nnums as i32 + 1, std::ptr::null(), 0, c, &st, e.as_mut_ptr()) } {
        return err(format!("{}: {}", name, err_text(&e)));
    }
    Ok(Value::Null)
}

fn point_shape(it: &mut Interp, name: &str, kind: i32, min: usize, mut a: Vec<Value>) -> R<Value> {
    let mut keep = Vec::new();
    let mut st = take_options(name, &mut a, 2, &mut keep)?;
    argc(name, a.len(), 2, 3)?;
    let p = points(name, &a[0], min)?;
    let c = color(name, &a[1])?;
    let mut extra = 0.0;
    if a.len() > 2 && !matches!(a[2], Value::Null) {
        extra = num(name, &a[2])?;
    }
    if kind == GK_POLYGON {
        st.outline = extra;
        if a.len() > 2 {
            st.set |= GS_OUTLINE;
        }
    }
    let mut t = target(it)?;
    let mut e = [0 as c_char; GFX_ERR];
    // SAFETY: a live target, the points, plain values.
    if !unsafe { gfx_shape(&mut t, kind, &extra, 1, p.as_ptr(), p.len() as i32, c, &st, e.as_mut_ptr()) } {
        return err(format!("{}: {}", name, err_text(&e)));
    }
    Ok(Value::Null)
}

fn win_arg(it: &Interp, name: &str, a: &[Value]) -> R<usize> {
    match a.first() {
        Some(Value::Window(id)) => Ok(*id),
        Some(Value::Null) | None => match it.gfx.pick_win() {
            Some(i) => Ok(i),
            None => no_window(),
        },
        Some(o) => err(format!("{} needs a WINDOW (from WINDOW()), not a {}", name, o.type_name())),
    }
}

fn sleep_for(it: &mut Interp, d: f64) -> R<()> {
    if d <= 0.0 {
        return Ok(());
    }
    if it.baton.is_some() {
        it.task_sleep(d)
    } else {
        std::thread::sleep(Duration::from_secs_f64(d));
        Ok(())
    }
}

fn input_win(it: &Interp) -> Option<*mut c_void> {
    it.gfx.pick_win().map(|i| it.gfx.wins[i].w)
}

pub fn call(it: &mut Interp, name: &'static str, mut a: Vec<Value>) -> R<Value> {
    let mut e = [0 as c_char; GFX_ERR];
    let mut keep: Vec<Rc<FontBox>> = Vec::new();
    // SAFETY (the unsafe blocks below): calls into the core with live windows, images and
    // fonts (kept alive by the values in `a`, `keep` and the interpreter), and plain values.
    match name {
        "WINDOW" => {
            let set = if a.len() > 2 && matches!(a.last(), Some(Value::Table(..))) { a.pop() } else { None };
            argc(name, a.len(), 2, 4)?;
            let w = whole(name, "the width", &a[0], 1, 16384)?;
            let h = whole(name, "the height", &a[1], 1, 16384)?;
            let title = match a.get(2) {
                Some(v) if !matches!(v, Value::Null) => display(v),
                _ => it.frames.first().map(|f| f.src.name.to_string()).unwrap_or_default(),
            };
            let mut scale = 1.0;
            if let Some(v) = a.get(3) {
                if !matches!(v, Value::Null) {
                    scale = num(name, v)?;
                }
            }
            let gw = unsafe { gfx_window_open(w, h, scale, cstr(&title).as_ptr(), e.as_mut_ptr()) };
            if gw.is_null() {
                return err(err_text(&e));
            }
            it.gfx.wins.push(Win { w: gw, handlers: Vec::new(), closed_by_program: false });
            remember_titles(&it.gfx);
            let id = it.gfx.wins.len() - 1;
            if let Some(s) = set {
                window_set(name, gw, &s)?;
                remember_titles(&it.gfx);
            }
            Ok(Value::Window(id))
        }
        "DRAW_TO" => {
            argc(name, a.len(), 0, 1)?;
            it.gfx.draw_img = None;
            match a.first() {
                None | Some(Value::Null) => {
                    it.gfx.sel_win = None;
                    Ok(Value::Null)
                }
                Some(Value::Image(_)) => err("DRAW_TO needs the name of an image's variable, like DRAW_TO(background), so the drawing has somewhere to stay"),
                Some(Value::Window(id)) => {
                    it.gfx.sel_win = Some(*id);
                    Ok(Value::Null)
                }
                Some(o) => err(format!("DRAW_TO needs a WINDOW or the name of an IMAGE's variable, not a {}", o.type_name())),
            }
        }
        "SHOW" => {
            argc(name, a.len(), 0, 0)?;
            it.gfx.show_all();
            unsafe { gfx_headless_frame_done() };
            use std::io::Write;
            let _ = it.out.flush();
            let d = unsafe { gfx_frame_delay(it.gfx.fps) };
            sleep_for(it, d)?;
            pump(it)?;
            // Closing the last window ends a program that never asks WINDOW_OPEN() (one that
            // does handles closing itself).
            if !it.gfx.asked_open && !it.gfx.wins.is_empty() && !it.gfx.any_open() && it.gfx.wins.iter().any(|w| !w.closed_by_program) {
                return Err(Fault::End);
            }
            Ok(Value::Null)
        }
        "FPS" => {
            argc(name, a.len(), 0, 1)?;
            match a.first() {
                Some(v) if !matches!(v, Value::Null) => {
                    let f = num(name, v)?;
                    if !(0.0..=1000.0).contains(&f) {
                        return err("FPS goes from 0 (as fast as possible) to 1000");
                    }
                    it.gfx.fps = f;
                    Ok(Value::Null)
                }
                _ => Ok(num_val(it.gfx.fps)),
            }
        }
        "WINDOW_OPEN" => {
            argc(name, a.len(), 0, 1)?;
            it.gfx.asked_open = true;
            if a.is_empty() && it.gfx.wins.is_empty() {
                return Ok(Value::Bool(false));
            }
            let i = win_arg(it, name, &a)?;
            Ok(Value::Bool(is_open(&it.gfx.wins[i])))
        }
        "WINDOW_CLOSE" => {
            argc(name, a.len(), 0, 1)?;
            let i = win_arg(it, name, &a)?;
            it.gfx.wins[i].closed_by_program = true;
            unsafe { gfx_window_close(it.gfx.wins[i].w) };
            Ok(Value::Null)
        }
        "WINDOW_TITLE" => {
            argc(name, a.len(), 0, 1)?;
            let Some(i) = it.gfx.pick_win() else { return no_window() };
            let w = it.gfx.wins[i].w;
            if a.is_empty() {
                return Ok(Value::text(&unsafe { CStr::from_ptr(gfx_window_title(w)) }.to_string_lossy()));
            }
            unsafe { gfx_window_set_title(w, cstr(&display(&a[0])).as_ptr()) };
            remember_titles(&it.gfx);
            Ok(Value::Null)
        }
        "WINDOW_RESIZE" => {
            argc(name, a.len(), 2, 2)?;
            let Some(i) = it.gfx.pick_win() else { return no_window() };
            let w = whole(name, "the width", &a[0], 1, 16384)?;
            let h = whole(name, "the height", &a[1], 1, 16384)?;
            if !unsafe { gfx_window_resize(it.gfx.wins[i].w, w, h, e.as_mut_ptr()) } {
                return err(err_text(&e));
            }
            Ok(Value::Null)
        }
        "WINDOW_SET" => {
            argc(name, a.len(), 1, 1)?;
            let Some(i) = it.gfx.pick_win() else { return no_window() };
            window_set(name, it.gfx.wins[i].w, &a[0])?;
            remember_titles(&it.gfx);
            Ok(Value::Null)
        }
        "STYLE" => {
            argc(name, a.len(), 0, 1)?;
            if a.is_empty() {
                return Ok(style_table());
            }
            let st = options(name, &a[0], &mut keep)?;
            unsafe { gfx_style_set(&st) };
            Ok(Value::Null)
        }
        "RESET_STYLE" => {
            argc(name, a.len(), 0, 0)?;
            unsafe { gfx_style_set(&GfxStyle::none()) };
            Ok(Value::Null)
        }
        "RGB" => {
            argc(name, a.len(), 3, 4)?;
            let what = ["red", "green", "blue", "alpha"];
            let mut c = [0i32, 0, 0, 255];
            for (i, v) in a.iter().enumerate() {
                c[i] = whole(name, what[i], v, 0, 255)?;
            }
            let argb = (c[3] as u32) << 24 | (c[0] as u32) << 16 | (c[1] as u32) << 8 | c[2] as u32;
            Ok(num_val(unsafe { gfx_color_to_number(argb) }))
        }
        "CLEAR" => {
            argc(name, a.len(), 0, 1)?;
            let c = if a.is_empty() { 0xFF000000 } else { color(name, &a[0])? };
            let opaque = it.gfx.draw_img.is_none();
            let t = target(it)?;
            unsafe { gfx_clear(t.img, c, opaque) };
            Ok(Value::Null)
        }
        "PIXEL" => draw_shape(it, name, GK_PIXEL, 2, false, a),
        "GET_PIXEL" => {
            argc(name, a.len(), 2, 2)?;
            let x = num(name, &a[0])?;
            let y = num(name, &a[1])?;
            let mut t = target(it)?;
            let mut c = 0u32;
            if unsafe { gfx_get_pixel(&mut t, x, y, &mut c) } {
                return Ok(num_val(unsafe { gfx_color_to_number(c) }));
            }
            Ok(Value::Null)
        }
        "LINE" => draw_shape(it, name, GK_LINE, 4, true, a),
        "ARROW" => draw_shape(it, name, GK_ARROW, 4, true, a),
        "LINES" => point_shape(it, name, GK_LINES, 1, a),
        "CURVE" => point_shape(it, name, GK_CURVE, 2, a),
        "POLYGON" => point_shape(it, name, GK_POLYGON, 3, a),
        "RECT" => draw_shape(it, name, GK_RECT, 4, true, a),
        "CIRCLE" => draw_shape(it, name, GK_CIRCLE, 3, true, a),
        "ELLIPSE" => draw_shape(it, name, GK_ELLIPSE, 4, true, a),
        "NGON" => draw_shape(it, name, GK_NGON, 4, true, a),
        "STAR" => draw_shape(it, name, GK_STAR, 5, true, a),
        "ARC" => draw_shape(it, name, GK_ARC, 5, true, a),
        "PIE" => draw_shape(it, name, GK_PIE, 5, true, a),
        "TRIANGLE" => {
            let mut st = take_options(name, &mut a, 7, &mut keep)?;
            argc(name, a.len(), 7, 8)?;
            let mut p = [GfxPt { x: 0.0, y: 0.0 }; 3];
            for (i, q) in p.iter_mut().enumerate() {
                q.x = num(name, &a[i * 2])?;
                q.y = num(name, &a[i * 2 + 1])?;
            }
            let c = color(name, &a[6])?;
            if a.len() > 7 && !matches!(a[7], Value::Null) {
                st.outline = num(name, &a[7])?;
                st.set |= GS_OUTLINE;
            }
            let mut t = target(it)?;
            let none = 0.0;
            if !unsafe { gfx_shape(&mut t, GK_TRIANGLE, &none, 1, p.as_ptr(), 3, c, &st, e.as_mut_ptr()) } {
                return err(format!("{}: {}", name, err_text(&e)));
            }
            Ok(Value::Null)
        }
        "FLOOD" => {
            argc(name, a.len(), 3, 3)?;
            let x = num(name, &a[0])?;
            let y = num(name, &a[1])?;
            let c = color(name, &a[2])?;
            let mut t = target(it)?;
            unsafe { gfx_flood(&mut t, x, y, c) };
            Ok(Value::Null)
        }
        "TEXT" => {
            let mut st = take_options(name, &mut a, 4, &mut keep)?;
            argc(name, a.len(), 4, 5)?;
            let x = num(name, &a[0])?;
            let y = num(name, &a[1])?;
            let c = color(name, &a[3])?;
            if a.len() > 4 && !matches!(a[4], Value::Null) {
                st.size = num(name, &a[4])?;
                st.set |= GS_SIZE;
            }
            let mut t = target(it)?;
            let s = display(&a[2]);
            unsafe { gfx_text(&mut t, x, y, s.as_ptr() as *const c_char, s.len(), c, &st, e.as_mut_ptr()) };
            Ok(Value::Null)
        }
        "TEXT_SIZE" => {
            let mut st = take_options(name, &mut a, 1, &mut keep)?;
            argc(name, a.len(), 1, 2)?;
            if a.len() > 1 && !matches!(a[1], Value::Null) {
                st.size = num(name, &a[1])?;
                st.set |= GS_SIZE;
            }
            let s = display(&a[0]);
            let (mut w, mut h) = (0.0, 0.0);
            unsafe { gfx_text_size(s.as_ptr() as *const c_char, s.len(), &st, &mut w, &mut h) };
            Ok(named_table(vec![("width", num_val(w)), ("height", num_val(h))]))
        }
        "FONT" => {
            argc(name, a.len(), 0, 1)?;
            let path = match a.first() {
                None | Some(Value::Null) => None,
                Some(Value::Text(s)) => Some(cstr(s)),
                _ => return err("FONT needs a font file's name (like \"Roboto.ttf\"), or \"pixel\""),
            };
            let f = unsafe { gfx_font_load(path.as_ref().map_or(std::ptr::null(), |p| p.as_ptr()), e.as_mut_ptr()) };
            if f.is_null() {
                return err(err_text(&e));
            }
            Ok(Value::Font(Rc::new(FontBox(f))))
        }
        "IMAGE" => {
            argc(name, a.len(), 1, 1)?;
            let Value::Text(path) = &a[0] else { return err("IMAGE needs an image file's name, like \"hero.png\"") };
            let img = unsafe { gfx_image_load(cstr(path).as_ptr(), e.as_mut_ptr()) };
            if img.is_null() {
                return err(err_text(&e));
            }
            Ok(image_val(img))
        }
        "IMAGE_NEW" => {
            argc(name, a.len(), 2, 3)?;
            let w = whole(name, "the width", &a[0], 1, 16384)?;
            let h = whole(name, "the height", &a[1], 1, 16384)?;
            let c = if a.len() > 2 { color(name, &a[2])? } else { 0 };
            let img = unsafe { gfx_image_new(w, h, c) };
            if img.is_null() {
                return err(format!("a {} x {} image is too big", w, h));
            }
            Ok(image_val(img))
        }
        "PART" => {
            argc(name, a.len(), 5, 5)?;
            let src = image_arg(name, &a[0])?.clone();
            let mut v = [0i32; 4];
            for i in 0..4 {
                v[i] = whole(name, if i < 2 { "the position" } else { "the size" }, &a[i + 1], if i < 2 { -1000000 } else { 1 }, 1000000)?;
            }
            let img = unsafe { gfx_image_part(src.0, v[0], v[1], v[2], v[3], e.as_mut_ptr()) };
            if img.is_null() {
                return err(format!("PART: {}", err_text(&e)));
            }
            Ok(image_val(img))
        }
        "DRAW" => {
            let st = take_options(name, &mut a, 3, &mut keep)?;
            argc(name, a.len(), 3, 3)?;
            let img = image_arg(name, &a[0])?.clone();
            let x = num(name, &a[1])?;
            let y = num(name, &a[2])?;
            let mut t = target(it)?;
            unsafe { gfx_draw_image(&mut t, img.0, x, y, &st, e.as_mut_ptr()) };
            Ok(Value::Null)
        }
        "SAVE_IMAGE" => {
            argc(name, a.len(), 2, 2)?;
            let img = image_arg(name, &a[0])?;
            let Value::Text(path) = &a[1] else { return err("SAVE_IMAGE needs a file name, like \"picture.png\"") };
            if !unsafe { gfx_image_save(img.0, cstr(path).as_ptr(), e.as_mut_ptr()) } {
                return err(err_text(&e));
            }
            Ok(Value::Null)
        }
        "SCREENSHOT" => {
            argc(name, a.len(), 0, 0)?;
            let t = target(it)?;
            let img = unsafe { gfx_image_copy(t.img) };
            if img.is_null() {
                return err("out of memory for the screenshot");
            }
            Ok(image_val(img))
        }
        "CAMERA" => {
            argc(name, a.len(), 0, 4)?;
            let Some(i) = it.gfx.pick_win() else { return no_window() };
            let w = it.gfx.wins[i].w;
            if a.is_empty() {
                unsafe { gfx_camera_set(w, 0, 0.0, 0.0, 1.0, 0.0) };
                return Ok(Value::Null);
            }
            if a.len() == 1 {
                return err("CAMERA needs both x and y (or nothing, to go back to the window's own positions)");
            }
            let x = num(name, &a[0])?;
            let y = num(name, &a[1])?;
            let zoom = match a.get(2) {
                Some(v) if !matches!(v, Value::Null) => num(name, v)?,
                _ => 1.0,
            };
            let angle = match a.get(3) {
                Some(v) if !matches!(v, Value::Null) => num(name, v)?,
                _ => 0.0,
            };
            if zoom <= 0.0 {
                return err("CAMERA's zoom must be more than 0");
            }
            unsafe { gfx_camera_set(w, 1, x, y, zoom, angle) };
            Ok(Value::Null)
        }
        "KEY_DOWN" | "KEY_PRESSED" | "KEY_RELEASED" => {
            argc(name, a.len(), 1, 1)?;
            let key = key_arg(name, &a[0])?;
            let which = match name {
                "KEY_DOWN" => 0,
                "KEY_PRESSED" => 1,
                _ => 2,
            };
            let r = match input_win(it) {
                None => false,
                Some(w) if key == ANY_KEY => (0..unsafe { gfx_keys_count() }).any(|k| (unsafe { gfx_input_key(w, which, k) }) != 0),
                Some(w) => (unsafe { gfx_input_key(w, which, key) }) != 0,
            };
            Ok(Value::Bool(r))
        }
        "KEYS_DOWN" => {
            argc(name, a.len(), 0, 0)?;
            let mut keys = Vec::new();
            if let Some(w) = input_win(it) {
                for k in 0..unsafe { gfx_keys_count() } {
                    if unsafe { gfx_input_key(w, 0, k) } != 0 {
                        if let Some(n) = key_name(k) {
                            keys.push(Value::text(&n));
                        }
                    }
                }
            }
            Ok(Value::new_table(Table::from_values(keys)))
        }
        "TYPED" => {
            argc(name, a.len(), 0, 0)?;
            Ok(match input_win(it) {
                Some(w) => Value::text(&unsafe { CStr::from_ptr(gfx_input_typed(w)) }.to_string_lossy()),
                None => Value::text(""),
            })
        }
        "MOUSE" => {
            argc(name, a.len(), 0, 0)?;
            let (mut x, mut y, mut wheel, mut inside) = (0.0, 0.0, 0.0, 0);
            let (mut wx, mut wy) = (0.0, 0.0);
            let mut b = [false; 3];
            if let Some(w) = input_win(it) {
                unsafe {
                    gfx_input_pointer(w, &mut x, &mut y, &mut wheel, &mut inside);
                    let f = gfx_window_frame(w);
                    gfx_screen_to_world(gfx_window_camera(w), gfx_image_width(f), gfx_image_height(f), x, y, &mut wx, &mut wy);
                    for (i, d) in b.iter_mut().enumerate() {
                        *d = gfx_input_mouse(w, 0, i as i32) != 0;
                    }
                }
            } else {
                wx = x;
                wy = y;
            }
            Ok(named_table(vec![
                ("x", num_val(x)),
                ("y", num_val(y)),
                ("world_x", num_val(wx)),
                ("world_y", num_val(wy)),
                ("left", Value::Bool(b[0])),
                ("right", Value::Bool(b[1])),
                ("middle", Value::Bool(b[2])),
                ("wheel", num_val(wheel)),
                ("inside", Value::Bool(inside != 0)),
            ]))
        }
        "MOUSE_PRESSED" | "MOUSE_RELEASED" => {
            argc(name, a.len(), 0, 1)?;
            let b = button_arg(name, &a)?;
            let which = if name == "MOUSE_PRESSED" { 1 } else { 2 };
            Ok(Value::Bool(input_win(it).is_some_and(|w| unsafe { gfx_input_mouse(w, which, b) } != 0)))
        }
        "ON_KEY" | "ON_KEY_UP" => {
            argc(name, a.len(), 2, 2)?;
            let key = key_arg(name, &a[0])?;
            set_handler(it, name, if name == "ON_KEY" { H_KEY } else { H_KEY_UP }, key, &a[1])?;
            Ok(Value::Null)
        }
        "ON_CLICK" | "ON_MOUSE_MOVE" | "ON_TYPE" | "ON_RESIZE" | "ON_CLOSE" => {
            argc(name, a.len(), 1, 1)?;
            let kind = match name {
                "ON_CLICK" => H_CLICK,
                "ON_MOUSE_MOVE" => H_MOVE,
                "ON_TYPE" => H_TYPE,
                "ON_RESIZE" => H_RESIZE,
                _ => H_CLOSE,
            };
            set_handler(it, name, kind, 0, &a[0])?;
            Ok(Value::Null)
        }
        _ => err(format!("{} isn't available", name)),
    }
}

/// The Rust and C sides agree on the options struct.
#[cfg(test)]
pub fn check_layout() -> bool {
    // SAFETY: a plain query.
    unsafe { gfx_style_size() == std::mem::size_of::<GfxStyle>() }
}
