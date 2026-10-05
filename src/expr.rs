//! Parsing a line of school algebra into an exact rational function of
//! one unknown, and remembering where it is undefined.
//!
//! Accepted: integers and decimals, one letter as the unknown, `+ - * /`,
//! `^` with an integer exponent, parentheses, implicit multiplication
//! (`2x`, `x(x+1)`, `(x+1)(x-1)`), and the typographic `−`, `×`, `·`, `÷`,
//! `²`, `³`.

use crate::poly::{Poly, Q};
use num_bigint::BigInt;
use num_traits::One;
use std::fmt;

/// The longest line read, in characters.
pub const MAX_LINE_CHARS: usize = 500;
/// The deepest nesting of brackets and signs (each sign applies to
/// everything after it, so `--x` is two levels).
pub const MAX_DEPTH: usize = 64;
/// The highest degree of any polynomial a line builds while it is read:
/// a numerator, a denominator, or the product of the divisors that can
/// vanish. The same as the largest exponent.
pub const MAX_DEGREE: usize = 64;
/// The most alternatives in an answer line (`x = 1 or x = 2 or ...`).
pub const MAX_ALTERNATIVES: usize = 12;

#[derive(Debug, Clone, PartialEq)]
pub enum ParseError {
    Empty,
    Unexpected {
        at: usize,
        found: String,
    },
    TwoUnknowns(char, char),
    ExponentNotInteger {
        at: usize,
    },
    ExponentTooLarge {
        at: usize,
    },
    DivisionByZero,
    TooManyEquals,
    /// Longer than [`MAX_LINE_CHARS`]; not read at all.
    TooLong {
        max: usize,
    },
    /// Brackets and signs nested deeper than [`MAX_DEPTH`].
    TooDeep {
        max: usize,
    },
    /// A polynomial of a degree above [`MAX_DEGREE`] would be needed.
    TooComplex {
        max: usize,
    },
    /// More alternatives than [`MAX_ALTERNATIVES`].
    TooManyAlternatives {
        max: usize,
    },
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseError::Empty => write!(f, "the line is empty"),
            ParseError::Unexpected { at, found } => {
                write!(f, "unexpected {found} at position {}", at + 1)
            }
            ParseError::TwoUnknowns(a, b) => {
                write!(f, "two unknowns ({a} and {b}); one is supported")
            }
            ParseError::ExponentNotInteger { at } => write!(
                f,
                "the exponent at position {} must be a whole number",
                at + 1
            ),
            ParseError::ExponentTooLarge { at } => {
                write!(f, "the exponent at position {} is too large", at + 1)
            }
            ParseError::DivisionByZero => write!(f, "division by zero"),
            ParseError::TooManyEquals => write!(f, "more than one '='"),
            ParseError::TooLong { max } => write!(f, "the line is longer than {max} characters"),
            ParseError::TooDeep { max } => {
                write!(f, "brackets and signs are nested more than {max} deep")
            }
            ParseError::TooComplex { max } => write!(
                f,
                "the line is too complex to check: its degree is above {max}"
            ),
            ParseError::TooManyAlternatives { max } => write!(f, "more than {max} alternatives"),
        }
    }
}

impl std::error::Error for ParseError {}

#[derive(Debug, Clone, PartialEq)]
enum Tok {
    Num(Q),
    Var(char),
    Op(char),
    LParen,
    RParen,
    Eq,
    Sup(i64),
}

fn lex(s: &str) -> Result<Vec<(usize, Tok)>, ParseError> {
    let chars: Vec<char> = s.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        match c {
            ' ' | '\t' => {}
            '0'..='9' | '.' => {
                let start = i;
                while i + 1 < chars.len() && (chars[i + 1].is_ascii_digit() || chars[i + 1] == '.')
                {
                    i += 1;
                }
                let text: String = chars[start..=i].iter().collect();
                let (int, frac) = text.split_once('.').unwrap_or((&text, ""));
                if frac.contains('.') || (int.is_empty() && frac.is_empty()) {
                    return Err(ParseError::Unexpected {
                        at: start,
                        found: format!("'{text}'"),
                    });
                }
                let digits = format!("{int}{frac}");
                let n: BigInt = digits.parse().map_err(|_| ParseError::Unexpected {
                    at: start,
                    found: format!("'{text}'"),
                })?;
                let d = BigInt::from(10).pow(frac.len() as u32);
                out.push((start, Tok::Num(Q::new(n, d))));
            }
            'a'..='z' | 'A'..='Z' => out.push((i, Tok::Var(c))),
            '+' | '*' | '/' | '^' | '-' => out.push((i, Tok::Op(c))),
            '−' | '–' => out.push((i, Tok::Op('-'))),
            '×' | '·' | '⋅' => out.push((i, Tok::Op('*'))),
            '÷' | ':' => out.push((i, Tok::Op('/'))),
            '²' => out.push((i, Tok::Sup(2))),
            '³' => out.push((i, Tok::Sup(3))),
            '(' | '[' => out.push((i, Tok::LParen)),
            '±' => {
                return Err(ParseError::Unexpected {
                    at: i,
                    found: "'±' (write it once per line, as in x = ±3)".into(),
                })
            }
            ')' | ']' => out.push((i, Tok::RParen)),
            '=' => out.push((i, Tok::Eq)),
            _ => {
                return Err(ParseError::Unexpected {
                    at: i,
                    found: format!("'{c}'"),
                })
            }
        }
        i += 1;
    }
    Ok(out)
}

