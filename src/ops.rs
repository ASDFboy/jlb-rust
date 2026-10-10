// Operators: math, text and table tricks, comparisons and truth rules.

use std::rc::Rc;

use crate::number::Number;
use crate::value::*;

const MAX_LEN: usize = 100_000_000;

/// The truth rules shared by IF, WHILE and every logic operator.
pub fn truth(v: &Value) -> R<bool> {
    match v {
        Value::Bool(b) => Ok(*b),
        Value::Null => Ok(false),
        Value::Num(n) if n.is_int(1) => Ok(true),
        Value::Num(n) if n.is_zero() => Ok(false),
        Value::Num(n) => err(format!("{} can't be used as TRUE or FALSE (only 1 and 0 can)", fmt_num(n))),
        Value::Text(s) => match &**s {
            "1" | "TRUE" => Ok(true),
            "0" | "FALSE" => Ok(false),
            _ => err(format!("the text {} can't be used as TRUE or FALSE", quote_text(s))),
        },
        other => err(format!("a {} can't be used as TRUE or FALSE", other.type_name())),
    }
}

fn whole(x: &Number, what: &str) -> R<i64> {
    if !x.is_integer() {
        return err(format!("{} must be a whole number, not {}", what, fmt_num(x)));
    }
    Ok(x.to_i64_saturating())
}

/// Value as a number for math. NULL counts as 0.
pub fn math_num(v: &Value, op: &str) -> R<Number> {
    match v {
        Value::Num(n) => Ok(n.clone()),
        Value::Null => Ok(Number::int(0)),
        Value::Text(s) => err(format!(
            "can't use {} on text ({}); convert it with NUM() first",
            op,
            quote_text(s)
        )),
        other => err(format!("can't use {} on a {}", op, other.type_name())),
    }
}

/// Text used when joining with +: NULL adds nothing.
fn join_str(v: &Value) -> String {
    match v {
        Value::Null => String::new(),
        o => display(o),
    }
}

fn repeat<T: Clone>(items: &[T], n: i64) -> R<Vec<T>> {
    let count = usize::try_from(n.unsigned_abs()).unwrap_or(usize::MAX);
    if items.len().saturating_mul(count) > MAX_LEN {
        return err("that would be too big to store");
    }
    let mut base: Vec<T> = items.to_vec();
    if n < 0 {
        base.reverse();
    }
    let mut out = Vec::with_capacity(base.len() * count);
    for _ in 0..count {
        out.extend(base.iter().cloned());
    }
    Ok(out)
}

fn items_of(t: &Table) -> Vec<Value> {
    t.items.iter().map(|s| s.borrow().clone()).collect()
}

/// New table: the given numbered items plus copies of `t`'s named entries.
fn rebuild(t: &Table, items: Vec<Value>) -> Value {
    let mut n = Table::from_values(items);
    n.named = t.named.deep_clone();
    Value::new_table(n)
}

pub fn binop(op: &str, a: Value, b: Value) -> R<Value> {
    use Value::*;
    match op {
        "+" => match (&a, &b) {
            (Table(..), Table(..)) => Ok(Value::new_table(crate::value::Table::from_values(vec![copy_val(a.clone()), copy_val(b.clone())]))),
            (Table(t, _), _) => {
                let mut v = items_of(t);
                v.push(copy_val(b.clone()));
                Ok(rebuild(t, v))
            }
            (_, Table(t, _)) => {
                let mut v = vec![copy_val(a.clone())];
                v.extend(items_of(t));
                Ok(rebuild(t, v))
            }
            (Text(_), _) | (_, Text(_)) => Ok(Value::text(&(join_str(&a) + &join_str(&b)))),
            _ => Ok(Num(math_num(&a, "+")?.add(&math_num(&b, "+")?)?)),
        },
        "-" => match (&a, &b) {
            (Table(t, _), Num(n)) => {
                let k = whole(n, "the number of items to remove")?;
                let mut v = items_of(t);
                if k >= 0 {
                    let keep = v.len().saturating_sub(usize::try_from(k).unwrap_or(usize::MAX));
                    v.truncate(keep);
                } else {
                    v.push(Num(n.neg()));
                }
                Ok(rebuild(t, v))
            }
            (Table(t, _), _) => {
                let v: Vec<Value> = items_of(t).into_iter().filter(|x| !match_eq(x, &b)).collect();
                Ok(rebuild(t, v))
            }
            (Text(s), Num(n)) => {
                let k = whole(n, "the number of characters to trim")?;
                if k >= 0 {
                    let cs: Vec<char> = s.chars().collect();
                    let keep = cs.len().saturating_sub(usize::try_from(k).unwrap_or(usize::MAX));
                    Ok(Value::text(&cs[..keep].iter().collect::<String>()))
                } else {
                    Ok(Value::text(&format!("{}{}", s, fmt_num(&n.neg()))))
                }
            }
            (Text(s), Text(t)) => {
                if t.is_empty() {
                    Ok(a.clone())
                } else {
                    Ok(Value::text(&s.replace(&**t, "")))
                }
            }
            (Text(_), Null) => Ok(a.clone()),
            (Text(_), _) => err(format!("can't subtract a {} from text", b.type_name())),
            (Num(_) | Null, Text(_)) => err("a number minus text has no meaning"),
            _ => Ok(Num(math_num(&a, "-")?.sub(&math_num(&b, "-")?)?)),
        },
        "*" => match (&a, &b) {
            (Table(t, _), Num(n)) | (Num(n), Table(t, _)) => {
                let n = whole(n, "the number of repeats")?;
                Ok(rebuild(t, repeat(&items_of(t), n)?))
            }
            (Table(..), _) | (_, Table(..)) => err("a table can only be multiplied by a number"),
            (Text(s), Num(n)) | (Num(n), Text(s)) => {
                if !n.is_integer() {
                    return err("can't repeat text a fraction of times");
                }
                let cs: Vec<char> = s.chars().collect();
                Ok(Value::text(&repeat(&cs, n.to_i64_saturating())?.into_iter().collect::<String>()))
            }
            (Text(_), Null) | (Null, Text(_)) => Ok(Value::text("")),
            (Text(_), _) | (_, Text(_)) => err("text can only be repeated a number of times (the repeat count must be a number)"),
            _ => Ok(Num(math_num(&a, "*")?.mul(&math_num(&b, "*")?)?)),
        },
        "/" | "%" | "!%" | "^" | "!^" => {
            let (x, y) = (math_num(&a, op)?, math_num(&b, op)?);
            Ok(Num(match op {
                "/" => x.div(&y)?,
                "%" => x.rem(&y)?,
                "!%" => x.quot(&y)?,
                "^" => x.pow(&y)?,
                _ => x.root(&y)?,
            }))
        }
        _ => err(format!("unknown operator {}", op)),
    }
}

