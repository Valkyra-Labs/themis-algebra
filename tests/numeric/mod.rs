//! A small floating-point evaluator for lines of school algebra, written
//! for the substitution check and nothing else.
//!
//! It deliberately shares no code with the engine: no parser, no exact
//! rationals, no polynomials. A line is parsed into a tree once and the
//! tree is evaluated in `f64` at a given x. A bug in the engine therefore
//! cannot hide behind the same bug here.
//!
//! Accepted: decimal numbers, one letter as the unknown, `+ - * /`, `^`
//! with an integer exponent (`x^2`, `x^-1`, `x^(-1)`), the superscripts
//! `²` and `³`, unary minus, parentheses, implicit multiplication (`5x`,
//! `2(x-1)`, `(x-2)(x-3)`, `2√3`), the typographic `−`, `–`, `×`, `·`,
//! `÷`, and `√` applied to the factor after it (`√5/2` is `(√5)/2`), so
//! that the engine's printed roots can be read back. A line with `=` is an
//! equation; an answer line may list alternatives separated by ` or `,
//! `,` or `;`, and one `±` per alternative expands into a `+` and a `-`
//! copy. Commas are separators, never decimal points.
//!
//! Every value carries a magnitude: a bound on the size of the terms that
//! were added or multiplied to produce it. Deciding that a value is zero
//! (a root, or a vanishing divisor) uses a tolerance relative to that
//! magnitude, plus a small absolute floor.

/// A parsed expression.
#[derive(Clone, Debug, PartialEq)]
pub enum Ast {
    Num(f64),
    X,
    Neg(Box<Ast>),
    Add(Box<Ast>, Box<Ast>),
    Sub(Box<Ast>, Box<Ast>),
    Mul(Box<Ast>, Box<Ast>),
    Div(Box<Ast>, Box<Ast>),
    Pow(Box<Ast>, i32),
    Sqrt(Box<Ast>),
}

/// A value and the magnitude of what produced it.
#[derive(Clone, Copy, Debug)]
pub struct Num {
    pub v: f64,
    pub mag: f64,
}

/// When a value counts as zero: `|v| <= abs + rel * mag`.
#[derive(Clone, Copy, Debug)]
pub struct Tol {
    pub rel: f64,
    pub abs: f64,
}

impl Tol {
    pub fn is_zero(&self, n: Num) -> bool {
        n.v.abs() <= self.abs + self.rel * n.mag
    }
}

/// A line: an expression, or an equation given as one or more
/// alternatives `left = right` (their solution sets are united).
#[derive(Clone, Debug, PartialEq)]
pub enum Line {
    Expression(Ast),
    Equation(Vec<(Ast, Ast)>),
}

/// What a line does at one point.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Status {
    /// Defined, and (for an equation) some alternative holds.
    Holds,
    /// Defined, and no alternative holds.
    Fails,
    /// Some divisor vanishes (in any alternative), or a square root of a
    /// negative number is taken.
    Undefined,
}

#[derive(Clone, Debug, PartialEq)]
enum Tok {
    Num(f64),
    Var(char),
    Op(char),
    Sup(i32),
    Sqrt,
    LParen,
    RParen,
    Eq,
}

fn lex(s: &str) -> Result<Vec<Tok>, String> {
    let chars: Vec<char> = s.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        match c {
            c if c.is_whitespace() => {}
            '0'..='9' | '.' => {
                let start = i;
                while i + 1 < chars.len() && (chars[i + 1].is_ascii_digit() || chars[i + 1] == '.')
                {
                    i += 1;
                }
                let text: String = chars[start..=i].iter().collect();
                let v: f64 = text
                    .parse()
                    .map_err(|_| format!("bad number '{text}' in '{s}'"))?;
                out.push(Tok::Num(v));
            }
            c if c.is_ascii_alphabetic() => out.push(Tok::Var(c)),
            '+' | '-' | '*' | '/' | '^' => out.push(Tok::Op(c)),
            '\u{2212}' | '\u{2013}' => out.push(Tok::Op('-')),
            '×' | '·' | '⋅' => out.push(Tok::Op('*')),
            '÷' => out.push(Tok::Op('/')),
            '²' => out.push(Tok::Sup(2)),
            '³' => out.push(Tok::Sup(3)),
            '√' => out.push(Tok::Sqrt),
            '(' => out.push(Tok::LParen),
            ')' => out.push(Tok::RParen),
            '=' => out.push(Tok::Eq),
            _ => return Err(format!("unexpected '{c}' in '{s}'")),
        }
        i += 1;
    }
    Ok(out)
}

