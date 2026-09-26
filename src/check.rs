//! Solution sets and step verdicts.
//!
//! An equation's solution set is the real roots of its numerator, minus
//! the points where it is undefined; or, when the numerator is the zero
//! polynomial, every real number of its domain. Two consecutive lines are
//! compared as sets: a correct step keeps the set; a wrong one loses or
//! gains roots, and the verdict names them exactly.

use crate::expr::{parse_line, Line, ParseError, Rational};
use crate::poly::{fmt_q, q, Poly, Root, Q};
use num_traits::Zero;

/// The real solutions of an equation.
#[derive(Clone, Debug, PartialEq)]
pub enum Solutions {
    /// The real roots of this squarefree polynomial (possibly none).
    Finite(Poly),
    /// Every real number except the real roots of `excluded`.
    AllExcept(Poly),
}

impl Solutions {
    pub fn of(r: &Rational) -> Solutions {
        if r.num.is_zero() {
            return Solutions::AllExcept(r.excluded.clone());
        }
        if r.num.degree() == Some(0) {
            return Solutions::Finite(Poly::one());
        }
        let sf = r.num.squarefree();
        let ex = r.excluded.clone();
        let g = sf.gcd(&ex);
        Solutions::Finite(sf.divrem(&g).0.monic())
    }

    /// The roots, exact where possible; `None` for infinitely many.
    pub fn roots(&self) -> Option<Vec<Root>> {
        match self {
            Solutions::Finite(p) => Some(p.real_roots()),
            Solutions::AllExcept(_) => None,
        }
    }
}

/// What changed between two lines.
#[derive(Clone, Debug, PartialEq)]
pub enum Verdict {
    /// Same solution set (equations) or the same function on the common
    /// domain (expressions).
    Equivalent,
    /// Equations whose solution sets differ.
    Changed {
        lost: Vec<Root>,
        gained: Vec<Root>,
        /// The first line holds for every x of its domain; the second
        /// does not.
        lost_infinitely_many: bool,
        gained_infinitely_many: bool,
    },
    /// Expressions that are different functions; `witness` is an x where
    /// both are defined and their values differ.
    NotEqual { witness: Q, before: Q, after: Q },
}

/// A step check, with any change of domain reported separately (a step
/// can keep the solutions and still widen the domain, for example by
/// cancelling a factor).
#[derive(Clone, Debug, PartialEq)]
pub struct StepCheck {
    pub verdict: Verdict,
    /// Points where the first line was undefined and the second is not.
    pub domain_widened_at: Vec<Root>,
    /// Points where the second line is undefined and the first was not.
    pub domain_narrowed_at: Vec<Root>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CheckError {
    Parse {
        line: usize,
        error: ParseError,
    },
    /// One line is an equation and the other an expression.
    KindMismatch,
    /// The lines use different letters for the unknown.
    DifferentUnknowns(char, char),
}

impl std::fmt::Display for CheckError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CheckError::Parse { line, error } => write!(f, "line {line}: {error}"),
            CheckError::KindMismatch => {
                write!(f, "one line is an equation and the other is an expression")
            }
            CheckError::DifferentUnknowns(a, b) => {
                write!(f, "the lines use different unknowns ({a} and {b})")
            }
        }
    }
}

impl std::error::Error for CheckError {}

fn roots_of_quotient(a: &Poly, g: &Poly) -> Vec<Root> {
    a.divrem(g).0.real_roots()
}

fn domain_diff(before: &Poly, after: &Poly) -> (Vec<Root>, Vec<Root>) {
    let g = before.gcd(after);
    let widened = if before.degree().unwrap_or(0) > 0 {
        roots_of_quotient(before, &g)
    } else {
        vec![]
    };
    let narrowed = if after.degree().unwrap_or(0) > 0 {
        roots_of_quotient(after, &g)
    } else {
        vec![]
    };
    (widened, narrowed)
}

fn compare_solutions(a: &Solutions, b: &Solutions) -> Verdict {
    match (a, b) {
        (Solutions::Finite(p), Solutions::Finite(r)) => {
            let g = p.gcd(r);
            let lost = if p.degree().unwrap_or(0) > 0 {
                roots_of_quotient(p, &g)
            } else {
                vec![]
            };
            let gained = if r.degree().unwrap_or(0) > 0 {
                roots_of_quotient(r, &g)
            } else {
                vec![]
            };
            if lost.is_empty() && gained.is_empty() {
                Verdict::Equivalent
            } else {
                Verdict::Changed {
                    lost,
                    gained,
                    lost_infinitely_many: false,
                    gained_infinitely_many: false,
                }
            }
        }
        (Solutions::AllExcept(e1), Solutions::AllExcept(e2)) => {
            // Points newly excluded are lost solutions; points no longer
            // excluded are gained ones.
            let (gained, lost) = domain_diff(e1, e2);
            if lost.is_empty() && gained.is_empty() {
                Verdict::Equivalent
            } else {
                Verdict::Changed {
                    lost,
                    gained,
                    lost_infinitely_many: false,
                    gained_infinitely_many: false,
                }
            }
        }
        (Solutions::AllExcept(_), Solutions::Finite(_)) => Verdict::Changed {
            lost: vec![],
            gained: vec![],
            lost_infinitely_many: true,
            gained_infinitely_many: false,
        },
        (Solutions::Finite(_), Solutions::AllExcept(_)) => Verdict::Changed {
            lost: vec![],
            gained: vec![],
            lost_infinitely_many: false,
            gained_infinitely_many: true,
        },
    }
}