/// A rational function `num / den` of the unknown, reduced, together with
/// the polynomial whose real roots are the points where the original
/// expression is undefined (every divisor that can vanish, before any
/// cancellation).
#[derive(Clone, Debug, PartialEq)]
pub struct Rational {
    pub num: Poly,
    pub den: Poly,
    /// Squarefree, monic; its real roots are excluded from the domain.
    pub excluded: Poly,
}

impl Rational {
    fn constant(v: Q) -> Self {
        Rational {
            num: Poly::constant(v),
            den: Poly::one(),
            excluded: Poly::one(),
        }
    }
    fn var() -> Self {
        Rational {
            num: Poly::x(),
            den: Poly::one(),
            excluded: Poly::one(),
        }
    }
    fn exclude(a: &Poly, b: &Poly) -> Poly {
        let prod = a.mul(b);
        if prod.degree().unwrap_or(0) == 0 {
            Poly::one()
        } else {
            prod.squarefree()
        }
    }
    fn reduced(num: Poly, den: Poly, excluded: Poly) -> Self {
        if num.is_zero() {
            return Rational {
                num,
                den: Poly::one(),
                excluded,
            };
        }
        let g = num.gcd(&den);
        let (mut n, _) = num.divrem(&g);
        let (mut d, _) = den.divrem(&g);
        // Monic denominator for a canonical form.
        let k = Q::one() / d.lead();
        n = n.scale(&k);
        d = d.scale(&k);
        Rational {
            num: n,
            den: d,
            excluded,
        }
    }
    pub fn add(&self, o: &Self) -> Self {
        Self::reduced(
            self.num.mul(&o.den).add(&o.num.mul(&self.den)),
            self.den.mul(&o.den),
            Self::exclude(&self.excluded, &o.excluded),
        )
    }
    pub fn neg(&self) -> Self {
        Rational {
            num: self.num.neg(),
            den: self.den.clone(),
            excluded: self.excluded.clone(),
        }
    }
    pub fn sub(&self, o: &Self) -> Self {
        self.add(&o.neg())
    }
    pub fn mul(&self, o: &Self) -> Self {
        Self::reduced(
            self.num.mul(&o.num),
            self.den.mul(&o.den),
            Self::exclude(&self.excluded, &o.excluded),
        )
    }
    pub fn div(&self, o: &Self) -> Result<Self, ParseError> {
        if o.num.is_zero() {
            return Err(ParseError::DivisionByZero);
        }
        // Dividing by o is undefined where o is zero.
        let ex = Self::exclude(&Self::exclude(&self.excluded, &o.excluded), &o.num);
        Ok(Self::reduced(
            self.num.mul(&o.den),
            self.den.mul(&o.num),
            ex,
        ))
    }
    fn powi(&self, e: i64) -> Result<Self, ParseError> {
        let mut acc = Rational::constant(Q::one());
        for _ in 0..e.unsigned_abs() {
            acc = acc.mul(self);
        }
        if e < 0 {
            let one = Rational::constant(Q::one());
            acc = one.div(&acc)?;
        }
        acc.excluded = Self::exclude(&acc.excluded, &self.excluded);
        Ok(acc)
    }
    pub fn is_zero(&self) -> bool {
        self.num.is_zero()
    }
}

fn deg(p: &Poly) -> usize {
    p.degree().unwrap_or(0)
}