struct Parser<'a> {
    toks: &'a [Tok],
    pos: usize,
    unknown: Option<char>,
}

impl Parser<'_> {
    fn peek(&self) -> Option<&Tok> {
        self.toks.get(self.pos)
    }

    fn bump(&mut self) -> Option<Tok> {
        let t = self.toks.get(self.pos).cloned();
        self.pos += 1;
        t
    }

    // sum := product (('+' | '-') product)*
    fn sum(&mut self) -> Result<Ast, String> {
        let mut acc = self.product()?;
        loop {
            match self.peek() {
                Some(Tok::Op('+')) => {
                    self.pos += 1;
                    acc = Ast::Add(Box::new(acc), Box::new(self.product()?));
                }
                Some(Tok::Op('-')) => {
                    self.pos += 1;
                    acc = Ast::Sub(Box::new(acc), Box::new(self.product()?));
                }
                _ => return Ok(acc),
            }
        }
    }

    // product := signed (('*' | '/') signed | factor)*
    // The bare `factor` case is implicit multiplication.
    fn product(&mut self) -> Result<Ast, String> {
        let mut acc = self.signed()?;
        loop {
            match self.peek() {
                Some(Tok::Op('*')) => {
                    self.pos += 1;
                    acc = Ast::Mul(Box::new(acc), Box::new(self.signed()?));
                }
                Some(Tok::Op('/')) => {
                    self.pos += 1;
                    acc = Ast::Div(Box::new(acc), Box::new(self.signed()?));
                }
                Some(Tok::Num(_) | Tok::Var(_) | Tok::LParen | Tok::Sqrt) => {
                    acc = Ast::Mul(Box::new(acc), Box::new(self.factor()?));
                }
                _ => return Ok(acc),
            }
        }
    }

    // signed := ('-' | '+') signed | factor
    fn signed(&mut self) -> Result<Ast, String> {
        match self.peek() {
            Some(Tok::Op('-')) => {
                self.pos += 1;
                Ok(Ast::Neg(Box::new(self.signed()?)))
            }
            Some(Tok::Op('+')) => {
                self.pos += 1;
                self.signed()
            }
            _ => self.factor(),
        }
    }

    // factor := '√' factor | primary ('^' exponent | superscript)?
    fn factor(&mut self) -> Result<Ast, String> {
        if self.peek() == Some(&Tok::Sqrt) {
            self.pos += 1;
            return Ok(Ast::Sqrt(Box::new(self.factor()?)));
        }
        let base = self.primary()?;
        match self.peek() {
            Some(Tok::Sup(n)) => {
                let n = *n;
                self.pos += 1;
                Ok(Ast::Pow(Box::new(base), n))
            }
            Some(Tok::Op('^')) => {
                self.pos += 1;
                let n = self.exponent()?;
                Ok(Ast::Pow(Box::new(base), n))
            }
            _ => Ok(base),
        }
    }

    // exponent := '('? ('-' | '+')? integer ')'?
    fn exponent(&mut self) -> Result<i32, String> {
        let paren = self.peek() == Some(&Tok::LParen);
        if paren {
            self.pos += 1;
        }
        let mut sign = 1;
        match self.peek() {
            Some(Tok::Op('-')) => {
                sign = -1;
                self.pos += 1;
            }
            Some(Tok::Op('+')) => self.pos += 1,
            _ => {}
        }
        let n = match self.bump() {
            Some(Tok::Num(v)) if v.fract() == 0.0 && v <= 64.0 => v as i32,
            other => return Err(format!("exponent must be a small integer, found {other:?}")),
        };
        if paren && self.bump() != Some(Tok::RParen) {
            return Err("unclosed exponent".into());
        }
        Ok(sign * n)
    }

    // primary := number | letter | '(' sum ')'
    fn primary(&mut self) -> Result<Ast, String> {
        match self.bump() {
            Some(Tok::Num(v)) => Ok(Ast::Num(v)),
            Some(Tok::Var(c)) => {
                if let Some(u) = self.unknown {
                    if u != c {
                        return Err(format!("two unknowns: {u} and {c}"));
                    }
                }
                self.unknown = Some(c);
                Ok(Ast::X)
            }
            Some(Tok::LParen) => {
                let inner = self.sum()?;
                if self.bump() != Some(Tok::RParen) {
                    return Err("missing ')'".into());
                }
                Ok(inner)
            }
            other => Err(format!(
                "expected a number, the unknown or '(', found {other:?}"
            )),
        }
    }
}