/// A rational x where both polynomials are nonzero (small integers first,
/// then halves).
fn defined_point(a: &Poly, b: &Poly, differ: &Poly) -> Option<Q> {
    let candidates = (0..40i64).flat_map(|n| [q(n), q(-n)]).chain(
        (1..40i64).flat_map(|n| [Q::new(n.into(), 2.into()), Q::new((-n).into(), 2.into())]),
    );
    candidates
        .into_iter()
        .find(|x| !a.eval(x).is_zero() && !b.eval(x).is_zero() && !differ.eval(x).is_zero())
}

fn eval(r: &Rational, x: &Q) -> Q {
    r.num.eval(x) / r.den.eval(x)
}

/// Check the step from `before` to `after`.
pub fn check_step(before: &str, after: &str) -> Result<StepCheck, CheckError> {
    let (a, va) = parse_line(before).map_err(|error| CheckError::Parse { line: 1, error })?;
    let (b, vb) = parse_line(after).map_err(|error| CheckError::Parse { line: 2, error })?;
    if let (Some(x), Some(y)) = (va, vb) {
        if x != y {
            return Err(CheckError::DifferentUnknowns(x, y));
        }
    }
    let (ra, rb, is_eq) = match (a, b) {
        (Line::Equation(x), Line::Equation(y)) => (x, y, true),
        (Line::Expression(x), Line::Expression(y)) => (x, y, false),
        _ => return Err(CheckError::KindMismatch),
    };
    let (domain_widened_at, domain_narrowed_at) = domain_diff(&ra.excluded, &rb.excluded);
    let verdict = if is_eq {
        compare_solutions(&Solutions::of(&ra), &Solutions::of(&rb))
    } else {
        let diff = ra.sub(&rb);
        if diff.is_zero() {
            Verdict::Equivalent
        } else {
            let x = defined_point(&ra.excluded, &rb.excluded, &diff.num).unwrap_or_else(|| q(1000));
            Verdict::NotEqual {
                witness: x.clone(),
                before: eval(&ra, &x),
                after: eval(&rb, &x),
            }
        }
    };
    Ok(StepCheck {
        verdict,
        domain_widened_at,
        domain_narrowed_at,
    })
}

/// Solve one equation (for auditing exercises): the solution set.
pub fn solve(line: &str) -> Result<Solutions, CheckError> {
    match parse_line(line)
        .map_err(|error| CheckError::Parse { line: 1, error })?
        .0
    {
        Line::Equation(r) => Ok(Solutions::of(&r)),
        Line::Expression(_) => Err(CheckError::KindMismatch),
    }
}

/// A short plain-English explanation of a step check.
pub fn explain(c: &StepCheck) -> String {
    let list = |v: &[Root]| {
        v.iter()
            .map(|r| match r {
                Root::Approx(x) => format!("x ≈ {x:.6}"),
                _ => format!("x = {r}"),
            })
            .collect::<Vec<_>>()
            .join(", ")
    };
    let mut out = match &c.verdict {
        Verdict::Equivalent => "Correct: the step keeps the same solutions.".to_string(),
        Verdict::Changed {
            lost,
            gained,
            lost_infinitely_many,
            gained_infinitely_many,
        } => {
            let mut parts = Vec::new();
            if *lost_infinitely_many {
                parts.push(
                    "the first line holds for every x in its domain, the second does not"
                        .to_string(),
                );
            }
            if *gained_infinitely_many {
                parts.push(
                    "the second line holds for every x in its domain, the first does not"
                        .to_string(),
                );
            }
            if !lost.is_empty() {
                parts.push(format!("this step loses {}", list(lost)));
            }
            if !gained.is_empty() {
                parts.push(format!(
                    "this step adds {}, which the previous line does not have",
                    list(gained)
                ));
            }
            let mut s = parts.join("; ");
            if let Some(first) = s.get(..1) {
                s = first.to_uppercase() + &s[1..];
            }
            s + "."
        }
        Verdict::NotEqual {
            witness,
            before,
            after,
        } => format!(
            "The expressions are not equal: at x = {} the first is {} and the second is {}.",
            fmt_q(witness),
            fmt_q(before),
            fmt_q(after)
        ),
    };
    if !c.domain_widened_at.is_empty() {
        out.push_str(&format!(" The new line is defined at {}, where the previous one was not; keep that restriction.", list(&c.domain_widened_at)));
    }
    if !c.domain_narrowed_at.is_empty() {
        out.push_str(&format!(
            " The new line is undefined at {}.",
            list(&c.domain_narrowed_at)
        ));
    }
    out
}
