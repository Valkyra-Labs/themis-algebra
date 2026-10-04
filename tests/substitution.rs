//! The substitution check: every verdict the engine gives on the corpus is
//! checked by putting the points it names back into both lines, with an
//! independent floating-point evaluator (`numeric`) that shares no code
//! with the engine.
//!
//! - A lost root satisfies the previous line and not the new one (or the
//!   new one is undefined there).
//! - A gained root satisfies the new line and not the previous one (or the
//!   previous one is undefined there).
//! - For an equivalent step, every root `solve` reports for either line
//!   satisfies both lines; for a changed step, every root of one line that
//!   is not reported lost or gained satisfies the other line too.
//! - A point where the domain widened is undefined on the previous line
//!   and defined on the new one; narrowed, the other way round.
//! - "Infinitely many" (an identity) is checked at fixed sample points:
//!   the identity holds at every sample where it is defined, and the other
//!   line fails at one of them at least.
//! - For expressions: equal values at the sample points when judged
//!   equivalent; the witness values when judged not equal.
//! - The roots the corpus table states, and the printed form of every root
//!   the engine reports, are read back by the evaluator and substituted
//!   or compared too.
//!
//! This checks the points the engine names. It does not prove that no
//! other root exists.

mod common;
mod numeric;

use common::EQUATIONS;
use num_traits::ToPrimitive;
use numeric::{parse_expr, parse_line, Line, Status, Tol};
use themis_algebra::{check_step, solve, Root, Solutions, Verdict, Q};

/// Exact roots (rational, or a quadratic surd computed with one `sqrt`)
/// are known to the last bit or two of an f64. The corpus lines have
/// degree at most four and small coefficients, so evaluating them at such
/// a point loses a few hundred ulps at most, relative to the magnitude of
/// the terms: about 1e-13. 1e-9 leaves four orders of margin above that,
/// and stays many orders below the relative residual of a point that is
/// not a root (the test prints the measured extremes of both).
const EXACT: Tol = Tol {
    rel: 1e-9,
    abs: 1e-12,
};

/// Approximate roots (`x ≈ ...`) are refined by the engine in f64, so the
/// residual there includes the refinement error times the slope; 1e-6
/// relative is the stated allowance.
const APPROX: Tol = Tol {
    rel: 1e-6,
    abs: 1e-9,
};

/// Printed approximate roots carry six decimals: a rounding error of at
/// most 5e-7, allowed 1e-6.
const PRINTED_APPROX: f64 = 1e-6;

/// Points for identities and expression equivalence: arbitrary values
/// with many digits, away from the small integers and halves where the
/// corpus lines have their roots and poles.
const SAMPLES: [f64; 8] = [
    -2.837_461_9,
    -1.372_915_3,
    -0.581_273_6,
    0.291_748_3,
    0.631_195_7,
    1.733_482_1,
    3.046_719_2,
    5.385_164_8,
];

struct Point {
    x: f64,
    tol: Tol,
    /// Exact (rational or surd) rather than approximate.
    exact: bool,
    label: String,
}

#[derive(Clone, Copy, Debug)]
enum Want {
    Holds,
    DoesNotHold,
    Undefined,
    Defined,
}

#[derive(Default)]
struct Report {
    failures: Vec<String>,
    steps: usize,
    engine_errors: Vec<String>,
    /// (step, point) pairs substituted; a root that `solve` reports for
    /// both lines of a step counts once per line it came from.
    points: usize,
    /// Line evaluations at those points.
    substitutions: usize,
    /// Largest relative residual among exact points required to hold.
    worst_hold: f64,
    /// The same among approximate points.
    worst_hold_approx: f64,
    /// Smallest relative residual among exact points required not to
    /// hold where the line is defined.
    closest_fail: Option<f64>,
}

struct Step<'a> {
    name: String,
    before: &'a str,
    after: &'a str,
    b: Line,
    a: Line,
}

fn to_f64(v: &Q) -> f64 {
    v.to_f64()
        .filter(|x| x.is_finite())
        .unwrap_or_else(|| panic!("{v} has no finite f64 value"))
}

fn close(a: f64, b: f64, rel: f64) -> bool {
    (a - b).abs() <= rel * a.abs().max(b.abs()).max(1.0)
}

