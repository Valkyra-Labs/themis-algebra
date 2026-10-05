//! Univariate polynomials with exact rational coefficients, and their
//! real roots: counted exactly with Sturm sequences, found exactly when
//! rational or quadratic, and isolated and refined otherwise.

use num_bigint::BigInt;
use num_integer::Integer;
use num_rational::BigRational;
use num_traits::{One, Signed, ToPrimitive, Zero};
use std::fmt;

pub type Q = BigRational;

pub fn q(n: i64) -> Q {
    Q::from_integer(BigInt::from(n))
}

/// Coefficients from the constant term up; no trailing zeros.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Poly {
    c: Vec<Q>,
}

impl Poly {
    pub fn zero() -> Self {
        Poly { c: vec![] }
    }
    pub fn constant(v: Q) -> Self {
        Poly { c: vec![v] }.trimmed()
    }
    pub fn one() -> Self {
        Poly::constant(Q::one())
    }
    /// The polynomial `x`.
    pub fn x() -> Self {
        Poly {
            c: vec![Q::zero(), Q::one()],
        }
    }
    pub fn from_coeffs(c: Vec<Q>) -> Self {
        Poly { c }.trimmed()
    }
    fn trimmed(mut self) -> Self {
        while self.c.last().is_some_and(|v| v.is_zero()) {
            self.c.pop();
        }
        self
    }
    pub fn is_zero(&self) -> bool {
        self.c.is_empty()
    }
    /// Degree; the zero polynomial has none.
    pub fn degree(&self) -> Option<usize> {
        self.c.len().checked_sub(1)
    }
    pub fn coeffs(&self) -> &[Q] {
        &self.c
    }
    pub fn lead(&self) -> Q {
        self.c.last().cloned().unwrap_or_else(Q::zero)
    }
    pub fn add(&self, o: &Poly) -> Poly {
        let n = self.c.len().max(o.c.len());
        let c = (0..n)
            .map(|i| {
                self.c.get(i).cloned().unwrap_or_else(Q::zero)
                    + o.c.get(i).cloned().unwrap_or_else(Q::zero)
            })
            .collect();
        Poly { c }.trimmed()
    }
    pub fn neg(&self) -> Poly {
        Poly {
            c: self.c.iter().map(|v| -v).collect(),
        }
    }
    pub fn sub(&self, o: &Poly) -> Poly {
        self.add(&o.neg())
    }
    pub fn scale(&self, k: &Q) -> Poly {
        Poly {
            c: self.c.iter().map(|v| v * k).collect(),
        }
        .trimmed()
    }
    pub fn mul(&self, o: &Poly) -> Poly {
        if self.is_zero() || o.is_zero() {
            return Poly::zero();
        }
        let mut c = vec![Q::zero(); self.c.len() + o.c.len() - 1];
        for (i, a) in self.c.iter().enumerate() {
            for (j, b) in o.c.iter().enumerate() {
                c[i + j] += a * b;
            }
        }
        Poly { c }.trimmed()
    }
    pub fn pow(&self, n: u32) -> Poly {
        (0..n).fold(Poly::one(), |acc, _| acc.mul(self))
    }
    /// Quotient and remainder; `d` must not be zero.
    pub fn divrem(&self, d: &Poly) -> (Poly, Poly) {
        assert!(!d.is_zero(), "division by the zero polynomial");
        let dd = d.degree().expect("nonzero");
        let dl = d.lead();
        let mut r = self.clone();
        let mut quot = vec![Q::zero(); self.c.len().saturating_sub(dd).max(1)];
        while let Some(rd) = r.degree() {
            if rd < dd {
                break;
            }
            let k = r.lead() / &dl;
            let shift = rd - dd;
            quot[shift] = k.clone();
            let mut t = vec![Q::zero(); shift];
            t.extend(d.c.iter().map(|v| v * &k));
            r = r.sub(&Poly { c: t });
        }
        (Poly { c: quot }.trimmed(), r)
    }
    pub fn monic(&self) -> Poly {
        if self.is_zero() {
            return self.clone();
        }
        self.scale(&(Q::one() / self.lead()))
    }
    /// Monic greatest common divisor.
    pub fn gcd(&self, o: &Poly) -> Poly {
        let (mut a, mut b) = (self.clone(), o.clone());
        while !b.is_zero() {
            let (_, r) = a.divrem(&b);
            a = b;
            b = r;
        }
        a.monic()
    }
    pub fn derivative(&self) -> Poly {
        Poly {
            c: self
                .c
                .iter()
                .enumerate()
                .skip(1)
                .map(|(i, v)| v * q(i as i64))
                .collect(),
        }
        .trimmed()
    }
    /// The same roots, each once: `p / gcd(p, p')`, monic.
    pub fn squarefree(&self) -> Poly {
        if self.degree().unwrap_or(0) == 0 {
            return self.monic();
        }
        self.divrem(&self.gcd(&self.derivative())).0.monic()
    }
    pub fn eval(&self, x: &Q) -> Q {
        self.c.iter().rev().fold(Q::zero(), |acc, v| acc * x + v)
    }
    pub fn eval_f64(&self, x: f64) -> f64 {
        self.c
            .iter()
            .rev()
            .fold(0.0, |acc, v| acc * x + v.to_f64().unwrap_or(f64::NAN))
    }

