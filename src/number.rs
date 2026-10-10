// Numbers.
//
// Whole numbers and fractions are exact, and grow to as many digits as they need:
// 0.1 + 0.2 is exactly 0.3, 1 / 3 * 3 is exactly 1, and 2 ^ 200 keeps every digit.
// Only answers that can't be written exactly (most roots and trig results) are
// approximate; those use 64-bit floating point.

use std::cmp::Ordering;
use std::fmt;
use std::rc::Rc;

use crate::value::{err, R};

const BASE: u64 = 1_000_000_000;
/// Exact numbers can use up to about this many digits.
pub const MAX_DIGITS: usize = 1_000_000;
/// A fraction that can't be written as a plain decimal (like 1/3) stays an exact
/// fraction until its bottom number passes this many digits. Then it's rounded
/// to this many significant decimals, so long calculations can't grow forever.
pub const FRACTION_DIGITS: usize = 50;
/// Decimals that go on forever show this many significant decimals.
pub const SHOW_DIGITS: usize = 16;

fn too_big<T>() -> R<T> {
    err("that number needs too many digits to store (the limit is a million)")
}

// ---------- unlimited whole numbers ----------

/// A whole number of any size, 0 or more. Stored in base 1,000,000,000, lowest part first.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct Nat(Vec<u32>);

impl Ord for Nat {
    fn cmp(&self, o: &Nat) -> Ordering {
        self.0.len().cmp(&o.0.len()).then_with(|| self.0.iter().rev().cmp(o.0.iter().rev()))
    }
}

impl PartialOrd for Nat {
    fn partial_cmp(&self, o: &Nat) -> Option<Ordering> {
        Some(self.cmp(o))
    }
}

fn gcd_u64(mut a: u64, mut b: u64) -> u64 {
    while b != 0 {
        (a, b) = (b, a % b);
    }
    a
}

impl Nat {
    pub fn zero() -> Nat {
        Nat(Vec::new())
    }
    pub fn one() -> Nat {
        Nat(vec![1])
    }
    pub fn from_u64(mut x: u64) -> Nat {
        let mut v = Vec::new();
        while x > 0 {
            v.push((x % BASE) as u32);
            x /= BASE;
        }
        Nat(v)
    }
    fn norm(mut self) -> Nat {
        while self.0.last() == Some(&0) {
            self.0.pop();
        }
        self
    }
    pub fn is_zero(&self) -> bool {
        self.0.is_empty()
    }
    pub fn is_one(&self) -> bool {
        self.0 == [1]
    }
    pub fn is_odd(&self) -> bool {
        self.0.first().is_some_and(|d| d % 2 == 1)
    }
    pub fn to_u64(&self) -> Option<u64> {
        if self.0.len() > 3 {
            return None;
        }
        let mut r: u128 = 0;
        for &d in self.0.iter().rev() {
            r = r * BASE as u128 + d as u128;
        }
        u64::try_from(r).ok()
    }
    /// How many decimal digits it has (0 has none).
    pub fn digits(&self) -> usize {
        match self.0.last() {
            None => 0,
            Some(&t) => (self.0.len() - 1) * 9 + t.ilog10() as usize + 1,
        }
    }
    /// Roughly log10 of the number (it must not be 0).
    pub fn approx_log10(&self) -> f64 {
        let n = self.0.len();
        if n == 1 {
            return (self.0[0] as f64).log10();
        }
        let top = self.0[n - 1] as f64 * BASE as f64 + self.0[n - 2] as f64;
        top.log10() + ((n - 2) * 9) as f64
    }

    pub fn add(&self, o: &Nat) -> Nat {
        let (a, b) = if self.0.len() >= o.0.len() { (self, o) } else { (o, self) };
        let mut r = Vec::with_capacity(a.0.len() + 1);
        let mut c = 0u64;
        for i in 0..a.0.len() {
            let s = a.0[i] as u64 + b.0.get(i).copied().unwrap_or(0) as u64 + c;
            r.push((s % BASE) as u32);
            c = s / BASE;
        }
        if c > 0 {
            r.push(c as u32);
        }
        Nat(r)
    }

    /// self - o, where self >= o.
    pub fn sub(&self, o: &Nat) -> Nat {
        let mut r = Vec::with_capacity(self.0.len());
        let mut borrow = 0i64;
        for i in 0..self.0.len() {
            let mut s = self.0[i] as i64 - o.0.get(i).copied().unwrap_or(0) as i64 - borrow;
            if s < 0 {
                s += BASE as i64;
                borrow = 1;
            } else {
                borrow = 0;
            }
            r.push(s as u32);
        }
        Nat(r).norm()
    }

