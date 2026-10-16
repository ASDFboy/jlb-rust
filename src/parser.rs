// Builds the syntax tree from tokens.

use std::collections::HashMap;
use std::rc::Rc;

use crate::ast::*;
use crate::lexer::{Tok, Token, BUILTINS};

pub struct ParseError {
    pub msg: String,
    pub line: usize,
    /// Where the problem starts (characters from the start of the file), when known.
    pub pos: Option<usize>,
}

type PR<T> = Result<T, ParseError>;

const ASSIGN_OPS: &[&str] = &["=", "@=", "+=", "-=", "*=", "/=", "^=", "!^=", "%=", "!%="];

const LEVELS: &[&[&str]] = &[
    &["|", "||", "!|", "!||"],
    &["~", "!~"],
    &["&", "&&", "!&", "!&&"],
    &["==", "===", "!==", "!==="],
    &["<", ">", "<=", ">=", "<==", ">=="],
    &["+", "-"],
    &["*", "/", "%", "!%"],
];

fn describe(t: &Tok) -> String {
    match t {
        Tok::Num(_) => "a number".into(),
        Tok::Str(_) => "some text".into(),
        Tok::Name(n) => format!("`{}`", n),
        Tok::Kw(k) => k.to_string(),
        Tok::Op(o) => format!("`{}`", o),
        Tok::Newline => "the end of the line".into(),
        Tok::Eof => "the end of the file".into(),
    }
}

pub struct Parser {
    t: Vec<Token>,
    p: usize,
    loops: usize,
    funcs: usize,
}

impl Parser {
    pub fn new(t: Vec<Token>) -> Self {
        Parser { t, p: 0, loops: 0, funcs: 0 }
    }

    fn peek(&self) -> &Tok {
        &self.t[self.p].tok
    }
    fn peek_at(&self, k: usize) -> &Tok {
        &self.t[(self.p + k).min(self.t.len() - 1)].tok
    }
    fn line(&self) -> usize {
        self.t[self.p].line
    }
    fn advance(&mut self) -> Tok {
        let t = self.t[self.p].tok.clone();
        if self.p + 1 < self.t.len() {
            self.p += 1;
        }
        t
    }
    fn is_op(&self, o: &str) -> bool {
        matches!(self.peek(), Tok::Op(x) if *x == o)
    }
    fn is_kw(&self, k: &str) -> bool {
        matches!(self.peek(), Tok::Kw(x) if *x == k)
    }
    fn fail<T>(&self, msg: impl Into<String>) -> PR<T> {
        Err(ParseError { msg: msg.into(), line: self.line(), pos: Some(self.t[self.p].pos) })
    }
    fn expect_op(&mut self, o: &str) -> PR<()> {
        if self.is_op(o) {
            self.advance();
            Ok(())
        } else {
            self.fail(format!("expected `{}` but found {}", o, describe(self.peek())))
        }
    }
    fn expect_name(&mut self, what: &str) -> PR<Rc<str>> {
        match self.peek().clone() {
            Tok::Name(n) => {
                self.advance();
                Ok(n)
            }
            t => self.fail(format!("expected {} but found {}", what, describe(&t))),
        }
    }
    fn skip_seps(&mut self) {
        while matches!(self.peek(), Tok::Newline) || self.is_op(";") {
            self.advance();
        }
    }
    fn at_stmt_end(&self) -> bool {
        matches!(self.peek(), Tok::Newline | Tok::Eof) || self.is_op(";") || self.is_op("}")
    }

    pub fn program(&mut self) -> PR<Vec<Stmt>> {
        let mut v = Vec::new();
        self.skip_seps();
        while !matches!(self.peek(), Tok::Eof) {
            if self.is_op("}") {
                return self.fail("there's a } here without a matching {");
            }
            v.push(self.statement()?);
            if !self.at_stmt_end() {
                return self.fail("two statements on one line need a ; between them");
            }
            self.skip_seps();
        }
        check_points(&v)?;
        Ok(v)
    }