    fn sturm(&self) -> Vec<Poly> {
        let mut seq = vec![self.clone(), self.derivative()];
        while !seq.last().expect("two").is_zero() {
            let n = seq.len();
            let (_, r) = seq[n - 2].divrem(&seq[n - 1]);
            seq.push(r.neg());
        }
        seq.pop();
        seq
    }

    fn sign_changes(seq: &[Poly], x: &Q) -> usize {
        let signs: Vec<i32> = seq
            .iter()
            .map(|p| p.eval(x))
            .filter(|v| !v.is_zero())
            .map(|v| if v.is_positive() { 1 } else { -1 })
            .collect();
        signs.windows(2).filter(|w| w[0] != w[1]).count()
    }

    /// A bound B with every real root in (-B, B) (Cauchy).
    fn root_bound(&self) -> Q {
        let lead = self.lead().abs();
        let m = self
            .c
            .iter()
            .map(|v| v.abs() / &lead)
            .fold(Q::zero(), |a, b| if b > a { b } else { a });
        m + q(1)
    }

    /// Number of distinct real roots in the half-open interval (a, b].
    pub fn count_roots_in(&self, a: &Q, b: &Q) -> usize {
        if self.degree().unwrap_or(0) == 0 {
            return 0;
        }
        let s = self.squarefree().sturm();
        Self::sign_changes(&s, a) - Self::sign_changes(&s, b)
    }

    /// Number of distinct real roots.
    pub fn count_real_roots(&self) -> usize {
        if self.degree().unwrap_or(0) == 0 {
            return 0;
        }
        let b = self.root_bound();
        self.count_roots_in(&-b.clone(), &b)
    }

    /// Distinct real roots, exact when rational or quadratic surds.
    pub fn real_roots(&self) -> Vec<Root> {
        if self.degree().unwrap_or(0) == 0 {
            return vec![];
        }
        let mut rest = self.squarefree();
        let mut roots: Vec<Root> = Vec::new();
        for r in rational_roots(&rest) {
            rest = rest
                .divrem(&Poly::from_coeffs(vec![-r.clone(), Q::one()]))
                .0;
            roots.push(Root::Rational(r));
        }
        match rest.degree() {
            Some(2) => {
                let (c, b, a) = (&rest.c[0], &rest.c[1], &rest.c[2]);
                let disc = b * b - q(4) * a * c;
                if disc.is_positive() {
                    for sign in [-1, 1] {
                        roots.push(Root::Quadratic {
                            p: -b / (q(2) * a),
                            k: q(sign) / (q(2) * a),
                            d: disc.clone(),
                        });
                    }
                }
            }
            Some(d) if d > 2 => {
                for x in isolate_and_refine(&rest) {
                    roots.push(Root::Approx(x));
                }
            }
            _ => {}
        }
        roots.sort_by(|a, b| a.value().total_cmp(&b.value()));
        roots
    }
}

/// The work [`rational_roots`] may spend on candidates: their number
/// (pairs of divisors) times the degree, the number of steps of each
/// evaluation. A number up to 10^12 can have thousands of divisors, so a
/// short line could otherwise ask for tens of millions of evaluations.
const MAX_ROOT_CANDIDATE_STEPS: usize = 2_000_000;

