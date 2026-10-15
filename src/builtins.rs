// Built-in commands (ALL CAPS).

use std::io::{BufRead, Write};
use std::rc::Rc;
use std::time::{Duration, Instant};

use crate::interp::Interp;
use crate::number::{Number, Rounding};
use crate::ops::{compare, match_eq};
use crate::value::*;

fn count(name: &str, a: &[Value], min: usize, max: usize) -> R<()> {
    if a.len() >= min && a.len() <= max {
        return Ok(());
    }
    let need = if min == max {
        format!("{}", min)
    } else if max == usize::MAX {
        format!("at least {}", min)
    } else {
        format!("{} to {}", min, max)
    };
    let plural = if min == 1 && max == 1 { "" } else { "s" };
    err(format!("{} takes {} input{}, but got {}", name, need, plural, a.len()))
}

/// An optional input: missing or NULL both mean "use the default".
fn opt(a: &[Value], i: usize) -> Option<&Value> {
    a.get(i).filter(|v| !matches!(v, Value::Null))
}

fn number(name: &str, v: &Value) -> R<Number> {
    match v {
        Value::Num(n) => Ok(n.clone()),
        Value::Null => Ok(Number::int(0)),
        Value::Text(s) => err(format!("{} needs a number, not the text {} (use NUM() to convert)", name, quote_text(s))),
        o => err(format!("{} needs a number, not a {}", name, o.type_name())),
    }
}

const MAX_DECIMALS: i64 = 1000;

fn decimals(v: Option<&Value>) -> R<Option<usize>> {
    match v {
        None => Ok(None),
        Some(v) => {
            let d = number("decimal places", v)?;
            if !d.is_integer() || !(0..=MAX_DECIMALS).contains(&d.to_i64_saturating()) {
                return err(format!("decimal places must be a whole number from 0 to {}", MAX_DECIMALS));
            }
            Ok(Some(d.to_i64_saturating() as usize))
        }
    }
}

fn depth_arg(v: Option<&Value>) -> R<usize> {
    match v {
        None => Ok(0),
        Some(v) => {
            let d = number("depth", v)?;
            if !d.is_integer() || d.is_neg() {
                return err("depth must be a whole number, 0 or more");
            }
            Ok(usize::try_from(d.to_i64_saturating()).unwrap_or(usize::MAX))
        }
    }
}

fn table_arg<'a>(name: &str, v: &'a Value) -> R<&'a Rc<Table>> {
    match v {
        Value::Table(t, _) => Ok(t),
        o => err(format!("{} needs a table, not a {}", name, o.type_name())),
    }
}

fn text(s: impl AsRef<str>) -> Value {
    Value::text(s.as_ref())
}

/// Degree trig results are tidied to 12 decimals, so COS(90) is exactly 0 and SIN(30) exactly 0.5.
fn snap(x: f64) -> R<Number> {
    Number::from_f64_rounded(x, 12)
}

fn num_val(n: Number) -> Value {
    Value::Num(n)
}

pub fn to_num(v: &Value) -> R<Value> {
    match v {
        Value::Num(n) => Ok(Value::Num(n.clone())),
        Value::Text(s) => match parse_num_text(s) {
            Some(n) => Ok(Value::Num(n)),
            None => err(format!("can't convert {} to a number", quote_text(s))),
        },
        Value::Bool(b) => Ok(Value::int(if *b { 1 } else { 0 })),
        Value::Null => Ok(Value::int(0)),
        o => err(format!("can't convert a {} to a number", o.type_name())),
    }
}

