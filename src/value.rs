// Runtime values, tables, errors and formatting.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;

use crate::ast::FuncDecl;
use crate::number::Number;

/// A variable or table entry. `@` sharing makes two names point at the same slot.
pub type Slot = Rc<SlotData>;

/// What a slot holds, and whether it's a constant (made by CONST, which freezes
/// everything inside it too, so no name or table entry can change it).
pub struct SlotData {
    v: RefCell<Value>,
    constant: Cell<bool>,
}

impl SlotData {
    pub fn borrow(&self) -> std::cell::Ref<'_, Value> {
        self.v.borrow()
    }
    pub fn borrow_mut(&self) -> std::cell::RefMut<'_, Value> {
        self.v.borrow_mut()
    }
    pub fn is_const(&self) -> bool {
        self.constant.get()
    }
}
/// A file's global variables.
pub type Env = Rc<RefCell<OrdMap>>;

pub fn new_slot(v: Value) -> Slot {
    Rc::new(SlotData { v: RefCell::new(v), constant: Cell::new(false) })
}

/// A constant slot holding a frozen copy of `v`: its tables (and the tables inside
/// them) are copied into constant slots of their own, so nothing shared with an
/// ordinary variable gets frozen.
pub fn const_slot(v: Value) -> Slot {
    Rc::new(SlotData { v: RefCell::new(freeze(v)), constant: Cell::new(true) })
}

fn freeze(v: Value) -> Value {
    match v {
        Value::Table(rc, _) => {
            let mut named = OrdMap::new();
            for (k, s) in rc.named.iter() {
                named.insert(k.clone(), const_slot(s.borrow().clone()));
            }
            let t = Table { items: rc.items.iter().map(|s| const_slot(s.borrow().clone())).collect(), named, report: rc.report.clone() };
            Value::new_table(t)
        }
        o => o,
    }
}

/// Shared by every task's thread (see tasks.rs).
static NEXT_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

/// Every logical table copy gets its own identity, so `===` can tell a copy
/// from the same shared table even while copy-on-write keeps the data shared.
pub fn new_id() -> u64 {
    NEXT_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

pub struct Source {
    pub name: String,
    pub dir: PathBuf,
    /// The file's full path with links resolved (empty for Quick-try), for breakpoints.
    pub canon: PathBuf,
}

pub struct Function {
    pub decl: Rc<FuncDecl>,
    pub env: Env,
    pub src: Rc<Source>,
}

#[derive(Clone)]
pub enum Value {
    Num(Number),
    Text(Rc<str>),
    Bool(bool),
    Null,
    Table(Rc<Table>, u64),
    Func(Rc<Function>),
    Builtin(&'static str),
    /// A function running side by side with the program (START).
    Task(Rc<crate::tasks::TaskHandle>),
    /// Graphics (see graphics.rs): a picture, a window (by number), a font.
    Image(Rc<crate::graphics::ImageBox>),
    Window(usize),
    Font(Rc<crate::graphics::FontBox>),
}

impl Value {
    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Num(_) => "NUMBER",
            Value::Text(_) => "TEXT",
            Value::Bool(_) => "BOOL",
            Value::Null => "NULL",
            Value::Table(..) => "TABLE",
            Value::Func(_) | Value::Builtin(_) => "FUNCTION",
            Value::Task(_) => "TASK",
            Value::Image(_) => "IMAGE",
            Value::Window(_) => "WINDOW",
            Value::Font(_) => "FONT",
        }
    }
    pub fn new_table(t: Table) -> Value {
        Value::Table(Rc::new(t), new_id())
    }
    pub fn text(s: &str) -> Value {
        Value::Text(Rc::from(s))
    }
    pub fn int(i: i64) -> Value {
        Value::Num(Number::int(i))
    }
    pub fn is_func(&self) -> bool {
        matches!(self, Value::Func(_) | Value::Builtin(_))
    }
}

/// `=` copies. The data stays shared until one side changes it (copy-on-write).
pub fn copy_val(v: Value) -> Value {
    match v {
        Value::Table(rc, _) => Value::Table(rc, new_id()),
        o => o,
    }
}

/// A map that remembers insertion order.
#[derive(Default)]
pub struct OrdMap {
    entries: Vec<(Rc<str>, Slot)>,
    index: HashMap<Rc<str>, usize>,
}