/// Parse one expression (no `=`).
pub fn parse_expr(s: &str) -> Result<Ast, String> {
    let toks = lex(s)?;
    let mut p = Parser {
        toks: &toks,
        pos: 0,
        unknown: None,
    };
    let e = p.sum()?;
    if p.pos != toks.len() {
        return Err(format!("trailing input in '{s}'"));
    }
    Ok(e)
}

fn split_alternatives(s: &str) -> Result<Vec<String>, String> {
    let mut out = Vec::new();
    for chunk in s.split([',', ';']) {
        for piece in chunk
            .replace(" OR ", " or ")
            .replace(" Or ", " or ")
            .split(" or ")
        {
            let piece = piece.trim();
            if piece.is_empty() {
                continue;
            }
            match piece.matches('±').count() {
                0 => out.push(piece.to_string()),
                1 => {
                    out.push(piece.replace('±', "+"));
                    out.push(piece.replace('±', "-"));
                }
                _ => return Err(format!("more than one '±' in '{piece}'")),
            }
        }
    }
    if out.is_empty() {
        return Err("empty line".into());
    }
    Ok(out)
}

/// Parse a line: an expression, an equation, or an answer line with
/// alternatives.
pub fn parse_line(s: &str) -> Result<Line, String> {
    let alts = split_alternatives(s)?;
    let mut eqs = Vec::new();
    let mut unknown: Option<char> = None;
    for a in &alts {
        let letters: Vec<char> = a.chars().filter(|c| c.is_ascii_alphabetic()).collect();
        if let (Some(u), Some(&c)) = (unknown, letters.first()) {
            if u != c {
                return Err(format!("two unknowns: {u} and {c}"));
            }
        }
        unknown = unknown.or(letters.first().copied());
        let parts: Vec<&str> = a.split('=').collect();
        match parts.as_slice() {
            [e] if alts.len() == 1 => return Ok(Line::Expression(parse_expr(e)?)),
            [_] => return Err(format!("alternative '{a}' is not an equation")),
            [l, r] => eqs.push((parse_expr(l)?, parse_expr(r)?)),
            _ => return Err(format!("more than one '=' in '{a}'")),
        }
    }
    Ok(Line::Equation(eqs))
}

fn num(v: f64, mag: f64) -> Option<Num> {
    if v.is_finite() && mag.is_finite() {
        Some(Num { v, mag })
    } else {
        None
    }
}

/// Evaluate at x; `None` where the expression is undefined.
pub fn eval(e: &Ast, x: f64, tol: Tol) -> Option<Num> {
    match e {
        Ast::Num(v) => num(*v, v.abs()),
        Ast::X => num(x, x.abs()),
        Ast::Neg(a) => {
            let a = eval(a, x, tol)?;
            num(-a.v, a.mag)
        }
        Ast::Add(a, b) => {
            let (a, b) = (eval(a, x, tol)?, eval(b, x, tol)?);
            num(a.v + b.v, a.mag + b.mag)
        }
        Ast::Sub(a, b) => {
            let (a, b) = (eval(a, x, tol)?, eval(b, x, tol)?);
            num(a.v - b.v, a.mag + b.mag)
        }
        Ast::Mul(a, b) => {
            let (a, b) = (eval(a, x, tol)?, eval(b, x, tol)?);
            num(a.v * b.v, a.mag * b.mag)
        }
        Ast::Div(a, b) => {
            // Both operands are evaluated first: a divisor inside the
            // numerator makes the whole quotient undefined too.
            let (a, b) = (eval(a, x, tol)?, eval(b, x, tol)?);
            if tol.is_zero(b) {
                return None;
            }
            num(a.v / b.v, a.mag / b.v.abs())
        }
        Ast::Pow(a, n) => {
            let a = eval(a, x, tol)?;
            let k = n.unsigned_abs() as i32;
            let p = num(a.v.powi(k), a.mag.powi(k))?;
            if *n >= 0 {
                Some(p)
            } else if tol.is_zero(p) {
                None
            } else {
                num(1.0 / p.v, 1.0 / p.v.abs())
            }
        }
        Ast::Sqrt(a) => {
            let a = eval(a, x, tol)?;
            if a.v < 0.0 {
                return None;
            }
            num(a.v.sqrt(), a.mag.sqrt())
        }
    }
}