    pub fn mul_small(&self, m: u32) -> Nat {
        if m == 0 || self.is_zero() {
            return Nat::zero();
        }
        let mut r = Vec::with_capacity(self.0.len() + 2);
        let mut c = 0u64;
        for &d in &self.0 {
            let t = d as u64 * m as u64 + c;
            r.push((t % BASE) as u32);
            c = t / BASE;
        }
        while c > 0 {
            r.push((c % BASE) as u32);
            c /= BASE;
        }
        Nat(r)
    }

    pub fn mul(&self, o: &Nat) -> Nat {
        if self.is_zero() || o.is_zero() {
            return Nat::zero();
        }
        if o.0.len() == 1 {
            return self.mul_small(o.0[0]);
        }
        if self.0.len() == 1 {
            return o.mul_small(self.0[0]);
        }
        let mut r = vec![0u64; self.0.len() + o.0.len()];
        for (i, &a) in self.0.iter().enumerate() {
            if a == 0 {
                continue;
            }
            let mut c = 0u64;
            for (j, &b) in o.0.iter().enumerate() {
                let t = r[i + j] + a as u64 * b as u64 + c;
                r[i + j] = t % BASE;
                c = t / BASE;
            }
            let mut k = i + o.0.len();
            while c > 0 {
                let t = r[k] + c;
                r[k] = t % BASE;
                c = t / BASE;
                k += 1;
            }
        }
        Nat(r.into_iter().map(|x| x as u32).collect()).norm()
    }

    pub fn divmod_small(&self, d: u32) -> (Nat, u32) {
        let mut q = vec![0u32; self.0.len()];
        let mut r = 0u64;
        for i in (0..self.0.len()).rev() {
            let cur = r * BASE + self.0[i] as u64;
            q[i] = (cur / d as u64) as u32;
            r = cur % d as u64;
        }
        (Nat(q).norm(), r as u32)
    }

    /// Quotient and remainder (Knuth's long division). `o` must not be 0.
    pub fn divmod(&self, o: &Nat) -> (Nat, Nat) {
        if self < o {
            return (Nat::zero(), self.clone());
        }
        if o.0.len() == 1 {
            let (q, r) = self.divmod_small(o.0[0]);
            return (q, Nat::from_u64(r as u64));
        }
        let n = o.0.len();
        let d = (BASE / (o.0[n - 1] as u64 + 1)) as u32;
        let v = o.mul_small(d).0;
        let mut u = self.mul_small(d).0;
        if u.len() == self.0.len() {
            u.push(0);
        }
        let m = u.len() - n - 1;
        let mut q = vec![0u32; m + 1];
        let (vt, vs) = (v[n - 1] as u64, v[n - 2] as u64);
        for j in (0..=m).rev() {
            let num = u[j + n] as u64 * BASE + u[j + n - 1] as u64;
            let mut qhat = num / vt;
            let mut rhat = num % vt;
            while qhat >= BASE || qhat * vs > rhat * BASE + u[j + n - 2] as u64 {
                qhat -= 1;
                rhat += vt;
                if rhat >= BASE {
                    break;
                }
            }
            let mut borrow = 0i64;
            let mut carry = 0u64;
            for i in 0..n {
                let p = qhat * v[i] as u64 + carry;
                carry = p / BASE;
                let mut t = u[i + j] as i64 - (p % BASE) as i64 - borrow;
                if t < 0 {
                    t += BASE as i64;
                    borrow = 1;
                } else {
                    borrow = 0;
                }
                u[i + j] = t as u32;
            }
            let t = u[j + n] as i64 - carry as i64 - borrow;
            if t < 0 {
                // qhat was one too big: add the divisor back.
                u[j + n] = (t + BASE as i64) as u32;
                qhat -= 1;
                let mut c = 0u64;
                for i in 0..n {
                    let s = u[i + j] as u64 + v[i] as u64 + c;
                    u[i + j] = (s % BASE) as u32;
                    c = s / BASE;
                }
                u[j + n] = ((u[j + n] as u64 + c) % BASE) as u32;
            } else {
                u[j + n] = t as u32;
            }
            q[j] = qhat as u32;
        }
        u.truncate(n);
        let (r, _) = Nat(u).norm().divmod_small(d);
        (Nat(q).norm(), r)
    }

    pub fn pow(&self, mut e: u64) -> Nat {
        let mut base = self.clone();
        let mut r = Nat::one();
        while e > 0 {
            if e & 1 == 1 {
                r = r.mul(&base);
            }
            e >>= 1;
            if e > 0 {
                base = base.mul(&base);
            }
        }
        r
    }

    pub fn pow10(k: usize) -> Nat {
        let mut v = vec![0u32; k / 9];
        v.push(10u32.pow((k % 9) as u32));
        Nat(v)
    }

    pub fn mul_pow10(&self, k: usize) -> Nat {
        if self.is_zero() {
            return Nat::zero();
        }
        let mut v = vec![0u32; k / 9];
        v.extend(self.mul_small(10u32.pow((k % 9) as u32)).0);
        Nat(v)
    }