    fn block(&mut self) -> PR<Vec<Stmt>> {
        if !self.is_op("{") {
            if self.is_op("=") {
                return self.fail("= assigns a value; use == to compare");
            }
            return self.fail(format!("expected {{ to start a block but found {}", describe(self.peek())));
        }
        let open = self.line();
        let open_pos = self.t[self.p].pos;
        self.advance();
        self.skip_seps();
        let mut v = Vec::new();
        while !self.is_op("}") {
            if matches!(self.peek(), Tok::Eof) {
                return Err(ParseError { msg: "this block is never closed with }".into(), line: open, pos: Some(open_pos) });
            }
            v.push(self.statement()?);
            if !self.at_stmt_end() {
                return self.fail("two statements on one line need a ; between them");
            }
            self.skip_seps();
        }
        self.advance();
        Ok(v)
    }

    fn loop_body(&mut self) -> PR<Vec<Stmt>> {
        self.loops += 1;
        let b = self.block();
        self.loops -= 1;
        b
    }

    fn statement(&mut self) -> PR<Stmt> {
        let line = self.line();
        let kind = match self.peek().clone() {
            Tok::Kw(k) => match k {
                "IF" => self.if_stmt()?,
                "WHILE" => {
                    self.advance();
                    let c = self.expr()?;
                    StmtKind::While(c, self.loop_body()?)
                }
                "REPEAT" => {
                    self.advance();
                    let c = self.expr()?;
                    StmtKind::Repeat(c, self.loop_body()?)
                }
                "FOR" => self.for_stmt()?,
                // FUNC GLOBAL name(...) is the same as GLOBAL FUNC name(...).
                "FUNC" if matches!(self.peek_at(1), Tok::Kw("GLOBAL")) && matches!(self.peek_at(2), Tok::Name(_)) => {
                    self.advance();
                    self.advance();
                    let name = self.expect_name("a function name")?;
                    StmtKind::Global(Box::new(StmtKind::FuncDef(self.func_rest(Some(name))?)))
                }
                "GLOBAL" => {
                    self.advance();
                    let inner = if self.is_kw("FUNC") && matches!(self.peek_at(1), Tok::Name(_)) {
                        self.advance();
                        let name = self.expect_name("a function name")?;
                        StmtKind::FuncDef(self.func_rest(Some(name))?)
                    } else if self.is_kw("CONST") {
                        self.const_stmt()?
                    } else {
                        let name = match self.peek() {
                            Tok::Name(n) => n.clone(),
                            _ => return self.fail("GLOBAL goes before a name that gets a value (GLOBAL score = 0), FUNC or CONST"),
                        };
                        match self.expr_or_assign()? {
                            k @ StmtKind::Assign(Expr::Var(_), _, _) => k,
                            StmtKind::Assign(..) => {
                                return self.fail(format!(
                                    "GLOBAL goes before a name, not a table entry. {} is the outside one already when the function has no {} of its own",
                                    name, name
                                ))
                            }
                            _ => return self.fail(format!("GLOBAL {} needs a value: GLOBAL {} = ...", name, name)),
                        }
                    };
                    StmtKind::Global(Box::new(inner))
                }
                "FUNC" if matches!(self.peek_at(1), Tok::Name(_)) => {
                    self.advance();
                    let name = self.expect_name("a function name")?;
                    StmtKind::FuncDef(self.func_rest(Some(name))?)
                }
                "RETURN" => {
                    if self.funcs == 0 {
                        return self.fail("RETURN can only be used inside a function");
                    }
                    self.advance();
                    if self.at_stmt_end() {
                        StmtKind::Return(None)
                    } else {
                        StmtKind::Return(Some(self.expr()?))
                    }
                }
                "BREAK" | "SKIP" => {
                    if self.loops == 0 {
                        return self.fail(format!("{} can only be used inside a loop", k));
                    }
                    self.advance();
                    if k == "BREAK" {
                        StmtKind::Break
                    } else {
                        StmtKind::Skip
                    }
                }
                "END" => {
                    self.advance();
                    StmtKind::End
                }
                "LOCAL" => {
                    if self.funcs == 0 {
                        return self.fail("LOCAL can only be used inside a function");
                    }
                    self.advance();
                    let name = self.expect_name("a variable name after LOCAL")?;
                    let v = if self.is_op("=") {
                        self.advance();
                        Some(self.expr()?)
                    } else {
                        None
                    };
                    StmtKind::Local(name, v)
                }
                "CONST" => self.const_stmt()?,
                "TRY" => {
                    self.advance();
                    let body = self.block()?;
                    let save = self.p;
                    self.skip_seps();
                    if !self.is_kw("FAIL") {
                        self.p = save;
                        return self.fail("TRY needs a FAIL block after it");
                    }
                    self.advance();
                    let name = self.expect_name("a name for the error after FAIL")?;
                    let fail = self.block()?;
                    StmtKind::Try(body, name, fail)
                }
                "GOTO" | "POINT" => {
                    self.advance();
                    let name = self.expect_name(&format!("a point name after {}", k))?;
                    if k == "GOTO" {
                        StmtKind::Goto(name)
                    } else {
                        StmtKind::Point(name)
                    }
                }
                "IMPORT" | "LOAD" => {
                    self.advance();
                    match self.advance() {
                        Tok::Str(s) => {
                            if k == "IMPORT" {
                                StmtKind::Import(s)
                            } else {
                                StmtKind::Load(s)
                            }
                        }
                        t => return self.fail(format!("{} needs a file name in quotes, not {}", k, describe(&t))),
                    }
                }
                "ELIF" | "ELSEIF" | "ELSE" => return self.fail(format!("{} without an IF before it", k)),
                "FAIL" => return self.fail("FAIL without a TRY before it"),
                _ => self.expr_or_assign()?,
            },
            _ => self.expr_or_assign()?,
        };
        Ok(Stmt { kind, line })
    }

