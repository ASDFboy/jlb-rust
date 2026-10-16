// Turns source text into tokens.

use std::rc::Rc;

use crate::number::Number;

#[derive(Clone, Debug, PartialEq)]
pub enum Tok {
    Num(Number),
    Str(Rc<str>),
    Name(Rc<str>),
    Kw(&'static str),
    Op(&'static str),
    Newline,
    Eof,
}

pub struct Token {
    pub tok: Tok,
    pub line: usize,
    /// Where the token starts, counted in characters from the start of the file.
    pub pos: usize,
}

pub struct LexError {
    pub msg: String,
    pub line: usize,
    /// Where the problem starts, counted in characters from the start of the file.
    pub pos: usize,
}

pub const KEYWORDS: &[&str] = &[
    "IF", "ELIF", "ELSEIF", "ELSE", "WHILE", "REPEAT", "FOR", "IN", "TO", "STEP", "BREAK", "SKIP",
    "DEF", "RETURN", "LOCAL", "GLOBAL", "CONST", "START", "TRY", "FAIL", "GOTO", "POINT", "IMPORT", "LOAD", "END", "TRUE",
    "FALSE", "NULL", "UNDEFINE", "INSERT", "DROP",
];

pub const BUILTINS: &[&str] = &[
    "PRINT", "NUM", "STR", "ENG", "HEX", "LEN", "ERROR", "INPUT", "READ", "READ_LINES", "WRITE",
    "EXISTS", "TYPE", "RANDOM", "ROUND", "FLOOR", "CEIL", "TIME", "DELTA_TIME", "WAIT", "UPPER",
    "LOWER", "SPLIT", "JOIN", "FIND", "ABS", "MIN", "MAX", "TABLE_MIN", "TABLE_MAX", "SIN", "COS",
    "TAN", "CSC", "SEC", "COT", "ASIN", "ACOS", "ATAN", "RAD_SIN", "RAD_COS", "RAD_TAN", "RAD_CSC",
    "RAD_SEC", "RAD_COT", "RAD_ASIN", "RAD_ACOS", "RAD_ATAN", "PI", "SORT", "ALPHA_SORT", "KEYS",
    "AWAIT", "DONE", "CANCEL",
];

// Longest first, so "!===" wins over "!==" and so on.
const OPS: &[&str] = &[
    "!===", "===", "!==", "!&&", "!||", "...", "!^=", "!%=", "<==", ">==", "==", "!=", "<=", ">=",
    "&&", "||", "!&", "!|", "!~", "!^", "!%", "^=", "%=", "+=", "-=", "*=", "/=", "@=", "=", "<",
    ">", "&", "|", "~", "^", "%", "+", "-", "*", "/", "@", "!", "(", ")", "[", "]", "{", "}", ",",
    ".", ":", ";",
];

pub fn find_word(w: &str) -> Option<&'static str> {
    // FUNC is the older name for DEF, and still works.
    if w == "FUNC" {
        return Some("DEF");
    }
    KEYWORDS.iter().chain(BUILTINS.iter()).chain(crate::graphics::NAMES.iter()).find(|k| **k == w).copied()
}

fn snake(word: &str) -> String {
    let mut out = String::new();
    for (i, ch) in word.chars().enumerate() {
        if ch.is_ascii_uppercase() {
            if i > 0 && !out.ends_with('_') {
                out.push('_');
            }
            out.push(ch.to_ascii_lowercase());
        } else {
            out.push(ch);
        }
    }
    out
}

pub fn lex(src: &str) -> Result<Vec<Token>, LexError> {
    let c: Vec<char> = src.chars().collect();
    let n = c.len();
    let at = |i: usize| -> char { if i < n { c[i] } else { '\0' } };
    let mut i = 0;
    let mut line = 1;
    let mut toks: Vec<Token> = Vec::new();
    let mut stack: Vec<char> = Vec::new();
    let sp = std::cell::Cell::new(0usize);
    let fail = |msg: String, line: usize| -> Result<Vec<Token>, LexError> { Err(LexError { msg, line, pos: sp.get() }) };

    while i < n {
        let ch = c[i];
        sp.set(i);
        if ch == '\n' {
            let in_brackets = matches!(stack.last(), Some('(') | Some('['));
            let continues = matches!(toks.last(), Some(Token { tok: Tok::Op(o), .. })
                if !matches!(*o, ")" | "]" | "}" | ";"));
            let fresh = matches!(toks.last(), None | Some(Token { tok: Tok::Newline, .. }));
            if !in_brackets && !continues && !fresh {
                toks.push(Token { tok: Tok::Newline, line, pos: i });
            }
            line += 1;
            i += 1;
            continue;
        }
        if ch.is_whitespace() {
            i += 1;
            continue;
        }
        // Comments: >>> ... <<< and >> to end of line.
        if ch == '>' && at(i + 1) == '>' {
            if at(i + 2) == '>' {
                let start = line;
                i += 3;
                loop {
                    if i >= n {
                        return fail("this >>> comment is never closed with <<<".into(), start);
                    }
                    if c[i] == '<' && at(i + 1) == '<' && at(i + 2) == '<' {
                        i += 3;
                        break;
                    }
                    if c[i] == '\n' {
                        line += 1;
                    }
                    i += 1;
                }
            } else {
                while i < n && c[i] != '\n' {
                    i += 1;
                }
            }
            continue;
        }
        // Double quotes: exactly as written.
        if ch == '"' {
            let start = line;
            let mut j = i + 1;
            let mut s = String::new();
            while j < n && c[j] != '"' {
                if c[j] == '\n' {
                    line += 1;
                }
                s.push(c[j]);
                j += 1;
            }
            if j >= n {
                return fail("this text is never closed with \"".into(), start);
            }
            toks.push(Token { tok: Tok::Str(Rc::from(s.as_str())), line: start, pos: sp.get() });
            i = j + 1;
            continue;
        }
        // Single quotes: backslash codes.
        if ch == '\'' {
            let start = line;
            let mut j = i + 1;
            let mut s = String::new();
            while j < n && c[j] != '\'' {
                if c[j] == '\\' {
                    let e = at(j + 1);
                    match e {
                        '\'' => s.push('\''),
                        '"' => s.push('"'),
                        'n' => s.push('\n'),
                        't' => s.push('\t'),
                        '\\' => s.push('\\'),
                        _ => {
                            return fail(
                                format!("\\{} isn't a backslash code (use \\' \\\" \\n \\t or \\\\)", e),
                                line,
                            )
                        }
                    }
                    j += 2;
                    continue;
                }
                if c[j] == '\n' {
                    line += 1;
                }
                s.push(c[j]);
                j += 1;
            }
            if j >= n {
                return fail("this text is never closed with '".into(), start);
            }
            toks.push(Token { tok: Tok::Str(Rc::from(s.as_str())), line: start, pos: sp.get() });
            i = j + 1;
            continue;
        }
        // Numbers.
        if ch.is_ascii_digit() {
            let st = i;
            let value = if ch == '0' && (at(i + 1) == 'x' || at(i + 1) == 'X') {
                i += 2;
                let hs = i;
                while at(i).is_ascii_hexdigit() {
                    i += 1;
                }
                if i == hs {
                    return fail("0x must be followed by hex digits".into(), line);
                }
                let h: String = c[st..i].iter().collect();
                match Number::parse_literal(&h) {
                    Ok(v) => v,
                    Err(_) => return fail("that number is too big".into(), line),
                }
            } else {
                if ch == '0' && at(i + 1).is_ascii_digit() {
                    return fail("numbers can't start with 0 (like 025)".into(), line);
                }
                while at(i).is_ascii_digit() {
                    i += 1;
                }
                if at(i) == '.' && at(i + 1).is_ascii_digit() {
                    i += 1;
                    while at(i).is_ascii_digit() {
                        i += 1;
                    }
                }
                if (at(i) == 'e' || at(i) == 'E')
                    && (at(i + 1).is_ascii_digit()
                        || ((at(i + 1) == '+' || at(i + 1) == '-') && at(i + 2).is_ascii_digit()))
                {
                    i += 2;
                    while at(i).is_ascii_digit() {
                        i += 1;
                    }
                }
                let t: String = c[st..i].iter().collect();
                match Number::parse_literal(&t) {
                    Ok(v) => v,
                    Err(_) => return fail("that number is too big".into(), line),
                }
            };
            if at(i).is_ascii_alphanumeric() || at(i) == '_' {
                return fail("names can't start with a digit".into(), line);
            }
            toks.push(Token { tok: Tok::Num(value), line, pos: sp.get() });
            continue;
        }
        // Names, keywords and built-ins.
        if ch.is_ascii_alphabetic() || ch == '_' {
            let st = i;
            while at(i).is_ascii_alphanumeric() || at(i) == '_' {
                i += 1;
            }
            let word: String = c[st..i].iter().collect();
            let upper = word.chars().any(|x| x.is_ascii_uppercase());
            let lower = word.chars().any(|x| x.is_ascii_lowercase());
            let tok = if upper && !lower {
                match find_word(&word) {
                    Some(k) => Tok::Kw(k),
                    None => {
                        return fail(
                            format!("{} isn't a command .jlb knows (your own names must be lowercase)", word),
                            line,
                        )
                    }
                }
            } else if upper {
                let up = word.to_ascii_uppercase();
                if find_word(&up).is_some() {
                    return fail(format!("did you mean {}? Built-in commands are written in ALL CAPS", up), line);
                }
                return fail(
                    format!("\"{}\" should be \"{}\" (your own names are lowercase with underscores)", word, snake(&word)),
                    line,
                );
            } else {
                Tok::Name(Rc::from(word.as_str()))
            };
            toks.push(Token { tok, line, pos: sp.get() });
            continue;
        }
        // Operators and punctuation.
        let mut matched: Option<&'static str> = None;
        for op in OPS {
            let oc: Vec<char> = op.chars().collect();
            if i + oc.len() <= n && c[i..i + oc.len()] == oc[..] {
                matched = Some(op);
                break;
            }
        }
        match matched {
            Some("!=") => {
                return fail("!= isn't an operator in .jlb; use !== (or !=== for the strict version)".into(), line)
            }
            Some(op) => {
                match op {
                    "(" | "[" | "{" => stack.push(op.chars().next().unwrap()),
                    ")" | "]" | "}" => {
                        stack.pop();
                    }
                    _ => {}
                }
                toks.push(Token { tok: Tok::Op(op), line, pos: sp.get() });
                i += op.chars().count();
            }
            None => return fail(format!("unexpected character '{}'", ch), line),
        }
    }
    toks.push(Token { tok: Tok::Newline, line, pos: n });
    toks.push(Token { tok: Tok::Eof, line, pos: n });
    Ok(toks)
}