impl Report {
    fn fail(&mut self, msg: String) {
        self.failures.push(msg);
    }

    fn print_margins(&self) {
        eprintln!(
            "  relative residual |left - right| / magnitude: largest where a root must hold \
             {:e} (exact points), {:e} (approximate points); smallest where an exact point \
             must not hold {:e}",
            self.worst_hold,
            self.worst_hold_approx,
            self.closest_fail.unwrap_or(f64::NAN)
        );
    }

    /// A root the engine reports, as a point; its printed form must name
    /// the same number.
    fn root_point(&mut self, step: &str, r: &Root) -> Point {
        let (x, tol) = match r {
            Root::Rational(v) => (to_f64(v), EXACT),
            Root::Quadratic { p, k, d } => {
                let d = to_f64(d);
                assert!(d > 0.0, "{step}: surd with d = {d}");
                (to_f64(p) + to_f64(k) * d.sqrt(), EXACT)
            }
            Root::Approx(x) => (*x, APPROX),
        };
        let text = r.to_string();
        let (printed, allowed) = match text.strip_prefix("≈ ") {
            Some(t) => (t, PRINTED_APPROX),
            None => (text.as_str(), EXACT.rel),
        };
        match parse_expr(printed).map(|e| numeric::eval(&e, 0.0, EXACT)) {
            Ok(Some(v)) if close(v.v, x, allowed) => {}
            other => self.fail(format!(
                "{step}: root printed as `{text}` reads back as {other:?}, but its value is {x:e}"
            )),
        }
        Point {
            x,
            tol,
            exact: !matches!(r, Root::Approx(_)),
            label: format!("x = {text}"),
        }
    }

    fn expect(&mut self, step: &Step, role: &str, after_line: bool, p: &Point, want: Want) {
        let (line, text, which) = if after_line {
            (&step.a, step.after, "new")
        } else {
            (&step.b, step.before, "previous")
        };
        self.substitutions += 1;
        let status = line.status(p.x, p.tol);
        let ok = match want {
            Want::Holds => status == Status::Holds,
            Want::DoesNotHold => status != Status::Holds,
            Want::Undefined => status == Status::Undefined,
            Want::Defined => status != Status::Undefined,
        };
        // The measured margins on both sides of the tolerance.
        if let Some(r) = line.relative_residual(p.x, p.tol) {
            match want {
                Want::Holds if p.exact => self.worst_hold = self.worst_hold.max(r),
                Want::Holds => self.worst_hold_approx = self.worst_hold_approx.max(r),
                Want::DoesNotHold if p.exact && status == Status::Fails => {
                    self.closest_fail = Some(self.closest_fail.map_or(r, |c| c.min(r)))
                }
                _ => {}
            }
        }
        if !ok {
            self.fail(format!(
                "{}: {role} {} (x ≈ {:e}) should be {want:?} on the {which} line `{text}`, \
                 but is {status:?}. Previous line: {}. New line: {}.",
                step.name,
                p.label,
                p.x,
                step.b.describe(p.x, p.tol),
                step.a.describe(p.x, p.tol),
            ));
        }
    }

    /// The line holds at every sample where it is defined (and is defined
    /// at one sample at least).
    fn expect_identity(&mut self, step: &Step, after_line: bool, why: &str) {
        let (line, text) = if after_line {
            (&step.a, step.after)
        } else {
            (&step.b, step.before)
        };
        let mut defined = 0;
        for &x in &SAMPLES {
            self.substitutions += 1;
            match line.status(x, EXACT) {
                Status::Holds => defined += 1,
                Status::Undefined => {}
                Status::Fails => {
                    self.fail(format!(
                        "{}: {why}, so `{text}` should hold for every x of its domain, \
                         but fails at x = {x}: {}",
                        step.name,
                        line.describe(x, EXACT)
                    ));
                    return;
                }
            }
        }
        if defined == 0 {
            self.fail(format!(
                "{}: `{text}` is undefined at every sample point",
                step.name
            ));
        }
    }