    pub fn gcd(a: &Nat, b: &Nat) -> Nat {
        let (mut a, mut b) = (a.clone(), b.clone());
        while !b.is_zero() {
            if let (Some(x), Some(y)) = (a.to_u64(), b.to_u64()) {
                return Nat::from_u64(gcd_u64(x, y));
            }
            let r = a.divmod(&b).1;
            a = b;
            b = r;
        }
        a
    }

    /// Divides out every factor `p` (with `chunk` = p^chunk_pow to go faster).
    fn strip(&self, chunk: u32, chunk_pow: usize, p: u32) -> (Nat, usize) {
        let mut n = self.clone();
        let mut c = 0;
        for (d, k) in [(chunk, chunk_pow), (p, 1)] {
            loop {
                let (q, r) = n.divmod_small(d);
                if r != 0 {
                    break;
                }
                n = q;
                c += k;
            }
        }
        (n, c)
    }

    /// From decimal digits (all 0-9).
    pub fn parse_dec(s: &str) -> Nat {
        let b = s.as_bytes();
        let mut v = Vec::with_capacity(b.len() / 9 + 1);
        let mut end = b.len();
        while end > 0 {
            let st = end.saturating_sub(9);
            let mut x = 0u32;
            for &c in &b[st..end] {
                x = x * 10 + (c - b'0') as u32;
            }
            v.push(x);
            end = st;
        }
        Nat(v).norm()
    }

    /// From hex digits (all 0-9, a-f, A-F).
    pub fn parse_hex(s: &str) -> Nat {
        let mut n = Nat::zero();
        for c in s.chars() {
            n = n.mul_small(16).add(&Nat::from_u64(c.to_digit(16).unwrap_or(0) as u64));
        }
        n
    }

    pub fn to_hex(&self) -> String {
        if self.is_zero() {
            return "0".into();
        }
        let mut parts = Vec::new();
        let mut n = self.clone();
        while !n.is_zero() {
            let (q, r) = n.divmod_small(1 << 28);
            parts.push(r);
            n = q;
        }
        let mut s = format!("{:X}", parts.pop().unwrap());
        for p in parts.iter().rev() {
            s.push_str(&format!("{:07X}", p));
        }
        s
    }
}

impl fmt::Display for Nat {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.0.last() {
            None => write!(f, "0"),
            Some(t) => {
                write!(f, "{}", t)?;
                for d in self.0.iter().rev().skip(1) {
                    write!(f, "{:09}", d)?;
                }
                Ok(())
            }
        }
    }
}

/// Floor of the q-th root.
fn nth_root_floor(n: &Nat, q: u64) -> Nat {
    // Start a little above the real root, then Newton's method walks down to it.
    let l = n.approx_log10() / q as f64;
    let mut x = if l < 15.0 {
        Nat::from_u64((10f64.powf(l) * (1.0 + 1e-9)).ceil() as u64 + 1)
    } else {
        let e = l.floor();
        let m = 10f64.powf(l - e) * 1e15 * (1.0 + 1e-9);
        Nat::from_u64(m.ceil() as u64 + 1).mul_pow10(e as usize - 15)
    };
    let q32 = q as u32;
    loop {
        let t = n.divmod(&x.pow(q - 1)).0;
        let y = x.mul_small(q32 - 1).add(&t).divmod_small(q32).0;
        if y >= x {
            break;
        }
        x = y;
    }
    while !x.is_zero() && x.pow(q) > *n {
        x = x.sub(&Nat::one());
    }
    loop {
        let y = x.add(&Nat::one());
        if y.pow(q) <= *n {
            x = y;
        } else {
            break;
        }
    }
    x
}

/// The q-th root, if it's a whole number.
fn exact_root(n: &Nat, q: u64) -> Option<Nat> {
    if n.is_zero() || n.is_one() || q == 1 {
        return Some(n.clone());
    }
    // Any root of more than 2 bits' worth is below 2, so it can't be whole.
    if q > u32::MAX as u64 || q as f64 > n.approx_log10() * std::f64::consts::LOG2_10 + 1.0 {
        return None;
    }
    let r = nth_root_floor(n, q);
    if r.pow(q) == *n {
        Some(r)
    } else {
        None
    }
}

/// If n/den can be written as a plain decimal, how many decimals it needs.
fn plain_decimals(den: &Nat) -> Option<usize> {
    let (a, twos) = den.strip(1 << 29, 29, 2);
    let (b, fives) = a.strip(244_140_625, 12, 5);
    if b.is_one() {
        Some(twos.max(fives))
    } else {
        None
    }
}

/// Adds signed magnitudes.
fn signed_add(an: bool, a: &Nat, bn: bool, b: &Nat) -> (bool, Nat) {
    if an == bn {
        return (an, a.add(b));
    }
    match a.cmp(b) {
        Ordering::Less => (bn, b.sub(a)),
        _ => (an, a.sub(b)),
    }
}