/// Refuses an operation whose result could need a polynomial of a degree
/// above [`MAX_DEGREE`], before any of it is computed; `degrees` are upper
/// bounds of the numerator, the denominator and the excluded points of
/// the result.
fn within_degree(degrees: [usize; 3]) -> Result<(), ParseError> {
    if degrees.iter().any(|d| *d > MAX_DEGREE) {
        Err(ParseError::TooComplex { max: MAX_DEGREE })
    } else {
        Ok(())
    }
}

fn add_degrees(a: &Rational, b: &Rational) -> [usize; 3] {
    [
        (deg(&a.num) + deg(&b.den)).max(deg(&b.num) + deg(&a.den)),
        deg(&a.den) + deg(&b.den),
        deg(&a.excluded) + deg(&b.excluded),
    ]
}

fn mul_degrees(a: &Rational, b: &Rational) -> [usize; 3] {
    [
        deg(&a.num) + deg(&b.num),
        deg(&a.den) + deg(&b.den),
        deg(&a.excluded) + deg(&b.excluded),
    ]
}

fn div_degrees(a: &Rational, b: &Rational) -> [usize; 3] {
    [
        deg(&a.num) + deg(&b.den),
        deg(&a.den) + deg(&b.num),
        deg(&a.excluded) + deg(&b.excluded) + deg(&b.num),
    ]
}

/// Bounds for `r` to the power `e`: every part of `r` times `|e|`, and
/// its excluded points once more.
fn pow_degrees(r: &Rational, e: i64) -> [usize; 3] {
    let n = usize::try_from(e.unsigned_abs()).unwrap_or(usize::MAX);
    let size = deg(&r.num).max(deg(&r.den));
    [
        size.saturating_mul(n),
        size.saturating_mul(n),
        size.saturating_mul(n).saturating_add(deg(&r.excluded)),
    ]
}

struct Parser {
    toks: Vec<(usize, Tok)>,
    pos: usize,
    var: Option<char>,
    /// Brackets and signs open around the current token.
    depth: usize,
}

