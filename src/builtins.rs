// Built-in commands (ALL CAPS).

use std::io::{BufRead, Write};
use std::rc::Rc;
use std::time::{Duration, Instant};

use crate::interp::Interp;
use crate::ops::{self, compare, match_eq};
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

fn number(name: &str, v: &Value) -> R<f64> {
    match v {
        Value::Num(n) => Ok(*n),
        Value::Null => Ok(0.0),
        Value::Text(s) => err(format!("{} needs a number, not the text {} (use NUM() to convert)", name, quote_text(s))),
        o => err(format!("{} needs a number, not a {}", name, o.type_name())),
    }
}

fn decimals(v: Option<&Value>) -> R<Option<i32>> {
    match v {
        None => Ok(None),
        Some(v) => {
            let d = number("decimal places", v)?;
            if d.fract() != 0.0 || !(0.0..=15.0).contains(&d) {
                return err("decimal places must be a whole number from 0 to 15");
            }
            Ok(Some(d as i32))
        }
    }
}

fn depth_arg(v: Option<&Value>) -> R<usize> {
    match v {
        None => Ok(0),
        Some(v) => {
            let d = number("depth", v)?;
            if d.fract() != 0.0 || d < 0.0 {
                return err("depth must be a whole number, 0 or more");
            }
            Ok(d as usize)
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

fn snap(x: f64) -> f64 {
    let r = round_to(x, 12);
    if r == 0.0 {
        0.0
    } else {
        r
    }
}

pub fn to_num(v: &Value) -> R<Value> {
    match v {
        Value::Num(n) => Ok(Value::Num(*n)),
        Value::Text(s) => match parse_num_text(s) {
            Some(n) => Ok(Value::Num(n)),
            None => err(format!("can't convert {} to a number", quote_text(s))),
        },
        Value::Bool(b) => Ok(Value::Num(if *b { 1.0 } else { 0.0 })),
        Value::Null => Ok(Value::Num(0.0)),
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
    if lo > hi {
        std::mem::swap(&mut lo, &mut hi);
    }
    let d = match decimals(opt(a, 2))? {
        Some(d) => d,
        None => decimal_places(lo).max(decimal_places(hi)),
    };
    let scale = 10f64.powi(d);
    let steps = ((hi - lo) * scale).round();
    if steps > 9e15 {
        return err("that RANDOM range is too big for that many decimal places");
    }
    let r = match opt(a, 3) {
        Some(seed) => {
            let s = number("the RANDOM seed", seed)?;
            let mut st = s.to_bits() ^ lo.to_bits().rotate_left(13) ^ hi.to_bits().rotate_left(29) ^ (d as u64).rotate_left(47);
            splitmix(&mut st);
            splitmix(&mut st)
        }
        None => splitmix(&mut it.rng),
    };
    let k = (r % (steps as u64 + 1)) as f64;
    ops::num(round_to(lo + k / scale, d))
}

fn trig(name: &str, x: f64) -> R<f64> {
    let (rad, base) = match name.strip_prefix("RAD_") {
        Some(b) => (true, b),
        None => (false, name),
    };
    let r = if rad { x } else { x.rem_euclid(360.0).to_radians() };
    let (s, c) = (r.sin(), r.cos());
    let tiny = |v: f64| v.abs() < 1e-12;
    let undefined = || err(format!("{} is undefined at {}", name, fmt_num(x)));
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
    Ok(if rad { out } else { snap(out) })
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
            return Some(vec![Value::Num((i + 1) as f64)]);
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
            .map(|(i, s)| (Value::Num((i + 1) as f64), s))
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
            let x = number(name, &a[0])?;
            if x == 0.0 {
                return Ok(text("0e0"));
            }
            let mut e = (x.abs().log10().floor() as i32).div_euclid(3) * 3;
            let mut m = round_to(x / 10f64.powi(e), 10);
            if m.abs() >= 1000.0 {
                m = round_to(m / 1000.0, 10);
                e += 3;
            }
            Ok(text(format!("{}e{}", fmt_num(m), e)))
        }
        "HEX" => {
            count(name, &a, 1, 1)?;
            let x = number(name, &a[0])?;
            if x.fract() != 0.0 {
                return err(format!("HEX only works on whole numbers, not {}", fmt_num(x)));
            }
            if x.abs() >= 9.2e18 {
                return err("that number is too big for HEX");
            }
            let n = x as i64;
            Ok(text(format!("{}0x{:X}", if n < 0 { "-" } else { "" }, n.unsigned_abs())))
        }
        "LEN" => {
            count(name, &a, 1, 1)?;
            let n = match &a[0] {
                Value::Table(t, _) => t.items.len(),
                Value::Text(s) => s.chars().count(),
                Value::Num(x) => fmt_num(*x).chars().count(),
                Value::Null => 0,
                o => return err(format!("LEN doesn't work on a {}", o.type_name())),
            };
            Ok(Value::Num(n as f64))
        }
        "ERROR" => {
            let msg = a.first().map(display).unwrap_or_else(|| "ERROR was called".into());
            err(msg)
        }
        "INPUT" => {
            count(name, &a, 0, 1)?;
            if let Some(p) = a.first() {
                let _ = write!(it.out, "{}", display(p));
            }
            let _ = it.out.flush();
            let mut line = String::new();
            let _ = std::io::stdin().lock().read_line(&mut line);
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
            let s = 10f64.powi(d);
            let y = match name {
                "ROUND" => (x * s).round(),
                "FLOOR" => (x * s).floor(),
                _ => (x * s).ceil(),
            } / s;
            ops::num(round_to(y, d))
        }
        "TIME" => {
            count(name, &a, 0, 1)?;
            let t = it.start.elapsed().as_secs_f64();
            Ok(Value::Num(match decimals(opt(&a, 0))? {
                Some(d) => round_to(t, d),
                None => t,
            }))
        }
        "DELTA_TIME" => {
            count(name, &a, 0, 1)?;
            let now = Instant::now();
            let dt = now.duration_since(it.last_delta).as_secs_f64();
            it.last_delta = now;
            Ok(Value::Num(match decimals(opt(&a, 0))? {
                Some(d) => round_to(dt, d),
                None => dt,
            }))
        }
        "WAIT" => {
            count(name, &a, 1, 1)?;
            let s = number(name, &a[0])?;
            let _ = it.out.flush();
            if s > 0.0 {
                std::thread::sleep(Duration::from_secs_f64(s));
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
                        Some(b) => Value::Num((s[..b].chars().count() + 1) as f64),
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
            Ok(Value::Num(number(name, &a[0])?.abs()))
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
            Ok(Value::Num(trig(name, number(name, &a[0])?)?))
        }
        "ATAN" | "RAD_ATAN" => {
            count(name, &a, 1, 2)?;
            let y = number(name, &a[0])?;
            let r = match a.get(1) {
                Some(x) => y.atan2(number(name, x)?),
                None => y.atan(),
            };
            Ok(Value::Num(if name == "ATAN" { snap(r.to_degrees()) } else { r }))
        }
        "PI" => {
            count(name, &a, 0, 1)?;
            let p = std::f64::consts::PI;
            Ok(Value::Num(match decimals(opt(&a, 0))? {
                Some(d) => round_to(p, d),
                None => p,
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
                        Value::Num(n) => *n,
                        Value::Null => 0.0,
                        Value::Text(s) => match parse_num_text(s) {
                            Some(n) => n,
                            None => return err(format!("SORT can't compare the text {} with numbers", quote_text(s))),
                        },
                        o => return err(format!("SORT can't compare a {}", o.type_name())),
                    };
                    keyed.push((k, v));
                }
                keyed.sort_by(|x, y| x.0.total_cmp(&y.0));
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
            let mut keys: Vec<Value> = (1..=t.items.len()).map(|i| Value::Num(i as f64)).collect();
            keys.extend(t.named.iter().map(|(k, _)| Value::Text(k.clone())));
            Ok(Value::new_table(Table::from_values(keys)))
        }
        _ => err(format!("{} isn't available", name)),
    }
}