impl Line {
    pub fn is_equation(&self) -> bool {
        matches!(self, Line::Equation(_))
    }

    /// For an equation: whether it holds, fails or is undefined at x.
    /// For an expression: `Undefined` or `Holds` (defined).
    pub fn status(&self, x: f64, tol: Tol) -> Status {
        match self {
            Line::Expression(e) => match eval(e, x, tol) {
                Some(_) => Status::Holds,
                None => Status::Undefined,
            },
            Line::Equation(alts) => {
                let mut any = false;
                for (l, r) in alts {
                    match (eval(l, x, tol), eval(r, x, tol)) {
                        (Some(l), Some(r)) => {
                            any |= tol.is_zero(Num {
                                v: l.v - r.v,
                                mag: l.mag + r.mag,
                            })
                        }
                        _ => return Status::Undefined,
                    }
                }
                if any {
                    Status::Holds
                } else {
                    Status::Fails
                }
            }
        }
    }

    /// The smallest `|left - right| / magnitude` over the alternatives
    /// (scale-free distance from holding); `None` where undefined.
    pub fn relative_residual(&self, x: f64, tol: Tol) -> Option<f64> {
        let Line::Equation(alts) = self else {
            return None;
        };
        let mut best = f64::INFINITY;
        for (l, r) in alts {
            let (l, r) = (eval(l, x, tol)?, eval(r, x, tol)?);
            let mag = (l.mag + r.mag).max(f64::MIN_POSITIVE);
            best = best.min((l.v - r.v).abs() / mag);
        }
        Some(best)
    }

    /// An expression's value at x.
    pub fn value(&self, x: f64, tol: Tol) -> Option<Num> {
        match self {
            Line::Expression(e) => eval(e, x, tol),
            Line::Equation(_) => None,
        }
    }