impl Parser {
    fn peek(&self) -> Option<&Tok> {
        self.toks.get(self.pos).map(|(_, t)| t)
    }
    fn at(&self) -> usize {
        self.toks
            .get(self.pos)
            .map(|(i, _)| *i)
            .unwrap_or(usize::MAX)
    }
    /// One level deeper (a bracket or a sign), within [`MAX_DEPTH`]: the
    /// parser recurses once per level, so this bounds its stack.
    fn enter(&mut self) -> Result<(), ParseError> {
        if self.depth >= MAX_DEPTH {
            return Err(ParseError::TooDeep { max: MAX_DEPTH });
        }
        self.depth += 1;
        Ok(())
    }
    fn unexpected(&self) -> ParseError {
        match self.toks.get(self.pos) {
            Some((i, t)) => ParseError::Unexpected {
                at: *i,
                found: format!("{t:?}"),
            },
            None => ParseError::Unexpected {
                at: self.toks.last().map(|(i, _)| *i + 1).unwrap_or(0),
                found: "end of line".into(),
            },
        }
    }
    fn expr(&mut self) -> Result<Rational, ParseError> {
        let mut acc = self.term()?;
        while let Some(Tok::Op(op @ ('+' | '-'))) = self.peek().cloned() {
            self.pos += 1;
            let rhs = self.term()?;
            within_degree(add_degrees(&acc, &rhs))?;
            acc = if op == '+' {
                acc.add(&rhs)
            } else {
                acc.sub(&rhs)
            };
        }
        Ok(acc)
    }
    fn term(&mut self) -> Result<Rational, ParseError> {
        let mut acc = self.unary()?;
        loop {
            match self.peek().cloned() {
                Some(Tok::Op('*')) => {
                    self.pos += 1;
                    let rhs = self.unary()?;
                    within_degree(mul_degrees(&acc, &rhs))?;
                    acc = acc.mul(&rhs);
                }
                Some(Tok::Op('/')) => {
                    self.pos += 1;
                    let rhs = self.unary()?;
                    within_degree(div_degrees(&acc, &rhs))?;
                    acc = acc.div(&rhs)?;
                }
                // Implicit multiplication: 2x, x(x+1), (x+1)(x-1), 2(3).
                Some(Tok::Num(_) | Tok::Var(_) | Tok::LParen) => {
                    let rhs = self.power()?;
                    within_degree(mul_degrees(&acc, &rhs))?;
                    acc = acc.mul(&rhs);
                }
                _ => return Ok(acc),
            }
        }
    }
    fn unary(&mut self) -> Result<Rational, ParseError> {
        match self.peek() {
            Some(Tok::Op(sign @ ('-' | '+'))) => {
                let negate = *sign == '-';
                self.pos += 1;
                self.enter()?;
                let r = self.unary()?;
                self.depth -= 1;
                Ok(if negate { r.neg() } else { r })
            }
            _ => self.power(),
        }
    }
    fn power(&mut self) -> Result<Rational, ParseError> {
        let base = self.atom()?;
        match self.peek().cloned() {
            Some(Tok::Sup(e)) => {
                self.pos += 1;
                within_degree(pow_degrees(&base, e))?;
                base.powi(e)
            }
            Some(Tok::Op('^')) => {
                self.pos += 1;
                let at = self.at();
                let e = self.exponent(at)?;
                within_degree(pow_degrees(&base, e))?;
                base.powi(e)
            }
            _ => Ok(base),
        }
    }
    /// An integer exponent: `2`, `-1`, `(2)`, `(-1)`.
    fn exponent(&mut self, at: usize) -> Result<i64, ParseError> {
        let mut paren = false;
        if self.peek() == Some(&Tok::LParen) {
            paren = true;
            self.pos += 1;
        }
        let mut sign = 1;
        if self.peek() == Some(&Tok::Op('-')) {
            sign = -1;
            self.pos += 1;
        }
        let Some(Tok::Num(n)) = self.peek().cloned() else {
            return Err(ParseError::ExponentNotInteger { at });
        };
        self.pos += 1;
        if !n.is_integer() {
            return Err(ParseError::ExponentNotInteger { at });
        }
        if paren {
            if self.peek() != Some(&Tok::RParen) {
                return Err(ParseError::ExponentNotInteger { at });
            }
            self.pos += 1;
        }
        let v: i64 = n
            .to_integer()
            .try_into()
            .map_err(|_| ParseError::ExponentTooLarge { at })?;
        if v.abs() > 64 {
            return Err(ParseError::ExponentTooLarge { at });
        }
        Ok(sign * v)
    }
    fn atom(&mut self) -> Result<Rational, ParseError> {
        match self.peek().cloned() {
            Some(Tok::Num(n)) => {
                self.pos += 1;
                Ok(Rational::constant(n))
            }
            Some(Tok::Var(c)) => {
                match self.var {
                    None => self.var = Some(c),
                    Some(v) if v != c => return Err(ParseError::TwoUnknowns(v, c)),
                    _ => {}
                }
                self.pos += 1;
                Ok(Rational::var())
            }
            Some(Tok::LParen) => {
                self.pos += 1;
                self.enter()?;
                let e = self.expr()?;
                if self.peek() != Some(&Tok::RParen) {
                    return Err(self.unexpected());
                }
                self.pos += 1;
                self.depth -= 1;
                Ok(e)
            }
            _ => Err(self.unexpected()),
        }
    }
}

/// A parsed line: an expression, or an equation `left = right`.
#[derive(Clone, Debug, PartialEq)]
pub enum Line {
    Expression(Rational),
    /// Stored as `left − right`, with the domain of both sides.
    Equation(Rational),
}

/// Split an answer line into its alternatives: `x = 2 or x = 3`,
/// `x = 2, x = 3`, `x = 2; x = 3`, and `x = ±3` (one `±` expands into a
/// `+` and a `-` copy).
fn alternatives(s: &str) -> Vec<String> {
    let mut parts: Vec<String> = Vec::new();
    let lower = s.replace(" OR ", " or ").replace(" Or ", " or ");
    for chunk in lower.split(" or ").flat_map(|c| c.split([',', ';'])) {
        let chunk = chunk.trim();
        if chunk.is_empty() {
            continue;
        }
        if chunk.matches('±').count() == 1 {
            parts.push(chunk.replace('±', "+"));
            parts.push(chunk.replace('±', "-"));
        } else {
            parts.push(chunk.to_string());
        }
    }
    parts
}

