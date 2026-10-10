// The syntax tree that the parser builds and the interpreter walks.

use std::rc::Rc;

use crate::number::Number;

pub enum Expr {
    Num(Number),
    Text(Rc<str>),
    Bool(bool),
    Null,
    Var(Rc<str>),
    Builtin(&'static str),
    /// Table literal: each item has an optional name (`name: value`).
    Table(Vec<(Option<Rc<str>>, Expr)>),
    Field(Box<Expr>, Rc<str>),
    Index(Box<Expr>, Box<Expr>),
    /// base[start TO end STEP step]
    Slice(Box<Expr>, Box<Expr>, Box<Expr>, Option<Box<Expr>>),
    Call(Box<Expr>, Vec<Expr>),
    Unary(&'static str, Box<Expr>),
    Binary(&'static str, Box<Expr>, Box<Expr>),
    Func(Rc<FuncDecl>),
    Undefine(Box<Expr>),
    Insert(Vec<Expr>),
    Drop(Vec<Expr>),
}

pub struct Param {
    pub name: Rc<str>,
    pub shared: bool,
    pub rest: bool,
    pub default: Option<Expr>,
}

pub struct FuncDecl {
    pub name: Option<Rc<str>>,
    pub params: Vec<Param>,
    pub body: Rc<Vec<Stmt>>,
}

pub struct LoopVar {
    pub name: Rc<str>,
    pub shared: bool,
}

pub enum StmtKind {
    Expr(Expr),
    Assign(Expr, &'static str, Expr),
    Local(Rc<str>, Option<Expr>),
    If(Vec<(Expr, Vec<Stmt>)>, Option<Vec<Stmt>>),
    While(Expr, Vec<Stmt>),
    Repeat(Expr, Vec<Stmt>),
    ForIn(LoopVar, Option<LoopVar>, Expr, Vec<Stmt>),
    ForTo(Rc<str>, Expr, Expr, Option<Expr>, Vec<Stmt>),
    Break,
    Skip,
    Return(Option<Expr>),
    End,
    FuncDef(Rc<FuncDecl>),
    Try(Vec<Stmt>, Rc<str>, Vec<Stmt>),
    Goto(Rc<str>),
    Point(Rc<str>),
    Import(Rc<str>),
    Load(Rc<str>),
}

pub struct Stmt {
    pub kind: StmtKind,
    pub line: usize,
}

pub fn is_lvalue(e: &Expr) -> bool {
    matches!(e, Expr::Var(_) | Expr::Field(..) | Expr::Index(..))
}