    fn const_stmt(&mut self) -> PR<StmtKind> {
        self.advance();
        let name = self.expect_name("a name after CONST")?;
        if !self.is_op("=") {
            return self.fail(&format!("CONST {} needs a value: CONST {} = ...", name, name));
        }
        self.advance();
        Ok(StmtKind::Const(name, self.expr()?))
    }

    fn expr_or_assign(&mut self) -> PR<StmtKind> {
        let e = self.expr()?;
        if let Tok::Op(o) = self.peek() {
            let o: &'static str = o;
            if ASSIGN_OPS.contains(&o) {
                if !is_lvalue(&e) {
                    return self.fail(format!("the left side of {} must be a variable, a table entry or a position", o));
                }
                self.advance();
                let rhs = self.expr()?;
                return Ok(StmtKind::Assign(e, o, rhs));
            }
        }
        Ok(StmtKind::Expr(e))
    }

    fn if_stmt(&mut self) -> PR<StmtKind> {
        self.advance();
        let c = self.expr()?;
        let b = self.block()?;
        let mut branches = vec![(c, b)];
        let mut otherwise = None;
        loop {
            let save = self.p;
            self.skip_seps();
            if self.is_kw("ELIF") || self.is_kw("ELSEIF") {
                self.advance();
                let c = self.expr()?;
                branches.push((c, self.block()?));
            } else if self.is_kw("ELSE") {
                self.advance();
                if self.is_kw("IF") {
                    self.advance();
                    let c = self.expr()?;
                    branches.push((c, self.block()?));
                } else {
                    otherwise = Some(self.block()?);
                    break;
                }
            } else {
                self.p = save;
                break;
            }
        }
        Ok(StmtKind::If(branches, otherwise))
    }

    fn loop_var(&mut self) -> PR<LoopVar> {
        let shared = if self.is_op("@") {
            self.advance();
            true
        } else {
            false
        };
        let name = self.expect_name("a loop variable name")?;
        Ok(LoopVar { name, shared })
    }

    fn for_stmt(&mut self) -> PR<StmtKind> {
        self.advance();
        let a = self.loop_var()?;
        let b = if self.is_op(",") {
            self.advance();
            Some(self.loop_var()?)
        } else {
            None
        };
        if !self.is_kw("IN") {
            return self.fail(format!("expected IN but found {}", describe(self.peek())));
        }
        self.advance();
        let iter = self.expr()?;
        if self.is_kw("TO") {
            if b.is_some() {
                return self.fail("a counting loop (FOR i IN 1 TO 10) takes just one variable");
            }
            self.advance();
            let end = self.expr()?;
            let step = if self.is_kw("STEP") {
                self.advance();
                Some(self.expr()?)
            } else {
                None
            };
            let body = self.loop_body()?;
            return Ok(StmtKind::ForTo(a.name, iter, end, step, body));
        }
        let body = self.loop_body()?;
        Ok(StmtKind::ForIn(a, b, iter, body))
    }