/// Parse a line. An answer line with alternatives (`x = 2 or x = 3`,
/// `x = ±3`) becomes one equation whose solutions are the union: the
/// product of the alternatives' `left − right`, with every alternative's
/// excluded points. The unknown's letter found in the line, if any.
pub fn parse_line(s: &str) -> Result<(Line, Option<char>), ParseError> {
    if s.chars().count() > MAX_LINE_CHARS {
        return Err(ParseError::TooLong {
            max: MAX_LINE_CHARS,
        });
    }
    let alts = alternatives(s);
    if alts.len() > MAX_ALTERNATIVES {
        return Err(ParseError::TooManyAlternatives {
            max: MAX_ALTERNATIVES,
        });
    }
    if alts.len() > 1 {
        let mut product: Option<Rational> = None;
        let mut var = None;
        for a in &alts {
            let (line, v) = parse_single(a)?;
            let Line::Equation(r) = line else {
                return Err(ParseError::Unexpected {
                    at: 0,
                    found: format!("'{a}' (each alternative must be an equation)"),
                });
            };
            if let (Some(x), Some(y)) = (var, v) {
                if x != y {
                    return Err(ParseError::TwoUnknowns(x, y));
                }
            }
            var = var.or(v);
            // The union of solution sets: zeros of the product of numerators.
            let part = Rational {
                num: r.num,
                den: Poly::one(),
                excluded: r.excluded,
            };
            product = Some(match product {
                None => part,
                Some(p) => {
                    within_degree(mul_degrees(&p, &part))?;
                    p.mul(&part)
                }
            });
        }
        return Ok((
            Line::Equation(product.expect("two or more alternatives")),
            var,
        ));
    }
    parse_single(s)
}

fn parse_single(s: &str) -> Result<(Line, Option<char>), ParseError> {
    let toks = lex(s)?;
    if toks.is_empty() {
        return Err(ParseError::Empty);
    }
    let eqs = toks.iter().filter(|(_, t)| *t == Tok::Eq).count();
    if eqs > 1 {
        return Err(ParseError::TooManyEquals);
    }
    let mut p = Parser {
        toks,
        pos: 0,
        var: None,
        depth: 0,
    };
    let left = p.expr()?;
    if eqs == 0 {
        if p.pos != p.toks.len() {
            return Err(p.unexpected());
        }
        return Ok((Line::Expression(left), p.var));
    }
    if p.peek() != Some(&Tok::Eq) {
        return Err(p.unexpected());
    }
    p.pos += 1;
    let right = p.expr()?;
    if p.pos != p.toks.len() {
        return Err(p.unexpected());
    }
    Ok((Line::Equation(left.sub(&right)), p.var))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::poly::q;
    use num_traits::Zero;

    fn eq(s: &str) -> Rational {
        match parse_line(s).expect("parses").0 {
            Line::Equation(r) => r,
            Line::Expression(_) => panic!("not an equation"),
        }
    }

    #[test]
    fn implicit_multiplication_and_typography() {
        let a = eq("2x(x+1) = 4");
        let b = eq("2*x*(x+1)=4");
        assert_eq!(a, b);
        let c = eq("x² − 9 = 0");
        let d = eq("x^2-9=0");
        assert_eq!(c, d);
        assert_eq!(eq("0.5x = 1"), eq("x/2 = 1"));
    }

    #[test]
    fn domain_remembers_cancelled_divisors() {
        // (x^2 - 1)/(x - 1) reduces to x + 1 but stays undefined at 1.
        let r = eq("(x^2 - 1)/(x - 1) = 2");
        assert_eq!(r.num.degree(), Some(1));
        assert!(r.excluded.eval(&q(1)).is_zero());
        assert!(!r.excluded.eval(&q(2)).is_zero());
    }

    #[test]
    fn answer_lines_with_alternatives() {
        let a = eq("x = 3 or x = -3");
        let b = eq("x = ±3");
        let c = eq("x = 3, x = -3");
        let d = eq("x^2 = 9");
        assert_eq!(a.num.squarefree(), d.num.squarefree());
        assert_eq!(b.num.squarefree(), d.num.squarefree());
        assert_eq!(c.num.squarefree(), d.num.squarefree());
    }

    #[test]
    fn errors_point_at_the_problem() {
        assert!(matches!(
            parse_line("x + y = 1"),
            Err(ParseError::TwoUnknowns('x', 'y'))
        ));
        assert!(matches!(
            parse_line("x^0.5 = 1"),
            Err(ParseError::ExponentNotInteger { .. })
        ));
        assert!(matches!(
            parse_line("x = 1 = 2"),
            Err(ParseError::TooManyEquals)
        ));
        assert!(matches!(
            parse_line("1/0 = x"),
            Err(ParseError::DivisionByZero)
        ));
        assert!(matches!(
            parse_line("x + = 1"),
            Err(ParseError::Unexpected { .. })
        ));
    }
}
