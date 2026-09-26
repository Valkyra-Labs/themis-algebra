//! A corpus of steps from school algebra, correct and wrong, with the
//! verdict each must get. Every entry is a mistake learners make (or its
//! correct counterpart); the explanation text is pinned too, because it
//! is what a learner reads.

use themis_algebra::{check_step, explain, solve, Solutions, Verdict};

fn roots(v: &[themis_algebra::Root]) -> Vec<String> {
    v.iter().map(|r| r.to_string()).collect()
}

/// (before, after, lost, gained)
const EQUATIONS: &[(&str, &str, &[&str], &[&str])] = &[
    // Taking a square root and forgetting the negative root.
    ("x^2 = 9", "x = 3", &["-3"], &[]),
    // Dividing both sides by x.
    ("x^2 = 5x", "x = 5", &["0"], &[]),
    ("x^3 = x", "x^2 = 1", &["0"], &[]),
    // Squaring both sides adds a root.
    ("x = 3", "x^2 = 9", &[], &["-3"]),
    // Correct moves.
    ("2x + 3 = 7", "2x = 4", &[], &[]),
    ("2x + 3 = 7", "x = 2", &[], &[]),
    ("x^2 - 5x + 6 = 0", "(x - 2)(x - 3) = 0", &[], &[]),
    ("0.5x = 2", "x = 4", &[], &[]),
    ("x - 1 = 0", "(x - 1)^2 = 0", &[], &[]),
    ("x^2 + 1 = 0", "x^2 = -1", &[], &[]),
    ("x² − 9 = 0", "(x−3)(x+3) = 0", &[], &[]),
    // Moving a term without changing its sign.
    ("2x + 3 = 7", "2x = 10", &["2"], &["5"]),
    // Distributing to the first term only.
    ("3(x - 2) = 9", "3x - 2 = 9", &["5"], &["11/3"]),
    // Wrong factorisation.
    ("x^2 - 5x + 6 = 0", "(x - 2)(x + 3) = 0", &["3"], &["-3"]),
    // Irrational roots are named exactly.
    (
        "x^2 - x - 1 = 0",
        "x^2 = x",
        &["1/2 - √5/2", "1/2 + √5/2"],
        &["0", "1"],
    ),
    // Multiplying by an expression that is zero somewhere.
    ("x/(x - 1) = 1/(x - 1)", "x = 1", &[], &["1"]),
    ("(x^2 - 1)/(x - 1) = 2", "x + 1 = 2", &[], &["1"]),
];

#[test]
fn equation_steps_get_the_right_verdict() {
    for (before, after, lost, gained) in EQUATIONS {
        let c = check_step(before, after).unwrap_or_else(|e| panic!("{before} -> {after}: {e}"));
        match &c.verdict {
            Verdict::Equivalent => assert!(
                lost.is_empty() && gained.is_empty(),
                "{before} -> {after}: judged equivalent, expected lost {lost:?} gained {gained:?}"
            ),
            Verdict::Changed {
                lost: l, gained: g, ..
            } => {
                assert_eq!(roots(l), *lost, "{before} -> {after}: lost");
                assert_eq!(roots(g), *gained, "{before} -> {after}: gained");
            }
            other => panic!("{before} -> {after}: unexpected {other:?}"),
        }
    }
}

#[test]
fn explanations_read_as_sentences() {
    let e = |a, b| explain(&check_step(a, b).unwrap());
    assert_eq!(e("x^2 = 9", "x = 3"), "This step loses x = -3.");
    assert_eq!(
        e("x = 3", "x^2 = 9"),
        "This step adds x = -3, which the previous line does not have."
    );
    assert_eq!(
        e("2x + 3 = 7", "2x = 4"),
        "Correct: the step keeps the same solutions."
    );
    assert_eq!(
        e("(x^2 - 1)/(x - 1) = 2", "x + 1 = 2"),
        "This step adds x = 1, which the previous line does not have. The new line is defined at x = 1, where the previous one was not; keep that restriction."
    );
    assert_eq!(
        e("x^3 = 2", "x = 1"),
        "This step loses x ≈ 1.259921; this step adds x = 1, which the previous line does not have."
    );
}

#[test]
fn identities_and_domains() {
    // An identity stays an identity.
    let c = check_step("2(x + 1) = 2x + 2", "0 = 0").unwrap();
    assert_eq!(c.verdict, Verdict::Equivalent);
    // Turning an identity into one value loses infinitely many solutions.
    let c = check_step("2(x + 1) = 2x + 2", "x = 0").unwrap();
    assert!(matches!(
        c.verdict,
        Verdict::Changed {
            lost_infinitely_many: true,
            ..
        }
    ));
    // Clearing a denominator keeps the solution but widens the domain.
    let c = check_step("1/x = 2", "1 = 2x").unwrap();
    assert_eq!(c.verdict, Verdict::Equivalent);
    assert_eq!(roots(&c.domain_widened_at), ["0"]);
}

#[test]
fn expression_steps() {
    let c = check_step("(x + 1)^2", "x^2 + 2x + 1").unwrap();
    assert_eq!(c.verdict, Verdict::Equivalent);
    // The classic: (a + b)^2 = a^2 + b^2.
    let c = check_step("(x + 1)^2", "x^2 + 1").unwrap();
    assert!(matches!(c.verdict, Verdict::NotEqual { .. }));
    assert_eq!(
        explain(&c),
        "The expressions are not equal: at x = 1 the first is 4 and the second is 2."
    );
    // Cancelling a common factor: same function, wider domain.
    let c = check_step("(x^2 - 1)/(x - 1)", "x + 1").unwrap();
    assert_eq!(c.verdict, Verdict::Equivalent);
    assert_eq!(roots(&c.domain_widened_at), ["1"]);
}

#[test]
fn solving_for_an_exercise_audit() {
    let s = solve("x^2 - 5x + 6 = 0").unwrap();
    assert_eq!(roots(&s.roots().unwrap()), ["2", "3"]);
    assert!(solve("x/(x - 1) = 1/(x - 1)")
        .unwrap()
        .roots()
        .unwrap()
        .is_empty());
    assert!(matches!(
        solve("2(x + 1) = 2x + 2").unwrap(),
        Solutions::AllExcept(_)
    ));
}

#[test]
fn mismatched_lines_are_errors() {
    assert!(check_step("x = 1", "x + 1").is_err());
    assert!(check_step("x = 1", "y = 1").is_err());
    assert!(check_step("x = 1", "x +* 1").is_err());
}