/// `q^d · f(p/q)` for the integer coefficients of `f` (constant term
/// first, degree d): zero exactly when `p/q` is a root, and computed
/// without fractions, so with no gcd at each step.
fn eval_scaled(ints: &[BigInt], p: &BigInt, q: &BigInt) -> BigInt {
    let mut terms = ints.iter().rev();
    let mut acc = terms.next().cloned().unwrap_or_else(BigInt::zero);
    let mut q_pow = BigInt::one();
    for a in terms {
        q_pow *= q;
        acc = acc * p + a * &q_pow;
    }
    acc
}

/// Rational roots of a polynomial with rational coefficients (rational
/// root theorem on the integer-cleared polynomial). Divisor enumeration
/// and the number of candidates are capped; school exercises stay far
/// below both caps. Past a cap, the roots not found here are left to
/// [`isolate_and_refine`].
fn rational_roots(p: &Poly) -> Vec<Q> {
    let Some(deg) = p.degree() else { return vec![] };
    if deg == 0 {
        return vec![];
    }
    let lcm = p.c.iter().fold(BigInt::one(), |acc, v| acc.lcm(v.denom()));
    let ints: Vec<BigInt> =
        p.c.iter()
            .map(|v| (v * Q::from_integer(lcm.clone())).to_integer())
            .collect();
    let mut out = Vec::new();
    // Zero is a root when the constant term vanishes (at most once: the
    // polynomial is squarefree).
    if ints[0].is_zero() {
        out.push(Q::zero());
    }
    let a0 = ints[ints.iter().position(|v| !v.is_zero()).expect("nonzero")].abs();
    let an = ints[deg].abs();
    let (Some(ps), Some(qs)) = (divisors(&a0), divisors(&an)) else {
        return out;
    };
    if ps.len().saturating_mul(qs.len()).saturating_mul(deg) > MAX_ROOT_CANDIDATE_STEPS {
        return out;
    }
    for pp in &ps {
        // Each fraction once, in lowest terms.
        for qq in qs.iter().filter(|qq| pp.gcd(qq).is_one()) {
            for sign in [1, -1] {
                let num = BigInt::from(sign) * pp;
                if eval_scaled(&ints, &num, qq).is_zero() {
                    out.push(Q::new(num, qq.clone()));
                }
            }
        }
    }
    out
}

fn divisors(n: &BigInt) -> Option<Vec<BigInt>> {
    let n = n.to_u64()?;
    if n == 0 || n > 1_000_000_000_000 {
        return None;
    }
    let mut out = Vec::new();
    let mut i = 1u64;
    while i * i <= n {
        if n % i == 0 {
            out.push(BigInt::from(i));
            if i * i != n {
                out.push(BigInt::from(n / i));
            }
        }
        i += 1;
        if i > 2_000_000 {
            return None;
        }
    }
    Some(out)
}

/// Real roots of a squarefree polynomial as f64: isolated by Sturm
/// bisection on exact rationals, then refined by bisection in f64.
fn isolate_and_refine(p: &Poly) -> Vec<f64> {
    let b = p.root_bound();
    let mut stack = vec![(-b.clone(), b)];
    let mut intervals = Vec::new();
    let s = p.sturm();
    while let Some((lo, hi)) = stack.pop() {
        let n = Poly::sign_changes(&s, &lo) - Poly::sign_changes(&s, &hi);
        if n == 0 {
            continue;
        }
        if n == 1 {
            intervals.push((lo, hi));
            continue;
        }
        let mid = (&lo + &hi) / q(2);
        stack.push((lo, mid.clone()));
        stack.push((mid, hi));
    }
    intervals
        .into_iter()
        .map(|(lo, hi)| {
            let (mut a, mut b) = (lo.to_f64().unwrap_or(0.0), hi.to_f64().unwrap_or(0.0));
            let fa = p.eval_f64(a);
            for _ in 0..200 {
                let m = 0.5 * (a + b);
                if p.eval_f64(m).signum() == fa.signum() {
                    a = m;
                } else {
                    b = m;
                }
            }
            0.5 * (a + b)
        })
        .collect()
}