    fn func_rest(&mut self, name: Option<Rc<str>>) -> PR<Rc<FuncDecl>> {
        self.expect_op("(")?;
        let mut params: Vec<Param> = Vec::new();
        while !self.is_op(")") {
            let rest = if self.is_op("...") {
                self.advance();
                true
            } else {
                false
            };
            let shared = if self.is_op("@") {
                self.advance();
                true
            } else {
                false
            };
            let pname = self.expect_name("an input name")?;
            let default = if self.is_op("=") {
                self.advance();
                Some(self.expr()?)
            } else {
                None
            };
            if params.iter().any(|p| p.rest) {
                return self.fail("an input with ... must be the last one");
            }
            if params.iter().any(|p| p.name == pname) {
                return self.fail(format!("the input {} is listed twice", pname));
            }
            if rest && default.is_some() {
                return self.fail("an input with ... can't have a default");
            }
            if !rest && default.is_none() && params.iter().any(|p| p.default.is_some()) {
                return self.fail("required inputs must come before optional ones");
            }
            params.push(Param { name: pname, shared, rest, default });
            if self.is_op(",") {
                self.advance();
            } else if !self.is_op(")") {
                return self.fail(format!("expected , or ) but found {}", describe(self.peek())));
            }
        }
        self.advance();
        let (sl, sf) = (self.loops, self.funcs);
        self.loops = 0;
        self.funcs += 1;
        let body = self.block();
        self.loops = sl;
        self.funcs = sf;
        let body = body?;
        check_points(&body)?;
        Ok(Rc::new(FuncDecl { name, params, body: Rc::new(body) }))
    }

    // ---------- expressions ----------

    pub fn expr(&mut self) -> PR<Expr> {
        self.bin_level(0)
    }