impl OrdMap {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn get(&self, k: &str) -> Option<&Slot> {
        self.index.get(k).map(|&i| &self.entries[i].1)
    }
    pub fn insert(&mut self, k: Rc<str>, s: Slot) {
        if let Some(&i) = self.index.get(&*k) {
            self.entries[i].1 = s;
        } else {
            self.index.insert(k.clone(), self.entries.len());
            self.entries.push((k, s));
        }
    }
    pub fn remove(&mut self, k: &str) -> Option<Slot> {
        let i = self.index.remove(k)?;
        let (_, s) = self.entries.remove(i);
        for j in i..self.entries.len() {
            let key = self.entries[j].0.clone();
            self.index.insert(key, j);
        }
        Some(s)
    }
    pub fn iter(&self) -> std::slice::Iter<'_, (Rc<str>, Slot)> {
        self.entries.iter()
    }
    pub fn len(&self) -> usize {
        self.entries.len()
    }
    pub fn deep_clone(&self) -> OrdMap {
        OrdMap {
            entries: self.entries.iter().map(|(k, s)| (k.clone(), new_slot(s.borrow().clone()))).collect(),
            index: self.index.clone(),
        }
    }
}

pub struct Table {
    pub items: Vec<Slot>,
    pub named: OrdMap,
    /// Set on error tables, so PRINT(err) shows the full report.
    pub report: Option<Rc<str>>,
}

impl Table {
    pub fn new() -> Self {
        Table { items: Vec::new(), named: OrdMap::new(), report: None }
    }
    pub fn from_values(vals: Vec<Value>) -> Self {
        Table { items: vals.into_iter().map(new_slot).collect(), named: OrdMap::new(), report: None }
    }
    pub fn item(&self, i: usize) -> Value {
        self.items[i].borrow().clone()
    }
}

// A copy gets fresh slots, so changing the copy never touches the original.
impl Clone for Table {
    fn clone(&self) -> Self {
        Table {
            items: self.items.iter().map(|s| new_slot(s.borrow().clone())).collect(),
            named: self.named.deep_clone(),
            report: self.report.clone(),
        }
    }
}

// ---------- errors ----------

/// Variables recorded for a crash report: (name, value).
pub type VarList = Vec<(Rc<str>, Value)>;

pub struct ErrorInfo {
    pub msg: String,
    pub line: usize,
    pub file: String,
    /// The file's full path, for editors (empty when there's no file, as in Quick-try).
    pub path: String,
    /// Line and file have been filled in.
    pub captured: bool,
    /// Call chain and variables were recorded (false for syntax errors).
    pub traced: bool,
    /// Running functions, innermost first: (name, current line, locals).
    pub frames: Vec<(String, usize, VarList)>,
    /// The full path of the file each of those functions is in.
    pub frame_paths: Vec<String>,
    pub outside: VarList,
}

pub enum Fault {
    Error(Box<ErrorInfo>),
    /// END: stops the program; not an error, and TRY doesn't catch it.
    End,
    /// The task was stopped with CANCEL; it unwinds without running anything else.
    Cancelled,
    /// An error in a task that nothing was AWAITing: it stops the program, and TRY
    /// (in whatever the program happened to be doing) doesn't catch it.
    TaskError(Box<ErrorInfo>),
}

pub type R<T> = Result<T, Fault>;

pub fn fault(msg: impl Into<String>) -> Fault {
    Fault::Error(Box::new(ErrorInfo {
        msg: msg.into(),
        line: 0,
        file: String::new(),
        path: String::new(),
        captured: false,
        traced: false,
        frames: Vec::new(),
        frame_paths: Vec::new(),
        outside: Vec::new(),
    }))
}

pub fn err<T>(msg: impl Into<String>) -> R<T> {
    Err(fault(msg))
}

pub fn report(e: &ErrorInfo) -> String {
    let mut s = format!("ERROR in {}, line {}: {}", e.file, e.line, e.msg);
    if !e.traced {
        return s;
    }
    if !e.frames.is_empty() {
        s.push_str("\n\nCalled from:");
        let w = e.frames.iter().map(|f| f.0.chars().count() + 2).max().unwrap_or(0);
        for (name, line, _) in &e.frames {
            s.push_str(&format!("\n    {:<w$}  line {}", format!("{}()", name), line, w = w));
        }
    }
    s.push_str("\n\nVariables:");
    let mut rows: Vec<(String, &VarList)> =
        e.frames.iter().map(|f| (format!("{}:", f.0), &f.2)).collect();
    rows.push(("outside:".into(), &e.outside));
    let w = rows.iter().map(|r| r.0.chars().count()).max().unwrap_or(0) + 1;
    for (label, vars) in rows {
        let list = if vars.is_empty() {
            "(none)".to_string()
        } else {
            vars.iter().map(|(k, v)| format!("{} = {}", k, repr(v))).collect::<Vec<_>>().join(", ")
        };
        s.push_str(&format!("\n    {:<w$}{}", label, list, w = w));
    }
    s
}