/// A real root: rational, a quadratic surd `p + k·√d`, or an
/// approximation (irrational roots of degree three and up).
#[derive(Clone, Debug, PartialEq)]
pub enum Root {
    Rational(Q),
    Quadratic { p: Q, k: Q, d: Q },
    Approx(f64),
}

impl Root {
    pub fn value(&self) -> f64 {
        match self {
            Root::Rational(r) => r.to_f64().unwrap_or(f64::NAN),
            Root::Quadratic { p, k, d } => {
                p.to_f64().unwrap_or(0.0)
                    + k.to_f64().unwrap_or(0.0) * d.to_f64().unwrap_or(0.0).sqrt()
            }
            Root::Approx(x) => *x,
        }
    }
}

pub fn fmt_q(v: &Q) -> String {
    if v.is_integer() {
        v.numer().to_string()
    } else {
        format!("{}/{}", v.numer(), v.denom())
    }
}

/// `sqrt(d)` for rational d written as `m·√s / n` with s square-free.
fn simplify_sqrt(d: &Q) -> (BigInt, BigInt, BigInt) {
    // sqrt(a/b) = sqrt(a·b)/b
    let n = d.numer() * d.denom();
    let mut outside = BigInt::one();
    let mut inside = n.clone();
    let mut f = BigInt::from(2);
    while &f * &f <= inside && f < BigInt::from(100_000) {
        let sq = &f * &f;
        while (&inside % &sq).is_zero() {
            inside /= &sq;
            outside *= &f;
        }
        f += 1;
    }
    (outside, inside, d.denom().clone())
}

impl fmt::Display for Root {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Root::Rational(r) => write!(f, "{}", fmt_q(r)),
            Root::Approx(x) => write!(f, "≈ {x:.6}"),
            Root::Quadratic { p, k, d } => {
                let (m, s, n) = simplify_sqrt(d);
                // value = p + k·(m/n)·√s
                let coef = k * Q::new(m, n);
                let sign = if coef.is_negative() { "-" } else { "+" };
                let c = coef.abs();
                let radical = if c.is_one() {
                    format!("√{s}")
                } else if c.is_integer() {
                    format!("{}√{s}", c.numer())
                } else if c.numer().is_one() {
                    format!("√{s}/{}", c.denom())
                } else {
                    format!("{}√{s}/{}", c.numer(), c.denom())
                };
                if p.is_zero() {
                    let lead = if coef.is_negative() { "-" } else { "" };
                    write!(f, "{lead}{radical}")
                } else {
                    write!(f, "{} {sign} {radical}", fmt_q(p))
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(c: &[i64]) -> Poly {
        Poly::from_coeffs(c.iter().map(|v| q(*v)).collect())
    }

    #[test]
    fn arithmetic_gcd_and_squarefree() {
        // (x-1)^2 (x+2) = x^3 - 3x + 2
        let a = p(&[2, -3, 0, 1]);
        assert_eq!(a.squarefree(), p(&[-2, 1, 1])); // (x-1)(x+2)
        assert_eq!(a.gcd(&p(&[-1, 1])), p(&[-1, 1]));
        let (qq, r) = a.divrem(&p(&[-1, 1]));
        assert!(r.is_zero());
        assert_eq!(qq, p(&[-2, 1, 1]));
    }

    #[test]
    fn real_roots_counted_and_found() {
        assert_eq!(p(&[1, 0, 1]).count_real_roots(), 0); // x^2 + 1
        assert_eq!(p(&[-9, 0, 1]).count_real_roots(), 2);
        let r = p(&[-9, 0, 1]).real_roots();
        assert_eq!(
            r.iter().map(|x| x.to_string()).collect::<Vec<_>>(),
            ["-3", "3"]
        );
        // x^2 - x - 1: (1 ± √5)/2
        let r = p(&[-1, -1, 1]).real_roots();
        assert_eq!(
            r.iter().map(|x| x.to_string()).collect::<Vec<_>>(),
            ["1/2 - √5/2", "1/2 + √5/2"]
        );
        // x^3 - 2: one real root, irrational
        let r = p(&[-2, 0, 0, 1]).real_roots();
        assert_eq!(r.len(), 1);
        assert!((r[0].value() - 2f64.cbrt()).abs() < 1e-12);
        // 2x - 1
        assert_eq!(p(&[-1, 2]).real_roots()[0].to_string(), "1/2");
    }
}