    fn bin_level(&mut self, lvl: usize) -> PR<Expr> {
        if lvl == LEVELS.len() {
            return self.unary();
        }
        let mut left = self.bin_level(lvl + 1)?;
        loop {
            let op: &'static str = match self.peek() {
                Tok::Op(o) if LEVELS[lvl].contains(o) => o,
                _ => break,
            };
            self.advance();
            let right = self.bin_level(lvl + 1)?;
            left = Expr::Binary(op, Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn unary(&mut self) -> PR<Expr> {
        if self.is_op("-") {
            self.advance();
            let e = self.unary()?;
            if let Expr::Num(n) = e {
                return Ok(Expr::Num(n.neg()));
            }
            return Ok(Expr::Unary("-", Box::new(e)));
        }
        if self.is_op("!") {
            self.advance();
            return Ok(Expr::Unary("!", Box::new(self.unary()?)));
        }
        if self.is_kw("START") {
            self.advance();
            let call = self.postfix()?;
            if !matches!(call, Expr::Call(..)) {
                return self.fail("START needs a function call after it, like START walk(hero)");
            }
            return Ok(Expr::Start(Box::new(call)));
        }
        self.power()
    }

    fn power(&mut self) -> PR<Expr> {
        let base = self.postfix()?;
        let op: &'static str = match self.peek() {
            Tok::Op(o) if *o == "^" || *o == "!^" => o,
            _ => return Ok(base),
        };
        self.advance();
        let rhs = self.unary()?;
        Ok(Expr::Binary(op, Box::new(base), Box::new(rhs)))
    }

    fn args(&mut self) -> PR<Vec<Expr>> {
        self.expect_op("(")?;
        let mut v = Vec::new();
        while !self.is_op(")") {
            v.push(self.expr()?);
            if self.is_op(",") {
                self.advance();
            } else if !self.is_op(")") {
                return self.fail(format!("expected , or ) but found {}", describe(self.peek())));
            }
        }
        self.advance();
        Ok(v)
    }

    fn postfix(&mut self) -> PR<Expr> {
        let mut e = self.primary()?;
        loop {
            if self.is_op("(") {
                let a = self.args()?;
                e = Expr::Call(Box::new(e), a);
            } else if self.is_op(".") {
                self.advance();
                let n = self.expect_name("a name after .")?;
                e = Expr::Field(Box::new(e), n);
            } else if self.is_op("[") {
                self.advance();
                let a = self.expr()?;
                if self.is_kw("TO") {
                    self.advance();
                    let b = self.expr()?;
                    let step = if self.is_kw("STEP") {
                        self.advance();
                        Some(Box::new(self.expr()?))
                    } else {
                        None
                    };
                    self.expect_op("]")?;
                    e = Expr::Slice(Box::new(e), Box::new(a), Box::new(b), step);
                } else {
                    self.expect_op("]")?;
                    e = Expr::Index(Box::new(e), Box::new(a));
                }
            } else {
                break;
            }
        }
        Ok(e)
    }

    fn primary(&mut self) -> PR<Expr> {
        match self.peek().clone() {
            Tok::Num(n) => {
                self.advance();
                Ok(Expr::Num(n))
            }
            Tok::Str(s) => {
                self.advance();
                Ok(Expr::Text(s))
            }
            Tok::Name(n) => {
                self.advance();
                Ok(Expr::Var(n))
            }
            Tok::Op("(") => {
                self.advance();
                let e = self.expr()?;
                self.expect_op(")")?;
                Ok(e)
            }
            Tok::Op("[") => self.table_literal(),
            Tok::Kw(k) => match k {
                "TRUE" => {
                    self.advance();
                    Ok(Expr::Bool(true))
                }
                "FALSE" => {
                    self.advance();
                    Ok(Expr::Bool(false))
                }
                "NULL" => {
                    self.advance();
                    Ok(Expr::Null)
                }
                "FUNC" => {
                    self.advance();
                    Ok(Expr::Func(self.func_rest(None)?))
                }
                "UNDEFINE" => {
                    self.advance();
                    let a = self.args()?;
                    if a.len() != 1 || !is_lvalue(&a[0]) {
                        return self.fail("UNDEFINE needs one name, like UNDEFINE(score) or UNDEFINE(player.score)");
                    }
                    Ok(Expr::Undefine(Box::new(a.into_iter().next().unwrap())))
                }
                "INSERT" => {
                    self.advance();
                    Ok(Expr::Insert(self.args()?))
                }
                "DROP" => {
                    self.advance();
                    Ok(Expr::Drop(self.args()?))
                }
                _ if BUILTINS.contains(&k) => {
                    self.advance();
                    Ok(Expr::Builtin(k))
                }
                _ => self.fail(format!("{} can't be used here", k)),
            },
            t => self.fail(format!("expected a value but found {}", describe(&t))),
        }
    }

    fn table_literal(&mut self) -> PR<Expr> {
        self.advance();
        let mut items = Vec::new();
        while !self.is_op("]") {
            if matches!(self.peek(), Tok::Eof) {
                return self.fail("this table is never closed with ]");
            }
            let key = match (self.peek().clone(), self.peek_at(1).clone()) {
                (Tok::Name(n), Tok::Op(":")) | (Tok::Str(n), Tok::Op(":")) => {
                    self.advance();
                    self.advance();
                    Some(n)
                }
                _ => None,
            };
            let v = self.expr()?;
            items.push((key, v));
            if self.is_op(",") {
                self.advance();
            } else if !self.is_op("]") {
                return self.fail(format!("expected , or ] in the table but found {}", describe(self.peek())));
            }
        }
        self.advance();
        Ok(Expr::Table(items))
    }
}

/// POINT names must be unique within one function (or one file's top level).
fn check_points(body: &[Stmt]) -> PR<()> {
    fn walk(b: &[Stmt], seen: &mut HashMap<Rc<str>, usize>) -> PR<()> {
        for s in b {
            match &s.kind {
                StmtKind::Point(n) => {
                    if let Some(first) = seen.get(n) {
                        return Err(ParseError {
                            msg: format!("POINT {} is already used on line {}", n, first),
                            line: s.line,
                            pos: None,
                        });
                    }
                    seen.insert(n.clone(), s.line);
                }
                StmtKind::If(br, e) => {
                    for (_, x) in br {
                        walk(x, seen)?;
                    }
                    if let Some(x) = e {
                        walk(x, seen)?;
                    }
                }
                StmtKind::While(_, x) | StmtKind::Repeat(_, x) | StmtKind::ForIn(_, _, _, x) => walk(x, seen)?,
                StmtKind::ForTo(_, _, _, _, x) => walk(x, seen)?,
                StmtKind::Try(a, _, b) => {
                    walk(a, seen)?;
                    walk(b, seen)?;
                }
                _ => {}
            }
        }
        Ok(())
    }
    walk(body, &mut HashMap::new())
}