// ---------- numbers ----------

/// An exact fraction in lowest terms. `den` is at least 1.
#[derive(Debug, PartialEq)]
pub struct Ratio {
    pub neg: bool,
    pub num: Nat,
    pub den: Nat,
    /// `den` is made only of 2s and 5s, so it's a plain decimal (like 0.25, not 1/3).
    pub plain: bool,
}

fn ratio(neg: bool, num: Nat, den: Nat, plain: bool) -> Rc<Ratio> {
    Rc::new(Ratio { neg, num, den, plain })
}

/// a / g, skipping the work when g is 1.
fn cut(a: &Nat, g: &Nat) -> Nat {
    if g.is_one() {
        a.clone()
    } else {
        a.divmod(g).0
    }
}

#[derive(Clone, Debug)]
pub enum Number {
    /// Exact whole numbers that fit in 64 bits (the fast, common case).
    Int(i64),
    /// Exact: bigger whole numbers, and fractions.
    Big(Rc<Ratio>),
    /// Approximate (always has a fractional part; whole results become exact).
    Float(f64),
}

#[derive(Clone, Copy, PartialEq)]
pub enum Rounding {
    Half,
    Floor,
    Ceil,
    Trunc,
}

use Number::*;

/// Builds an exact number from a fraction that's already in lowest terms.
/// `plain` says whether `den` is known to be made of 2s and 5s (None: work it out).
fn finish(neg: bool, num: Nat, den: Nat, plain: Option<bool>) -> R<Number> {
    if num.is_zero() {
        return Ok(Int(0));
    }
    if num.digits() > MAX_DIGITS || den.digits() > MAX_DIGITS {
        return too_big();
    }
    if den.is_one() {
        if let Some(u) = num.to_u64() {
            if u <= i64::MAX as u64 {
                return Ok(Int(if neg { -(u as i64) } else { u as i64 }));
            }
            if neg && u == 1 << 63 {
                return Ok(Int(i64::MIN));
            }
        }
        return Ok(Big(ratio(neg, num, den, true)));
    }
    let plain = plain.unwrap_or_else(|| plain_decimals(&den).is_some());
    let r = Ratio { neg, num, den, plain };
    if !plain && r.den.digits() > FRACTION_DIGITS {
        let p = first_decimal(&r) + FRACTION_DIGITS - 1;
        return round_ratio(&r, p, Rounding::Half);
    }
    Ok(Big(Rc::new(r)))
}

/// Builds an exact number from any fraction (den not 0).
fn make(neg: bool, num: Nat, den: Nat, plain: Option<bool>) -> R<Number> {
    if num.is_zero() {
        return Ok(Int(0));
    }
    let g = Nat::gcd(&num, &den);
    finish(neg, cut(&num, &g), cut(&den, &g), plain)
}

/// Position of the first non-zero digit after the decimal point (1 = tenths).
fn first_decimal(r: &Ratio) -> usize {
    let frac = r.num.divmod(&r.den).1;
    if frac.is_zero() {
        return 1;
    }
    let p = r.den.digits().saturating_sub(frac.digits());
    if frac.mul_pow10(p) < r.den {
        p + 1
    } else {
        p.max(1)
    }
}

/// Rounds to `p` decimal places.
fn round_ratio(r: &Ratio, p: usize, mode: Rounding) -> R<Number> {
    if p > MAX_DIGITS {
        return too_big();
    }
    let (mut q, rem) = r.num.mul_pow10(p).divmod(&r.den);
    let up = !rem.is_zero()
        && match mode {
            Rounding::Half => rem.mul_small(2) >= r.den,
            Rounding::Floor => r.neg,
            Rounding::Ceil => !r.neg,
            Rounding::Trunc => false,
        };
    if up {
        q = q.add(&Nat::one());
    }
    make(r.neg, q, Nat::pow10(p), Some(true))
}

/// The exact value of a 64-bit float, as (negative, top, bottom).
fn exact_of_f64(x: f64) -> (bool, Nat, Nat) {
    let bits = x.to_bits();
    let neg = bits >> 63 == 1;
    let ex = ((bits >> 52) & 0x7ff) as i64;
    let mut mant = bits & ((1u64 << 52) - 1);
    let mut e = if ex == 0 {
        -1074
    } else {
        mant |= 1 << 52;
        ex - 1075
    };
    if mant == 0 {
        return (false, Nat::zero(), Nat::one());
    }
    if e >= 0 {
        return (neg, Nat::from_u64(mant).mul(&Nat::from_u64(2).pow(e as u64)), Nat::one());
    }
    let tz = (mant.trailing_zeros() as i64).min(-e);
    mant >>= tz;
    e += tz;
    (neg, Nat::from_u64(mant), Nat::from_u64(2).pow((-e) as u64))
}