pub fn negate(v: Value) -> R<Value> {
    Ok(Value::Num(math_num(&v, "-")?.neg()))
}

fn cmp_num(v: &Value) -> R<Number> {
    match v {
        Value::Num(n) => Ok(n.clone()),
        Value::Null => Ok(Number::int(0)),
        Value::Text(s) => parse_num_text(s)
            .map_or_else(|| err(format!("can't compare the text {} with a number", quote_text(s))), Ok),
        other => err(format!("can't compare a {} with < or >", other.type_name())),
    }
}

pub fn compare(op: &str, a: &Value, b: &Value) -> R<bool> {
    if let (Value::Text(s), Value::Text(t)) = (a, b) {
        let (x, y) = (s.chars().count(), t.chars().count());
        return Ok(match op {
            "<" => x < y,
            ">" => x > y,
            "<=" => x <= y,
            ">=" => x >= y,
            "<==" => x < y || s == t,
            _ => x > y || s == t,
        });
    }
    let o = cmp_num(a)?.compare(&cmp_num(b)?);
    Ok(match op {
        "<" => o.is_lt(),
        ">" => o.is_gt(),
        "<=" | "<==" => o.is_le(),
        _ => o.is_ge(),
    })
}

fn tables_eq(a: &Table, b: &Table, depth: usize) -> bool {
    if depth > 40 {
        return true;
    }
    if a.items.len() != b.items.len() || a.named.len() != b.named.len() {
        return false;
    }
    for (x, y) in a.items.iter().zip(b.items.iter()) {
        if !loose_eq_d(&x.borrow(), &y.borrow(), depth + 1) {
            return false;
        }
    }
    for (k, s) in a.named.iter() {
        match b.named.get(k) {
            Some(o) => {
                if !loose_eq_d(&s.borrow(), &o.borrow(), depth + 1) {
                    return false;
                }
            }
            None => return false,
        }
    }
    true
}

pub fn loose_eq(a: &Value, b: &Value) -> bool {
    loose_eq_d(a, b, 0)
}

fn loose_eq_d(a: &Value, b: &Value, d: usize) -> bool {
    use Value::*;
    match (a, b) {
        (Num(x), Num(y)) => x == y,
        (Text(s), Text(t)) => s == t,
        (Text(s), Num(n)) | (Num(n), Text(s)) => parse_num_text(s).is_some_and(|v| v == *n),
        (Bool(x), Bool(y)) => x == y,
        (Bool(x), Num(n)) | (Num(n), Bool(x)) => n.is_int(if *x { 1 } else { 0 }),
        (Bool(x), Text(s)) | (Text(s), Bool(x)) => match &**s {
            "TRUE" | "1" => *x,
            "FALSE" | "0" => !*x,
            _ => false,
        },
        (Null, Null) => true,
        (Null, Num(n)) | (Num(n), Null) => n.is_zero(),
        (Null, Bool(x)) | (Bool(x), Null) => !*x,
        (Table(x, _), Table(y, _)) => Rc::ptr_eq(x, y) || tables_eq(x, y, d),
        (Func(f), Func(g)) => Rc::ptr_eq(f, g),
        (Builtin(x), Builtin(y)) => x == y,
        _ => false,
    }
}

pub fn strict_eq(a: &Value, b: &Value) -> bool {
    use Value::*;
    match (a, b) {
        (Num(x), Num(y)) => x == y,
        (Text(s), Text(t)) => s == t,
        (Bool(x), Bool(y)) => x == y,
        (Null, Null) => true,
        (Table(_, i), Table(_, j)) => i == j,
        (Func(f), Func(g)) => Rc::ptr_eq(f, g),
        (Builtin(x), Builtin(y)) => x == y,
        _ => false,
    }
}

/// FIND and `table - value`: === for everything except tables, which match by contents.
pub fn match_eq(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Table(..), Value::Table(..)) => loose_eq(a, b),
        _ => strict_eq(a, b),
    }
}