    /// Both sides' values at x, for failure messages.
    pub fn describe(&self, x: f64, tol: Tol) -> String {
        let show = |v: Option<Num>| match v {
            Some(n) => format!("{:e}", n.v),
            None => "undefined".to_string(),
        };
        match self {
            Line::Expression(e) => format!("value {}", show(eval(e, x, tol))),
            Line::Equation(alts) => alts
                .iter()
                .map(|(l, r)| {
                    format!(
                        "left {}, right {}",
                        show(eval(l, x, tol)),
                        show(eval(r, x, tol))
                    )
                })
                .collect::<Vec<_>>()
                .join("; "),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const T: Tol = Tol {
        rel: 1e-12,
        abs: 1e-15,
    };

    fn at(s: &str, x: f64) -> Option<f64> {
        eval(&parse_expr(s).unwrap(), x, T).map(|n| n.v)
    }

    fn status(s: &str, x: f64) -> Status {
        parse_line(s).unwrap().status(x, T)
    }

    #[test]
    fn evaluator_arithmetic_and_precedence() {
        assert_eq!(at("2 + 3*4", 0.0), Some(14.0));
        assert_eq!(at("(2 + 3)*4", 0.0), Some(20.0));
        assert_eq!(at("10 - 4 - 3", 0.0), Some(3.0));
        assert_eq!(at("12 / 3 / 2", 0.0), Some(2.0));
        assert_eq!(at("2^3", 0.0), Some(8.0));
        assert_eq!(at("x^2 - 5x + 6", 4.0), Some(2.0));
        assert_eq!(at("0.5x", 3.0), Some(1.5));
        // Unary minus binds looser than a power: -x^2 is -(x^2).
        assert_eq!(at("-x^2", 3.0), Some(-9.0));
        assert_eq!(at("(-x)^2", 3.0), Some(9.0));
        assert_eq!(at("2 - -3", 0.0), Some(5.0));
        assert_eq!(at("x^-1", 4.0), Some(0.25));
        assert_eq!(at("x^(-2)", 2.0), Some(0.25));
        assert_eq!(at("x^0", 5.0), Some(1.0));
    }

    #[test]
    fn evaluator_implicit_multiplication() {
        assert_eq!(at("5x", 2.0), Some(10.0));
        assert_eq!(at("2(x - 1)", 4.0), Some(6.0));
        assert_eq!(at("(x - 2)(x - 3)", 5.0), Some(6.0));
        assert_eq!(at("x(x + 1)", 3.0), Some(12.0));
        assert_eq!(at("2x^2", 3.0), Some(18.0));
        assert_eq!(at("3(x - 2)", 5.0), Some(9.0));
        assert_eq!(at("2x(x+1)", 1.0), Some(4.0));
    }

    #[test]
    fn evaluator_typography_and_radicals() {
        assert_eq!(at("x² − 9", 4.0), Some(7.0));
        assert_eq!(at("(x−3)(x+3)", 4.0), Some(7.0));
        assert_eq!(at("x³", 2.0), Some(8.0));
        assert_eq!(at("6 ÷ 3 × 2 · 2", 0.0), Some(8.0));
        let golden = (1.0 + 5f64.sqrt()) / 2.0;
        assert!((at("1/2 + √5/2", 0.0).unwrap() - golden).abs() < 1e-15);
        assert!((at("2√3/3", 0.0).unwrap() - 2.0 * 3f64.sqrt() / 3.0).abs() < 1e-15);
        assert!((at("-√5", 0.0).unwrap() + 5f64.sqrt()).abs() < 1e-15);
        assert_eq!(at("√(0 - 1)", 0.0), None);
    }

    #[test]
    fn evaluator_domain() {
        assert_eq!(at("1/x", 0.0), None);
        assert_eq!(at("x/(x - 1)", 1.0), None);
        // Undefined stays undefined after cancellation would remove it.
        assert_eq!(at("(x^2 - 1)/(x - 1)", 1.0), None);
        assert_eq!(at("(x^2 - 1)/(x - 1)", 2.0), Some(3.0));
        assert_eq!(at("0 * (1/x)", 0.0), None);
        assert_eq!(at("x^-1", 0.0), None);
        // A divisor that is zero only up to rounding counts as zero.
        let r = (1.0 + 5f64.sqrt()) / 2.0;
        assert_eq!(at("1/(x^2 - x - 1)", r), None);
        assert!(at("1/(x^2 - x - 1)", 1.6).is_some());
    }

    #[test]
    fn lines_equations_and_alternatives() {
        assert_eq!(status("x^2 = 5x", 5.0), Status::Holds);
        assert_eq!(status("x^2 = 5x", 0.0), Status::Holds);
        assert_eq!(status("x^2 = 5x", 1.0), Status::Fails);
        assert_eq!(status("x/(x - 1) = 1/(x - 1)", 1.0), Status::Undefined);
        assert_eq!(status("x = 2 or x = 3", 3.0), Status::Holds);
        assert_eq!(status("x = 2 OR x = 3", 2.0), Status::Holds);
        assert_eq!(status("x = 2, x = 3", 3.0), Status::Holds);
        assert_eq!(status("x = 2; x = 3", 4.0), Status::Fails);
        assert_eq!(status("x = ±3", -3.0), Status::Holds);
        assert_eq!(status("x = ±3", 3.0), Status::Holds);
        assert_eq!(status("x = ±3", 0.0), Status::Fails);
        assert_eq!(status("0 = 0", 7.0), Status::Holds);
        let line = parse_line("(x + 1)^2").unwrap();
        assert!(!line.is_equation());
        assert_eq!(line.value(2.0, T).unwrap().v, 9.0);
        let line = parse_line("x^2 = 9").unwrap();
        assert_eq!(line.describe(2.0, T), "left 4e0, right 9e0");
        assert_eq!(line.relative_residual(3.0, T), Some(0.0));
    }

    #[test]
    fn tolerance_is_relative_to_magnitude() {
        // (x - 0.1)(x - 0.2)... evaluated in f64 at 0.1 is not exactly 0
        // after expansion; the magnitude-relative test still calls it 0.
        let line = parse_line("x^2 - 0.3x + 0.02 = 0").unwrap();
        assert_eq!(line.status(0.1, T), Status::Holds);
        assert_eq!(line.status(0.1001, T), Status::Fails);
    }

    #[test]
    fn parse_errors() {
        assert!(parse_line("x + y = 1").is_err());
        assert!(parse_line("x = 1 = 2").is_err());
        assert!(parse_line("x^0.5 = 1").is_err());
        assert!(parse_line("(x + 1 = 2").is_err());
        assert!(parse_line("x + = 1").is_err());
        assert!(parse_line("x = 1 or 2").is_err());
        assert!(parse_line("x = 1 or y = 2").is_err());
        assert!(parse_line("x = ±±3").is_err());
        assert!(parse_line("").is_err());
        assert!(parse_line("x $ 1").is_err());
    }
}