    /// The line fails at one sample at least.
    fn expect_not_identity(&mut self, step: &Step, after_line: bool, why: &str) {
        let (line, text) = if after_line {
            (&step.a, step.after)
        } else {
            (&step.b, step.before)
        };
        self.substitutions += SAMPLES.len();
        if !SAMPLES
            .iter()
            .any(|&x| line.status(x, EXACT) == Status::Fails)
        {
            self.fail(format!(
                "{}: {why}, so `{text}` should fail somewhere, but it holds or is undefined \
                 at every sample point",
                step.name
            ));
        }
    }

    /// The roots `solve` reports for one line (checked against its
    /// classification), or `None` for an identity.
    fn solved(&mut self, step: &Step, after_line: bool) -> Option<Vec<Point>> {
        let text = if after_line { step.after } else { step.before };
        match solve(text).unwrap_or_else(|e| panic!("{}: solve `{text}`: {e}", step.name)) {
            Solutions::AllExcept(_) => {
                self.expect_identity(step, after_line, "solve reports every x of the domain");
                None
            }
            s @ Solutions::Finite(_) => {
                self.expect_not_identity(step, after_line, "solve reports finitely many roots");
                let roots = s.roots().expect("finite");
                Some(
                    roots
                        .iter()
                        .map(|r| self.root_point(&step.name, r))
                        .collect(),
                )
            }
        }
    }
}

fn near_any(p: &Point, others: &[Point]) -> bool {
    others
        .iter()
        .any(|o| close(p.x, o.x, p.tol.rel.max(o.tol.rel)))
}

/// Check one step. Returns false when the engine rejects it.
fn check(report: &mut Report, name: String, before: &str, after: &str) -> bool {
    let c = match check_step(before, after) {
        Ok(c) => c,
        Err(e) => {
            report
                .engine_errors
                .push(format!("{name}: `{before}` -> `{after}`: {e}"));
            return false;
        }
    };
    let parse = |s: &str| parse_line(s).unwrap_or_else(|e| panic!("{name}: evaluator: {e}"));
    let step = Step {
        name: format!("{name} (`{before}` -> `{after}`)"),
        before,
        after,
        b: parse(before),
        a: parse(after),
    };
    report.steps += 1;
    let is_eq = step.b.is_equation();
    if is_eq != step.a.is_equation() {
        report.fail(format!(
            "{}: the engine accepted the step, the evaluator reads one equation and one expression",
            step.name
        ));
        return true;
    }

    match &c.verdict {
        Verdict::Equivalent if is_eq => {
            let rb = report.solved(&step, false);
            let ra = report.solved(&step, true);
            if rb.is_some() != ra.is_some() {
                report.fail(format!(
                    "{}: judged equivalent, but one line is an identity and the other is not",
                    step.name
                ));
            }
            for p in rb.iter().chain(ra.iter()).flatten() {
                report.points += 1;
                report.expect(&step, "root", false, p, Want::Holds);
                report.expect(&step, "root", true, p, Want::Holds);
            }
        }
        Verdict::Changed {
            lost,
            gained,
            lost_infinitely_many,
            gained_infinitely_many,
        } => {
            let lost: Vec<Point> = lost
                .iter()
                .map(|r| report.root_point(&step.name, r))
                .collect();
            let gained: Vec<Point> = gained
                .iter()
                .map(|r| report.root_point(&step.name, r))
                .collect();
            for p in &lost {
                report.points += 1;
                report.expect(&step, "lost root", false, p, Want::Holds);
                report.expect(&step, "lost root", true, p, Want::DoesNotHold);
            }
            for p in &gained {
                report.points += 1;
                report.expect(&step, "gained root", true, p, Want::Holds);
                report.expect(&step, "gained root", false, p, Want::DoesNotHold);
            }
            if *lost_infinitely_many {
                report.expect_identity(&step, false, "the step is said to lose infinitely many");
                report.expect_not_identity(&step, true, "the step is said to lose infinitely many");
            }
            if *gained_infinitely_many {
                report.expect_identity(&step, true, "the step is said to gain infinitely many");
                report.expect_not_identity(
                    &step,
                    false,
                    "the step is said to gain infinitely many",
                );
            }
            // Roots of one line not reported as changed must be roots of
            // the other.
            if let Some(rb) = report.solved(&step, false) {
                for p in rb.iter().filter(|p| !near_any(p, &lost)) {
                    report.points += 1;
                    report.expect(&step, "kept root", true, p, Want::Holds);
                }
            }
            if let Some(ra) = report.solved(&step, true) {
                for p in ra.iter().filter(|p| !near_any(p, &gained)) {
                    report.points += 1;
                    report.expect(&step, "kept root", false, p, Want::Holds);
                }
            }
        }
        Verdict::Equivalent => {
            // Expressions: the same value wherever both are defined.
            let mut compared = 0;
            for &x in &SAMPLES {
                report.substitutions += 2;
                if let (Some(vb), Some(va)) = (step.b.value(x, EXACT), step.a.value(x, EXACT)) {
                    compared += 1;
                    let diff = numeric::Num {
                        v: vb.v - va.v,
                        mag: vb.mag + va.mag,
                    };
                    if !EXACT.is_zero(diff) {
                        report.fail(format!(
                            "{}: judged the same function, but at x = {x} the values are {:e} and {:e}",
                            step.name, vb.v, va.v
                        ));
                    }
                }
            }
            if compared == 0 {
                report.fail(format!(
                    "{}: no sample point where both expressions are defined",
                    step.name
                ));
            }
        }
        Verdict::NotEqual {
            witness,
            before: vb,
            after: va,
        } => {
            let x = to_f64(witness);
            report.points += 1;
            report.substitutions += 2;
            let got_b = step.b.value(x, EXACT);
            let got_a = step.a.value(x, EXACT);
            let matches = |got: Option<numeric::Num>, want: &Q| {
                got.is_some_and(|g| {
                    EXACT.is_zero(numeric::Num {
                        v: g.v - to_f64(want),
                        mag: g.mag + to_f64(want).abs(),
                    })
                })
            };
            let differ = match (got_b, got_a) {
                (Some(b), Some(a)) => !EXACT.is_zero(numeric::Num {
                    v: b.v - a.v,
                    mag: b.mag + a.mag,
                }),
                _ => false,
            };
            if !matches(got_b, vb) || !matches(got_a, va) || !differ {
                report.fail(format!(
                    "{}: witness x = {witness}: engine values {vb} and {va}, evaluator {} and {}",
                    step.name,
                    step.b.describe(x, EXACT),
                    step.a.describe(x, EXACT)
                ));
            }
        }
    }

    for r in &c.domain_widened_at {
        let p = report.root_point(&step.name, r);
        report.points += 1;
        report.expect(&step, "domain widened at", false, &p, Want::Undefined);
        report.expect(&step, "domain widened at", true, &p, Want::Defined);
    }
    for r in &c.domain_narrowed_at {
        let p = report.root_point(&step.name, r);
        report.points += 1;
        report.expect(&step, "domain narrowed at", true, &p, Want::Undefined);
        report.expect(&step, "domain narrowed at", false, &p, Want::Defined);
    }
    true
}