fn float_result(x: f64) -> R<Number> {
    if x.is_nan() {
        return err("that has no real answer");
    }
    Number::from_f64(x)
}

pub const PI_DIGITS: &str = "3.14159265358979323846264338327950288419716939937510582097494459230781640628620899862803482534211706798214808651";

impl Number {
    pub fn int(i: i64) -> Number {
        Int(i)
    }

    /// An approximate result. Whole results become exact.
    pub fn from_f64(x: f64) -> R<Number> {
        if !x.is_finite() {
            return err("that result is too big to store");
        }
        if x.fract() == 0.0 {
            if x.abs() < 9.2e18 {
                return Ok(Int(x as i64));
            }
            let (n, a, b) = exact_of_f64(x);
            return finish(n, a, b, Some(true));
        }
        Ok(Float(x))
    }

    /// The exact value of `x` rounded to `d` decimals (tidies trig results).
    pub fn from_f64_rounded(x: f64, d: usize) -> R<Number> {
        Number::from_f64(x)?.round(d, Rounding::Half)
    }

    /// Digits written in a program or in text: 25, 2.5, 1.5e-3, 0xFF (no sign).
    pub fn parse_literal(s: &str) -> R<Number> {
        if let Some(h) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
            return finish(false, Nat::parse_hex(h), Nat::one(), Some(true));
        }
        let (mant, exp) = match s.find(['e', 'E']) {
            Some(p) => (&s[..p], s[p + 1..].parse::<i64>().unwrap_or(i64::MAX)),
            None => (s, 0),
        };
        let (ip, fp) = mant.split_once('.').unwrap_or((mant, ""));
        let digits = format!("{}{}", ip, fp);
        let n = Nat::parse_dec(digits.trim_start_matches('0'));
        if n.is_zero() {
            return Ok(Int(0));
        }
        let scale = exp.saturating_sub(fp.len() as i64);
        if scale.unsigned_abs() > (MAX_DIGITS * 2) as u64 {
            return too_big();
        }
        if scale >= 0 {
            finish(false, n.mul_pow10(scale as usize), Nat::one(), Some(true))
        } else {
            make(false, n, Nat::pow10((-scale) as usize), Some(true))
        }
    }

    fn exact_parts(&self) -> (bool, Nat, Nat) {
        match self {
            Int(i) => (*i < 0, Nat::from_u64(i.unsigned_abs()), Nat::one()),
            Big(r) => (r.neg, r.num.clone(), r.den.clone()),
            Float(x) => exact_of_f64(*x),
        }
    }

    /// Whether this is a plain decimal (its bottom number is made of 2s and 5s).
    fn plain(&self) -> bool {
        match self {
            Big(r) => r.plain,
            _ => true,
        }
    }

    pub fn is_float(&self) -> bool {
        matches!(self, Float(_))
    }
    pub fn is_integer(&self) -> bool {
        match self {
            Int(_) => true,
            Big(r) => r.den.is_one(),
            Float(_) => false,
        }
    }
    pub fn is_zero(&self) -> bool {
        matches!(self, Int(0))
    }
    pub fn is_neg(&self) -> bool {
        match self {
            Int(i) => *i < 0,
            Big(r) => r.neg,
            Float(x) => *x < 0.0,
        }
    }
    pub fn is_int(&self, v: i64) -> bool {
        matches!(self, Int(i) if *i == v)
    }
    /// A whole number as i64, with huge ones pinned to the i64 limits.
    pub fn to_i64_saturating(&self) -> i64 {
        match self {
            Int(i) => *i,
            Big(r) if r.neg => i64::MIN,
            Big(_) => i64::MAX,
            Float(x) => *x as i64,
        }
    }

    pub fn to_f64(&self) -> f64 {
        match self {
            Int(i) => *i as f64,
            Float(x) => *x,
            Big(r) => {
                let v = match (r.num.to_u64(), r.den.to_u64()) {
                    (Some(a), Some(b)) if a < 1 << 53 && b < 1 << 53 => a as f64 / b as f64,
                    _ => {
                        let k = 20 - (r.num.digits() as i64 - r.den.digits() as i64);
                        let q = if k >= 0 {
                            r.num.mul_pow10(k as usize).divmod(&r.den).0
                        } else {
                            r.num.divmod(&r.den.mul_pow10((-k) as usize)).0
                        };
                        format!("{}e{}", q, -k).parse().unwrap_or(f64::INFINITY)
                    }
                };
                if r.neg {
                    -v
                } else {
                    v
                }
            }
        }
    }

    pub fn compare(&self, o: &Number) -> Ordering {
        match (self, o) {
            (Int(a), Int(b)) => a.cmp(b),
            (Float(a), Float(b)) => a.partial_cmp(b).unwrap_or(Ordering::Equal),
            _ => {
                let (an, a, ad) = self.exact_parts();
                let (bn, b, bd) = o.exact_parts();
                let (az, bz) = (a.is_zero(), b.is_zero());
                if az && bz {
                    return Ordering::Equal;
                }
                let sa = if az { 0 } else if an { -1 } else { 1 };
                let sb = if bz { 0 } else if bn { -1 } else { 1 };
                if sa != sb {
                    return sa.cmp(&sb);
                }
                let m = a.mul(&bd).cmp(&b.mul(&ad));
                if an {
                    m.reverse()
                } else {
                    m
                }
            }
        }
    }

    pub fn neg(&self) -> Number {
        match self {
            Int(i) => match i.checked_neg() {
                Some(v) => Int(v),
                None => Big(ratio(false, Nat::from_u64(1 << 63), Nat::one(), true)),
            },
            Float(x) => Float(-x),
            Big(r) => {
                if !r.neg && r.den.is_one() && r.num.to_u64() == Some(1 << 63) {
                    return Int(i64::MIN);
                }
                Big(ratio(!r.neg, r.num.clone(), r.den.clone(), r.plain))
            }
        }
    }

    pub fn abs(&self) -> Number {
        if self.is_neg() {
            self.neg()
        } else {
            self.clone()
        }
    }

    pub fn add(&self, o: &Number) -> R<Number> {
        match (self, o) {
            (Int(a), Int(b)) => {
                if let Some(c) = a.checked_add(*b) {
                    return Ok(Int(c));
                }
            }
            (Float(_), _) | (_, Float(_)) => return float_result(self.to_f64() + o.to_f64()),
            _ => {}
        }
        let plain = if self.plain() && o.plain() { Some(true) } else { None };
        let (an, a, ad) = self.exact_parts();
        let (bn, b, bd) = o.exact_parts();
        // Knuth's method: only small common factors ever need working out.
        let g = Nat::gcd(&ad, &bd);
        if g.is_one() {
            let (n, m) = signed_add(an, &a.mul(&bd), bn, &b.mul(&ad));
            return finish(n, m, ad.mul(&bd), plain);
        }
        let (n, t) = signed_add(an, &a.mul(&cut(&bd, &g)), bn, &b.mul(&cut(&ad, &g)));
        let g2 = Nat::gcd(&t, &g);
        finish(n, cut(&t, &g2), cut(&ad, &g).mul(&cut(&bd, &g2)), plain)
    }

    pub fn sub(&self, o: &Number) -> R<Number> {
        self.add(&o.neg())
    }

    pub fn mul(&self, o: &Number) -> R<Number> {
        match (self, o) {
            (Int(a), Int(b)) => {
                if let Some(c) = a.checked_mul(*b) {
                    return Ok(Int(c));
                }
            }
            (Float(_), _) | (_, Float(_)) => return float_result(self.to_f64() * o.to_f64()),
            _ => {}
        }
        let (an, a, ad) = self.exact_parts();
        let (bn, b, bd) = o.exact_parts();
        if a.digits() + b.digits() > MAX_DIGITS + 2 || ad.digits() + bd.digits() > MAX_DIGITS + 2 {
            return too_big();
        }
        // Cancel common factors first, so the result is already in lowest terms.
        let (g1, g2) = (Nat::gcd(&a, &bd), Nat::gcd(&b, &ad));
        let plain = if self.plain() && o.plain() { Some(true) } else { None };
        finish(an != bn, cut(&a, &g1).mul(&cut(&b, &g2)), cut(&ad, &g2).mul(&cut(&bd, &g1)), plain)
    }

    pub fn div(&self, o: &Number) -> R<Number> {
        if o.is_zero() {
            return err("can't divide by zero");
        }
        match (self, o) {
            (Int(a), Int(b)) => {
                if a % b == 0 {
                    if let Some(c) = a.checked_div(*b) {
                        return Ok(Int(c));
                    }
                }
            }
            (Float(_), _) | (_, Float(_)) => return float_result(self.to_f64() / o.to_f64()),
            _ => {}
        }
        let (an, a, ad) = self.exact_parts();
        let (bn, b, bd) = o.exact_parts();
        if a.digits() + bd.digits() > MAX_DIGITS + 2 || ad.digits() + b.digits() > MAX_DIGITS + 2 {
            return too_big();
        }
        let (g1, g2) = (Nat::gcd(&a, &b), Nat::gcd(&ad, &bd));
        finish(an != bn, cut(&a, &g1).mul(&cut(&bd, &g2)), cut(&ad, &g2).mul(&cut(&b, &g1)), None)
    }

    /// `!%`: how many whole times `o` fits (rounded toward zero).
    pub fn quot(&self, o: &Number) -> R<Number> {
        if o.is_zero() {
            return err("can't divide by zero");
        }
        match (self, o) {
            (Int(a), Int(b)) => {
                if let Some(c) = a.checked_div(*b) {
                    return Ok(Int(c));
                }
            }
            (Float(_), _) | (_, Float(_)) => return float_result((self.to_f64() / o.to_f64()).trunc()),
            _ => {}
        }
        self.div(o)?.round(0, Rounding::Trunc)
    }

    /// `%`: what's left over, with the sign of the left side.
    pub fn rem(&self, o: &Number) -> R<Number> {
        if o.is_zero() {
            return err("can't divide by zero");
        }
        match (self, o) {
            (Int(a), Int(b)) => return Ok(Int(a.checked_rem(*b).unwrap_or(0))),
            (Float(_), _) | (_, Float(_)) => return float_result(self.to_f64() % o.to_f64()),
            _ => {}
        }
        self.sub(&o.mul(&self.quot(o)?)?)
    }

    /// Rounds to `d` decimal places. The result is always exact.
    pub fn round(&self, d: usize, mode: Rounding) -> R<Number> {
        match self {
            Int(_) => Ok(self.clone()),
            Big(r) if r.den.is_one() => Ok(self.clone()),
            Big(r) => round_ratio(r, d, mode),
            Float(x) => {
                let (neg, num, den) = exact_of_f64(*x);
                round_ratio(&Ratio { neg, num, den, plain: true }, d, mode)
            }
        }
    }

    /// Roughly log10 of the size (not 0).
    fn approx_log10(&self) -> f64 {
        match self {
            Float(x) => x.abs().log10(),
            _ => {
                let (_, a, b) = self.exact_parts();
                a.approx_log10() - b.approx_log10()
            }
        }
    }

    /// `^`
    pub fn pow(&self, e: &Number) -> R<Number> {
        if self.is_float() || e.is_float() {
            let r = self.to_f64().powf(e.to_f64());
            if r.is_nan() {
                return err("that power has no real answer");
            }
            return float_result(r);
        }
        let (en, ep, eq) = e.exact_parts();
        if self.is_zero() {
            if ep.is_zero() {
                return Ok(Int(1));
            }
            if en {
                return err("can't divide by zero");
            }
            return Ok(Int(0));
        }
        let (bn, bp, bq) = self.exact_parts();
        // A fractional power is a root first: x ^ (p/q) = (q-th root of x) ^ p.
        let (rp, rq) = if eq.is_one() {
            (bp, bq)
        } else {
            if bn && !eq.is_odd() {
                return err("that power has no real answer");
            }
            let roots = eq.to_u64().and_then(|q| exact_root(&bp, q).zip(exact_root(&bq, q)));
            match roots {
                Some(r) => r,
                None => {
                    // Not exact: work it out approximately.
                    let a = self.abs();
                    let ev = e.to_f64();
                    let af = a.to_f64();
                    let m = if af.is_finite() && af > 1e-300 { af.powf(ev) } else { 10f64.powf(a.approx_log10() * ev) };
                    return float_result(if bn && ep.is_odd() { -m } else { m });
                }
            }
        };
        let neg = bn && ep.is_odd();
        if rp.is_one() && rq.is_one() {
            return Ok(Int(if neg { -1 } else { 1 }));
        }
        let p = match ep.to_u64() {
            Some(p) => p,
            None => return too_big(),
        };
        let size = |n: &Nat| if n.is_one() { 0.0 } else { n.approx_log10() * p as f64 };
        if size(&rp) > MAX_DIGITS as f64 || size(&rq) > MAX_DIGITS as f64 {
            return too_big();
        }
        let (num, den) = (rp.pow(p), rq.pow(p));
        if en {
            finish(neg, den, num, None)
        } else {
            finish(neg, num, den, None)
        }
    }

    /// `!^`: the n-th root.
    pub fn root(&self, n: &Number) -> R<Number> {
        if n.is_zero() {
            return err("can't take a 0th root");
        }
        let odd_whole = match n {
            Int(i) => i % 2 != 0,
            Big(r) => r.den.is_one() && r.num.is_odd(),
            Float(_) => false,
        };
        if self.is_neg() && !odd_whole {
            return err("an even root of a negative number has no real answer");
        }
        if self.is_float() || n.is_float() {
            let (x, k) = (self.to_f64(), n.to_f64());
            let r = if x < 0.0 { -(-x).powf(1.0 / k) } else { x.powf(1.0 / k) };
            return float_result(r);
        }
        self.pow(&Int(1).div(n)?)
    }

    /// Digits after the decimal point, as shown.
    pub fn decimal_places(&self) -> usize {
        let s = self.to_string();
        if let Some(epos) = s.find('e') {
            let (m, e) = s.split_at(epos);
            let exp: i64 = e[1..].parse().unwrap_or(0);
            let md = m.find('.').map(|p| (m.len() - p - 1) as i64).unwrap_or(0);
            return (md - exp).max(0) as usize;
        }
        s.find('.').map(|p| s.len() - p - 1).unwrap_or(0)
    }

    /// Hex text for whole numbers: 0xFF, -0x1A.
    pub fn to_hex(&self) -> Option<String> {
        if !self.is_integer() {
            return None;
        }
        let (neg, n, _) = self.exact_parts();
        Some(format!("{}0x{}", if neg { "-" } else { "" }, n.to_hex()))
    }

    /// Engineering notation: the exponent is a multiple of 3.
    pub fn to_eng(&self) -> R<String> {
        if self.is_zero() {
            return Ok("0e0".into());
        }
        if let Float(x) = self {
            let x = *x;
            let rt = |v: f64| (v * 1e10).round() / 1e10;
            let mut e = (x.abs().log10().floor() as i32).div_euclid(3) * 3;
            let mut m = rt(x / 10f64.powi(e));
            if m.abs() >= 1000.0 {
                m = rt(m / 1000.0);
                e += 3;
            }
            return Ok(format!("{}e{}", Number::from_f64(m)?, e));
        }
        let (_, a, b) = self.exact_parts();
        // floor(log10 |x|), exactly.
        let mut e = a.digits() as i64 - b.digits() as i64;
        let abs = self.abs();
        let ten_to = |k: i64| -> R<Number> { Int(10).pow(&Int(k)) };
        while abs.compare(&ten_to(e)?) == Ordering::Less {
            e -= 1;
        }
        while abs.compare(&ten_to(e + 1)?) != Ordering::Less {
            e += 1;
        }
        let e3 = e.div_euclid(3) * 3;
        let m = self.div(&ten_to(e3)?)?;
        Ok(format!("{}e{}", m, e3))
    }

    /// PI rounded to `d` decimals (up to 100).
    pub fn pi(d: usize) -> R<Number> {
        if d > 100 {
            return err("PI only knows 100 decimal places");
        }
        let digits = PI_DIGITS.replace('.', "");
        let n = Nat::parse_dec(&digits);
        let r = Ratio { neg: false, num: n, den: Nat::pow10(digits.len() - 1), plain: true };
        round_ratio(&r, d, Rounding::Half)
    }
}