fn splitmix(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

fn random(it: &mut Interp, a: &[Value]) -> R<Value> {
    count("RANDOM", a, 2, 4)?;
    let mut lo = number("RANDOM", &a[0])?;
    let mut hi = number("RANDOM", &a[1])?;
    if lo.compare(&hi).is_gt() {
        std::mem::swap(&mut lo, &mut hi);
    }
    let d = match decimals(opt(a, 2))? {
        Some(d) => d,
        None => lo.decimal_places().max(hi.decimal_places()),
    };
    let scale = Number::int(10).pow(&Number::int(d as i64))?;
    let steps = hi.sub(&lo)?.mul(&scale)?.round(0, Rounding::Half)?;
    if steps.compare(&Number::int(9_000_000_000_000_000)).is_gt() {
        return err("that RANDOM range is too big for that many decimal places");
    }
    let steps = steps.to_i64_saturating() as u64;
    let r = match opt(a, 3) {
        Some(seed) => {
            let s = number("the RANDOM seed", seed)?.to_f64();
            let mut st = s.to_bits()
                ^ lo.to_f64().to_bits().rotate_left(13)
                ^ hi.to_f64().to_bits().rotate_left(29)
                ^ (d as u64).rotate_left(47);
            splitmix(&mut st);
            splitmix(&mut st)
        }
        None => splitmix(&mut it.rng),
    };
    let k = Number::int((r % (steps + 1)) as i64);
    Ok(num_val(lo.add(&k.div(&scale)?)?.round(d, Rounding::Half)?))
}

fn trig(name: &str, xn: &Number) -> R<Number> {
    let x = xn.to_f64();
    let (rad, base) = match name.strip_prefix("RAD_") {
        Some(b) => (true, b),
        None => (false, name),
    };
    let r = if rad { x } else { x.rem_euclid(360.0).to_radians() };
    let (s, c) = (r.sin(), r.cos());
    let tiny = |v: f64| v.abs() < 1e-12;
    let undefined = || err(format!("{} is undefined at {}", name, fmt_num(xn)));
    let out = match base {
        "SIN" => s,
        "COS" => c,
        "TAN" => {
            if tiny(c) {
                return undefined();
            }
            s / c
        }
        "CSC" => {
            if tiny(s) {
                return undefined();
            }
            1.0 / s
        }
        "SEC" => {
            if tiny(c) {
                return undefined();
            }
            1.0 / c
        }
        "COT" => {
            if tiny(s) {
                return undefined();
            }
            c / s
        }
        "ASIN" | "ACOS" => {
            if !(-1.0..=1.0).contains(&x) {
                return err(format!("{} only works on numbers from -1 to 1", name));
            }
            let a = if base == "ASIN" { x.asin() } else { x.acos() };
            if rad {
                a
            } else {
                a.to_degrees()
            }
        }
        _ => return err(format!("unknown trig function {}", name)),
    };
    // Degree results are tidied (COS(90) is exactly 0); radian results keep full precision.
    if rad {
        Number::from_f64(out)
    } else {
        snap(out)
    }
}

fn extreme(name: &str, vals: Vec<Value>, want_max: bool) -> R<Value> {
    let mut best: Option<Value> = None;
    for v in vals {
        if matches!(v, Value::Table(..)) {
            return err(format!("{} works on single values; use TABLE_{} for a table", name, name));
        }
        best = Some(match best {
            None => v,
            Some(b) => {
                let better = if want_max { compare(">", &v, &b)? } else { compare("<", &v, &b)? };
                if better {
                    v
                } else {
                    b
                }
            }
        });
    }
    Ok(best.unwrap_or(Value::Null))
}

fn collect_values(t: &Table, depth: usize, out: &mut Vec<Value>) {
    let all = t.items.iter().chain(t.named.iter().map(|(_, s)| s));
    for s in all {
        let v = s.borrow().clone();
        match &v {
            Value::Table(sub, _) => {
                if depth > 0 {
                    collect_values(sub, depth - 1, out);
                }
            }
            _ => out.push(v),
        }
    }
}

fn find_path(t: &Table, v: &Value, depth: usize) -> Option<Vec<Value>> {
    for (i, s) in t.items.iter().enumerate() {
        if match_eq(&s.borrow(), v) {
            return Some(vec![Value::int(i as i64 + 1)]);
        }
    }
    for (k, s) in t.named.iter() {
        if match_eq(&s.borrow(), v) {
            return Some(vec![Value::Text(k.clone())]);
        }
    }
    if depth > 0 {
        let subs = t
            .items
            .iter()
            .enumerate()
            .map(|(i, s)| (Value::int(i as i64 + 1), s))
            .chain(t.named.iter().map(|(k, s)| (Value::Text(k.clone()), s)));
        for (key, s) in subs {
            if let Value::Table(sub, _) = &*s.borrow() {
                if let Some(mut p) = find_path(sub, v, depth - 1) {
                    p.insert(0, key);
                    return Some(p);
                }
            }
        }
    }
    None
}

fn with_named(t: &Table, items: Vec<Value>) -> Value {
    let mut n = Table::from_values(items);
    n.named = t.named.deep_clone();
    Value::new_table(n)
}

pub fn call(it: &mut Interp, name: &'static str, a: Vec<Value>) -> R<Value> {
    match name {
        "PRINT" => {
            let s = a.iter().map(display).collect::<Vec<_>>().join(" ");
            let _ = writeln!(it.out, "{}", s);
            if it.ide {
                let _ = it.out.flush();
            }
            Ok(Value::Null)
        }
        "NUM" => {
            count(name, &a, 1, 1)?;
            to_num(&a[0])
        }
        "STR" => {
            count(name, &a, 1, 1)?;
            Ok(text(display(&a[0])))
        }
        "ENG" => {
            count(name, &a, 1, 1)?;
            Ok(text(number(name, &a[0])?.to_eng()?))
        }
        "HEX" => {
            count(name, &a, 1, 1)?;
            let x = number(name, &a[0])?;
            match x.to_hex() {
                Some(h) => Ok(text(h)),
                None => err(format!("HEX only works on whole numbers, not {}", fmt_num(&x))),
            }
        }
        "LEN" => {
            count(name, &a, 1, 1)?;
            let n = match &a[0] {
                Value::Table(t, _) => t.items.len(),
                Value::Text(s) => s.chars().count(),
                Value::Num(x) => fmt_num(x).chars().count(),
                Value::Null => 0,
                o => return err(format!("LEN doesn't work on a {}", o.type_name())),
            };
            Ok(Value::int(n as i64))
        }
        "ERROR" => {
            let msg = a.first().map(display).unwrap_or_else(|| "ERROR was called".into());
            err(msg)
        }
        "INPUT" => {
            count(name, &a, 0, 1)?;
            let prompt = a.first().map(display).unwrap_or_default();
            if it.repl {
                // Quick-try reads code from input, so ask the editor for the answer instead.
                let _ = it.out.flush();
                let _ = writeln!(it.out, "{}{{\"input\":{}}}", crate::ide::MARK, crate::ide::json_str(&prompt));
            } else {
                let _ = write!(it.out, "{}", prompt);
            }
            let _ = it.out.flush();
            let mut line = String::new();
            match &it.debug_input {
                // While debugging, standard input also carries the debugger's commands.
                Some(rx) => line = rx.recv().unwrap_or_default(),
                None => {
                    let _ = std::io::stdin().lock().read_line(&mut line);
                }
            }
            while line.ends_with('\n') || line.ends_with('\r') {
                line.pop();
            }
            Ok(text(line))
        }
        "READ" | "READ_LINES" => {
            count(name, &a, 1, 1)?;
            let p = display(&a[0]);
            let s = std::fs::read_to_string(&p).map_err(|e| fault(format!("couldn't read the file {}: {}", p, e)))?;
            if name == "READ" {
                Ok(text(s))
            } else {
                Ok(Value::new_table(Table::from_values(s.lines().map(text).collect())))
            }
        }
        "WRITE" => {
            count(name, &a, 2, 2)?;
            let p = display(&a[0]);
            std::fs::write(&p, display(&a[1])).map_err(|e| fault(format!("couldn't write the file {}: {}", p, e)))?;
            Ok(Value::Null)
        }
        "EXISTS" => {
            count(name, &a, 1, 1)?;
            Ok(Value::Bool(std::path::Path::new(&display(&a[0])).exists()))
        }
        "TYPE" => {
            count(name, &a, 1, 1)?;
            Ok(text(a[0].type_name()))
        }
        "RANDOM" => random(it, &a),
        "ROUND" | "FLOOR" | "CEIL" => {
            count(name, &a, 1, 2)?;
            let x = number(name, &a[0])?;
            let d = decimals(opt(&a, 1))?.unwrap_or(0);
            let mode = match name {
                "ROUND" => Rounding::Half,
                "FLOOR" => Rounding::Floor,
                _ => Rounding::Ceil,
            };
            Ok(num_val(x.round(d, mode)?))
        }
        "TIME" | "DELTA_TIME" => {
            count(name, &a, 0, 1)?;
            let t = if name == "TIME" {
                it.start.elapsed().as_secs_f64()
            } else {
                let now = Instant::now();
                let dt = now.duration_since(it.last_delta).as_secs_f64();
                it.last_delta = now;
                dt
            };
            Ok(num_val(match decimals(opt(&a, 0))? {
                Some(d) => Number::from_f64_rounded(t, d)?,
                None => Number::from_f64(t)?,
            }))
        }
        "WAIT" => {
            count(name, &a, 1, 1)?;
            let s = number(name, &a[0])?.to_f64();
            let _ = it.out.flush();
            if s > 0.0 {
                std::thread::sleep(Duration::from_secs_f64(s.min(1e9)));
            }
            Ok(Value::Null)
        }
        "UPPER" | "LOWER" => {
            count(name, &a, 1, 1)?;
            let s = display(&a[0]);
            Ok(text(if name == "UPPER" { s.to_uppercase() } else { s.to_lowercase() }))
        }
        "SPLIT" => {
            count(name, &a, 2, 2)?;
            let s = match &a[0] {
                Value::Text(s) => s.clone(),
                o => return err(format!("SPLIT needs text, not a {}", o.type_name())),
            };
            let sep = display(&a[1]);
            let parts: Vec<Value> = if sep.is_empty() {
                s.chars().map(|c| text(c.to_string())).collect()
            } else {
                s.split(sep.as_str()).map(text).collect()
            };
            Ok(Value::new_table(Table::from_values(parts)))
        }
        "JOIN" => {
            count(name, &a, 1, 2)?;
            let t = table_arg(name, &a[0])?;
            let sep = opt(&a, 1).map(display).unwrap_or_default();
            let parts: Vec<String> = t.items.iter().map(|s| display(&s.borrow())).collect();
            Ok(text(parts.join(&sep)))
        }
        "FIND" => {
            count(name, &a, 2, 3)?;
            match &a[0] {
                Value::Text(s) => {
                    let part = display(&a[1]);
                    Ok(match s.find(part.as_str()) {
                        Some(b) => Value::int(s[..b].chars().count() as i64 + 1),
                        None => Value::Null,
                    })
                }
                Value::Table(t, _) => {
                    let depth = depth_arg(opt(&a, 2))?;
                    Ok(match find_path(t, &a[1], depth) {
                        None => Value::Null,
                        Some(mut p) if p.len() == 1 => p.pop().unwrap(),
                        Some(p) => Value::new_table(Table::from_values(p)),
                    })
                }
                o => err(format!("FIND searches text or a table, not a {}", o.type_name())),
            }
        }
        "ABS" => {
            count(name, &a, 1, 1)?;
            Ok(num_val(number(name, &a[0])?.abs()))
        }
        "MIN" | "MAX" => {
            count(name, &a, 1, usize::MAX)?;
            extreme(name, a, name == "MAX")
        }
        "TABLE_MIN" | "TABLE_MAX" => {
            count(name, &a, 1, 2)?;
            let t = table_arg(name, &a[0])?;
            let mut vals = Vec::new();
            collect_values(t, depth_arg(opt(&a, 1))?, &mut vals);
            extreme(&name[6..], vals, name == "TABLE_MAX")
        }
        "SIN" | "COS" | "TAN" | "CSC" | "SEC" | "COT" | "ASIN" | "ACOS" | "RAD_SIN" | "RAD_COS" | "RAD_TAN"
        | "RAD_CSC" | "RAD_SEC" | "RAD_COT" | "RAD_ASIN" | "RAD_ACOS" => {
            count(name, &a, 1, 1)?;
            Ok(num_val(trig(name, &number(name, &a[0])?)?))
        }
        "ATAN" | "RAD_ATAN" => {
            count(name, &a, 1, 2)?;
            let y = number(name, &a[0])?.to_f64();
            let r = match a.get(1) {
                Some(x) => y.atan2(number(name, x)?.to_f64()),
                None => y.atan(),
            };
            Ok(num_val(if name == "ATAN" { snap(r.to_degrees())? } else { Number::from_f64(r)? }))
        }
        "PI" => {
            count(name, &a, 0, 1)?;
            Ok(num_val(match decimals(opt(&a, 0))? {
                Some(d) => Number::pi(d)?,
                None => Number::from_f64(std::f64::consts::PI)?,
            }))
        }
        "SORT" => {
            count(name, &a, 1, 1)?;
            let t = table_arg(name, &a[0])?;
            let vals: Vec<Value> = t.items.iter().map(|s| s.borrow().clone()).collect();
            let sorted = if vals.iter().all(|v| matches!(v, Value::Text(_))) {
                let mut keyed: Vec<(usize, Value)> = vals
                    .into_iter()
                    .map(|v| (if let Value::Text(s) = &v { s.chars().count() } else { 0 }, v))
                    .collect();
                keyed.sort_by_key(|k| k.0);
                keyed.into_iter().map(|k| k.1).collect()
            } else {
                let mut keyed = Vec::with_capacity(vals.len());
                for v in vals {
                    let k = match &v {
                        Value::Num(n) => n.clone(),
                        Value::Null => Number::int(0),
                        Value::Text(s) => match parse_num_text(s) {
                            Some(n) => n,
                            None => return err(format!("SORT can't compare the text {} with numbers", quote_text(s))),
                        },
                        o => return err(format!("SORT can't compare a {}", o.type_name())),
                    };
                    keyed.push((k, v));
                }
                keyed.sort_by(|x, y| x.0.compare(&y.0));
                keyed.into_iter().map(|k| k.1).collect()
            };
            Ok(with_named(t, sorted))
        }
        "ALPHA_SORT" => {
            count(name, &a, 1, 1)?;
            let t = table_arg(name, &a[0])?;
            let mut keyed: Vec<(String, Value)> =
                t.items.iter().map(|s| (display(&s.borrow()), s.borrow().clone())).collect();
            keyed.sort_by(|x, y| x.0.to_lowercase().cmp(&y.0.to_lowercase()).then(x.0.cmp(&y.0)));
            Ok(with_named(t, keyed.into_iter().map(|k| k.1).collect()))
        }
        "KEYS" => {
            count(name, &a, 1, 1)?;
            let t = table_arg(name, &a[0])?;
            let mut keys: Vec<Value> = (1..=t.items.len()).map(|i| Value::int(i as i64)).collect();
            keys.extend(t.named.iter().map(|(k, _)| Value::Text(k.clone())));
            Ok(Value::new_table(Table::from_values(keys)))
        }
        _ => err(format!("{} isn't available", name)),
    }
}