/// A root as the corpus states it (`-3`, `1/2 + √5/2`, `≈ 1.259921`).
fn stated_point(step: &str, text: &str) -> Point {
    let (body, tol) = match text.strip_prefix("≈ ") {
        Some(t) => (t, APPROX),
        None => (text, EXACT),
    };
    let x = parse_expr(body)
        .ok()
        .and_then(|e| numeric::eval(&e, 0.0, EXACT))
        .unwrap_or_else(|| panic!("{step}: cannot read the stated root `{text}`"))
        .v;
    Point {
        x,
        tol,
        exact: !text.starts_with('≈'),
        label: format!("x = {text} (stated by the corpus)"),
    }
}

/// The steps `corpus.rs` checks inline, outside the shared table: every
/// `check_step("a", "b")` and `e("a", "b")` call with two string
/// literals. Reading them from the source keeps this list from drifting.
fn inline_corpus_steps() -> Vec<(String, String)> {
    let src = include_str!("corpus.rs");
    let literal = |s: &str| -> Option<(String, usize)> {
        let s0 = s.trim_start();
        let skipped = s.len() - s0.len();
        let rest = s0.strip_prefix('"')?;
        let end = rest.find('"')?;
        let lit = &rest[..end];
        assert!(!lit.contains('\\'), "escapes in corpus literals: {lit}");
        Some((lit.to_string(), skipped + 1 + end + 1))
    };
    let mut out = Vec::new();
    for (i, _) in src.match_indices('(') {
        let ident: String = src[..i]
            .chars()
            .rev()
            .take_while(|c| c.is_alphanumeric() || *c == '_')
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();
        if ident != "check_step" && ident != "e" {
            continue;
        }
        let rest = &src[i + 1..];
        let Some((a, n)) = literal(rest) else {
            continue;
        };
        let Some(rest) = rest[n..].trim_start().strip_prefix(',') else {
            continue;
        };
        let Some((b, n)) = literal(rest) else {
            continue;
        };
        if rest[n..].trim_start().starts_with(')') {
            out.push((a, b));
        }
    }
    out
}

