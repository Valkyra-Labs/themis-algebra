//! The table of equation steps from the corpus, shared by `corpus.rs`
//! (which pins each verdict) and `substitution.rs` (which checks each
//! verdict by substituting the named roots into both lines).

/// (before, after, lost, gained)
pub const EQUATIONS: &[(&str, &str, &[&str], &[&str])] = &[
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
    // Answer lines with alternatives.
    ("x^2 = 9", "x = ±3", &[], &[]),
    ("x^2 = 9", "x = 3 or x = -3", &[], &[]),
    ("x^2 - 5x + 6 = 0", "x = 2, x = 3", &[], &[]),
    ("x^2 - 5x + 6 = 0", "x = 2 or x = -3", &["3"], &["-3"]),
    // Multiplying by an expression that is zero somewhere.
    ("x/(x - 1) = 1/(x - 1)", "x = 1", &[], &["1"]),
    ("(x^2 - 1)/(x - 1) = 2", "x + 1 = 2", &[], &["1"]),
];