// ---------- formatting ----------

pub fn fmt_num(x: &Number) -> String {
    x.to_string()
}

pub fn display(v: &Value) -> String {
    let mut s = String::new();
    write_val(&mut s, v, false, 0);
    s
}

pub fn repr(v: &Value) -> String {
    let mut s = String::new();
    write_val(&mut s, v, true, 0);
    s
}

pub fn quote_text(s: &str) -> String {
    if !s.contains('"') {
        return format!("\"{}\"", s);
    }
    let mut o = String::from("'");
    for ch in s.chars() {
        match ch {
            '\'' => o.push_str("\\'"),
            '\\' => o.push_str("\\\\"),
            '\n' => o.push_str("\\n"),
            '\t' => o.push_str("\\t"),
            c => o.push(c),
        }
    }
    o.push('\'');
    o
}

pub fn is_ident(s: &str) -> bool {
    let mut cs = s.chars();
    match cs.next() {
        Some(c) if c.is_ascii_lowercase() || c == '_' => {}
        _ => return false,
    }
    cs.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

fn write_val(out: &mut String, v: &Value, quote: bool, depth: usize) {
    match v {
        Value::Num(n) => out.push_str(&fmt_num(n)),
        Value::Text(s) => {
            if quote {
                out.push_str(&quote_text(s))
            } else {
                out.push_str(s)
            }
        }
        Value::Bool(b) => out.push_str(if *b { "TRUE" } else { "FALSE" }),
        Value::Null => out.push_str("NULL"),
        Value::Func(f) => match &f.decl.name {
            Some(n) => {
                out.push_str("FUNCTION ");
                out.push_str(n)
            }
            None => out.push_str("FUNCTION"),
        },
        Value::Builtin(b) => out.push_str(b),
        Value::Task(t) => {
            out.push_str("TASK ");
            out.push_str(&t.name)
        }
        Value::Image(i) => out.push_str(&format!("IMAGE {}x{}", i.width(), i.height())),
        Value::Window(id) => {
            out.push_str("WINDOW ");
            out.push_str(&crate::graphics::window_title(*id))
        }
        Value::Font(f) => out.push_str(if f.is_pixel() { "FONT pixel" } else { "FONT" }),
        Value::Table(t, _) => {
            if let Some(r) = &t.report {
                out.push_str(r);
                return;
            }
            if depth > 40 {
                out.push_str("[...]");
                return;
            }
            out.push('[');
            let mut first = true;
            for s in &t.items {
                if !first {
                    out.push_str(", ");
                }
                first = false;
                write_val(out, &s.borrow(), true, depth + 1);
            }
            for (k, s) in t.named.iter() {
                if !first {
                    out.push_str(", ");
                }
                first = false;
                if is_ident(k) {
                    out.push_str(k);
                } else {
                    out.push_str(&quote_text(k));
                }
                out.push_str(": ");
                write_val(out, &s.borrow(), true, depth + 1);
            }
            out.push(']');
        }
    }
}

/// Text that counts as a number: 25, -25, 2.5, 1e3, 1.5e-3, 0xFF.
/// Not numbers: "025", " 25 ", "1,000", ".5", "".
pub fn parse_num_text(s: &str) -> Option<Number> {
    let (neg, body) = match s.strip_prefix('-') {
        Some(r) => (true, r),
        None => (false, s),
    };
    if body.is_empty() {
        return None;
    }
    let v = if let Some(h) = body.strip_prefix("0x").or_else(|| body.strip_prefix("0X")) {
        if h.is_empty() || !h.chars().all(|c| c.is_ascii_hexdigit()) {
            return None;
        }
        Number::parse_literal(body).ok()?
    } else {
        let b = body.as_bytes();
        let mut i;
        if b[0] == b'0' {
            i = 1;
        } else if b[0].is_ascii_digit() {
            i = 0;
            while i < b.len() && b[i].is_ascii_digit() {
                i += 1;
            }
        } else {
            return None;
        }
        if i < b.len() && b[i] == b'.' {
            i += 1;
            let st = i;
            while i < b.len() && b[i].is_ascii_digit() {
                i += 1;
            }
            if i == st {
                return None;
            }
        }
        if i < b.len() && (b[i] == b'e' || b[i] == b'E') {
            i += 1;
            if i < b.len() && (b[i] == b'+' || b[i] == b'-') {
                i += 1;
            }
            let st = i;
            while i < b.len() && b[i].is_ascii_digit() {
                i += 1;
            }
            if i == st {
                return None;
            }
        }
        if i != b.len() {
            return None;
        }
        Number::parse_literal(body).ok()?
    };
    Some(if neg { v.neg() } else { v })
}