/// A small deterministic generator (xorshift64*).
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_f491_4f6c_dd1d)
    }
    fn range(&mut self, lo: i64, hi: i64) -> i64 {
        lo + (self.next() % (hi - lo + 1) as u64) as i64
    }
}

/// The factor `a·x − b` with root `b/a`.
#[derive(Clone, Copy, PartialEq)]
struct Factor {
    a: i64,
    b: i64,
}

impl Factor {
    fn text(&self) -> String {
        let ax = if self.a == 1 {
            "x".to_string()
        } else {
            format!("{}x", self.a)
        };
        match self.b {
            0 => format!("({ax})"),
            b if b > 0 => format!("({ax} - {b})"),
            b => format!("({ax} + {})", -b),
        }
    }
    /// The root as a reduced fraction with a positive denominator.
    fn root(&self) -> (i64, i64) {
        let g = gcd(self.b.abs(), self.a);
        (self.b / g, self.a / g)
    }
}

fn gcd(a: i64, b: i64) -> i64 {
    if b == 0 {
        a.max(1)
    } else {
        gcd(b, a % b)
    }
}

fn product_line(fs: &[Factor]) -> String {
    fs.iter().map(Factor::text).collect::<String>() + " = 0"
}

fn root_set(fs: &[Factor]) -> Vec<(i64, i64)> {
    let mut v: Vec<(i64, i64)> = fs.iter().map(Factor::root).collect();
    v.sort_by(|x, y| (x.0 * y.1).cmp(&(y.0 * x.1)));
    v.dedup();
    v
}

fn minus(a: &[(i64, i64)], b: &[(i64, i64)]) -> Vec<(i64, i64)> {
    a.iter().filter(|r| !b.contains(r)).copied().collect()
}

fn as_fractions(step: &str, roots: &[Root]) -> Vec<(i64, i64)> {
    roots
        .iter()
        .map(|r| match r {
            Root::Rational(v) => (
                v.numer().to_i64().expect("small"),
                v.denom().to_i64().expect("small"),
            ),
            other => panic!("{step}: expected a rational root, got {other:?}"),
        })
        .collect()
}

#[test]
fn engine_verdicts_survive_substitution() {
    let mut report = Report::default();
    let mut seen: Vec<(String, String)> = Vec::new();

    for (i, (before, after, lost, gained)) in EQUATIONS.iter().enumerate() {
        seen.push((before.to_string(), after.to_string()));
        let name = format!("corpus table entry {}", i + 1);
        if !check(&mut report, name.clone(), before, after) {
            report.fail(format!("{name}: the engine rejects a table step"));
            continue;
        }
        // The roots the table states, checked without the engine at all.
        let step = Step {
            name: format!("{name} (`{before}` -> `{after}`)"),
            before,
            after,
            b: parse_line(before).expect("parsed above"),
            a: parse_line(after).expect("parsed above"),
        };
        for text in lost.iter() {
            let p = stated_point(&step.name, text);
            report.points += 1;
            report.expect(&step, "stated lost root", false, &p, Want::Holds);
            report.expect(&step, "stated lost root", true, &p, Want::DoesNotHold);
        }
        for text in gained.iter() {
            let p = stated_point(&step.name, text);
            report.points += 1;
            report.expect(&step, "stated gained root", true, &p, Want::Holds);
            report.expect(&step, "stated gained root", false, &p, Want::DoesNotHold);
        }
    }

    let inline = inline_corpus_steps();
    assert!(
        inline.len() >= 10,
        "found only {} inline steps in corpus.rs; has its layout changed?",
        inline.len()
    );
    let mut inline_new = 0;
    for (before, after) in inline {
        if seen.contains(&(before.clone(), after.clone())) {
            continue;
        }
        seen.push((before.clone(), after.clone()));
        inline_new += 1;
        check(&mut report, "corpus inline step".into(), &before, &after);
    }

    eprintln!(
        "substitution: {} distinct corpus steps ({} in the table, {} more inline), {} checked, \
         {} rejected by the engine as errors; {} points substituted, {} line evaluations",
        EQUATIONS.len() + inline_new,
        EQUATIONS.len(),
        inline_new,
        report.steps,
        report.engine_errors.len(),
        report.points,
        report.substitutions
    );
    for e in &report.engine_errors {
        eprintln!("  rejected: {e}");
    }
    report.print_margins();
    // The corpus's error cases are the only steps the engine may reject.
    assert!(
        report
            .engine_errors
            .iter()
            .all(|e| e.contains("different unknowns")
                || e.contains("equation and the other")
                || e.contains("line 2")),
        "unexpected engine errors: {:#?}",
        report.engine_errors
    );
    assert!(
        report.failures.is_empty(),
        "{} substitution failures:\n{}",
        report.failures.len(),
        report.failures.join("\n")
    );
}