impl PartialEq for Number {
    fn eq(&self, o: &Number) -> bool {
        self.compare(o) == Ordering::Equal
    }
}

fn fmt_float(x: f64) -> String {
    if !(1e-6..1e16).contains(&x.abs()) {
        return format!("{:e}", x);
    }
    format!("{}", x)
}

fn fmt_ratio(r: &Ratio) -> String {
    let sign = if r.neg { "-" } else { "" };
    let (mut ip, frac) = r.num.divmod(&r.den);
    if frac.is_zero() {
        return format!("{}{}", sign, ip);
    }
    let (p, q) = match if r.plain { plain_decimals(&r.den) } else { None } {
        // Ends: show every digit.
        Some(k) => (k, frac.mul_pow10(k).divmod(&r.den).0),
        // Goes on forever: show SHOW_DIGITS significant decimals, rounded.
        None => {
            let p = first_decimal(r) + SHOW_DIGITS - 1;
            let (mut q, rem) = frac.mul_pow10(p).divmod(&r.den);
            if rem.mul_small(2) >= r.den {
                q = q.add(&Nat::one());
            }
            if q == Nat::pow10(p) {
                ip = ip.add(&Nat::one());
                q = Nat::zero();
            }
            (p, q)
        }
    };
    let ds = format!("{:0>w$}", q.to_string(), w = p);
    let ds = ds.trim_end_matches('0');
    if ds.is_empty() {
        format!("{}{}", sign, ip)
    } else {
        format!("{}{}.{}", sign, ip, ds)
    }
}

impl fmt::Display for Number {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Int(i) => write!(f, "{}", i),
            Float(x) => write!(f, "{}", fmt_float(*x)),
            Big(r) => write!(f, "{}", fmt_ratio(r)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn n(s: &str) -> Number {
        Number::parse_literal(s).ok().unwrap()
    }

    #[test]
    fn decimals_are_exact() {
        assert_eq!(n("0.1").add(&n("0.2")).ok().unwrap().to_string(), "0.3");
        let third = Int(1).div(&Int(3)).ok().unwrap();
        assert_eq!(third.to_string(), "0.3333333333333333");
        assert_eq!(third.mul(&Int(3)).ok().unwrap().to_string(), "1");
        assert_eq!(Int(2).pow(&Int(100)).ok().unwrap().to_string(), "1267650600228229401496703205376");
    }

    #[test]
    fn roots() {
        let r = |x: &str, q: i64| Number::parse_literal(x).ok().unwrap().root(&Int(q)).ok().unwrap().to_string();
        assert_eq!(r("16", 2), "4");
        assert_eq!(r("0.25", 2), "0.5");
        assert_eq!(r("1267650600228229401496703205376", 100), "2");
        assert_eq!(r("2", 2), "1.4142135623730951");
    }
}