#[test]
fn generated_factorised_steps_survive_substitution() {
    let mut rng = Rng(0x7e5e_ed00_2026_1004);
    let mut report = Report::default();
    let mut mismatches = Vec::new();
    for n in 0..300 {
        let k = rng.range(2, 4) as usize;
        let mut fs: Vec<Factor> = (0..k)
            .map(|_| Factor {
                a: rng.range(1, 3),
                b: rng.range(-6, 6),
            })
            .collect();
        let before = product_line(&fs);
        let original = fs.clone();
        let op = rng.range(0, 3);
        let kind = match op {
            // Dividing both sides by a factor.
            0 => {
                fs.remove(rng.range(0, k as i64 - 1) as usize);
                "drop a factor"
            }
            // Multiplying both sides by a new factor.
            1 => {
                fs.push(Factor {
                    a: rng.range(1, 3),
                    b: rng.range(-6, 6),
                });
                "add a factor"
            }
            // A sign slip inside one factor.
            2 => {
                let i = rng.range(0, k as i64 - 1) as usize;
                fs[i].b = -fs[i].b;
                "flip a sign"
            }
            // Reordering the factors: a correct step.
            _ => {
                fs.reverse();
                "reorder"
            }
        };
        let after = product_line(&fs);
        let name = format!("generated step {n} ({kind})");
        if !check(&mut report, name.clone(), &before, &after) {
            report.fail(format!("{name}: rejected by the engine"));
            continue;
        }
        // The verdict known by construction, from the factors alone.
        let rb = root_set(&original);
        let ra = root_set(&fs);
        let (want_lost, want_gained) = (minus(&rb, &ra), minus(&ra, &rb));
        let c = check_step(&before, &after).expect("checked above");
        let (got_lost, got_gained) = match &c.verdict {
            Verdict::Equivalent => (vec![], vec![]),
            Verdict::Changed { lost, gained, .. } => {
                (as_fractions(&name, lost), as_fractions(&name, gained))
            }
            other => panic!("{name}: {other:?}"),
        };
        let sorted = |mut v: Vec<(i64, i64)>| {
            v.sort_by(|x, y| (x.0 * y.1).cmp(&(y.0 * x.1)));
            v
        };
        let (got_lost, got_gained) = (sorted(got_lost), sorted(got_gained));
        if got_lost != want_lost || got_gained != want_gained {
            mismatches.push(format!(
                "{name}: `{before}` -> `{after}`: engine lost {got_lost:?} gained {got_gained:?}, \
                 by construction lost {want_lost:?} gained {want_gained:?}"
            ));
        }
    }
    eprintln!(
        "substitution: {} generated steps, {} points, {} line evaluations",
        report.steps, report.points, report.substitutions
    );
    report.print_margins();
    assert!(
        report.failures.is_empty() && mismatches.is_empty(),
        "{} substitution failures, {} construction mismatches:\n{}\n{}",
        report.failures.len(),
        mismatches.len(),
        report.failures.join("\n"),
        mismatches.join("\n")
    );
}
