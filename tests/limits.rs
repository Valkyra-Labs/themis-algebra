//! Lines a learner can type or paste that are far beyond school algebra:
//! very long, deeply nested, of a huge degree or with many alternatives.
//! Each gets an error naming the limit it is over, quickly; none overflows
//! the stack or runs for minutes.

use std::time::{Duration, Instant};
use themis_algebra::{
    check_step, parse_line, CheckError, ParseError, MAX_ALTERNATIVES, MAX_DEGREE, MAX_DEPTH,
    MAX_LINE_CHARS,
};

/// The error a line gets as the second line of a step.
fn error_of(line: &str) -> ParseError {
    match check_step("x = 1", line) {
        Err(CheckError::Parse { line: 2, error }) => error,
        other => panic!("{line:.60}: expected a parse error on line 2, got {other:?}"),
    }
}

#[test]
fn a_line_longer_than_the_limit_is_not_read() {
    let long = format!("x = {}1", "1 + ".repeat(MAX_LINE_CHARS / 4));
    assert!(long.chars().count() > MAX_LINE_CHARS);
    assert_eq!(
        error_of(&long),
        ParseError::TooLong {
            max: MAX_LINE_CHARS
        }
    );
    // Characters, not bytes: a line of typographic signs at the limit is read.
    let at_limit = format!("x = {}", "−".repeat(MAX_LINE_CHARS - 5) + "1");
    assert_eq!(at_limit.chars().count(), MAX_LINE_CHARS);
    assert!(at_limit.len() > MAX_LINE_CHARS);
    assert!(!matches!(
        parse_line(&at_limit),
        Err(ParseError::TooLong { .. })
    ));
}

#[test]
fn nesting_deeper_than_the_limit_is_an_error_not_a_stack_overflow() {
    let parens = |n: usize| format!("{}x{} = 1", "(".repeat(n), ")".repeat(n));
    assert!(parse_line(&parens(MAX_DEPTH)).is_ok());
    assert_eq!(
        error_of(&parens(MAX_DEPTH + 1)),
        ParseError::TooDeep { max: MAX_DEPTH }
    );
    // Signs nest too: each one applies to everything after it.
    let signs = |n: usize| format!("{}x = 1", "-".repeat(n));
    assert!(parse_line(&signs(MAX_DEPTH)).is_ok());
    assert_eq!(
        error_of(&signs(MAX_DEPTH + 1)),
        ParseError::TooDeep { max: MAX_DEPTH }
    );
    // The longest line there can be, all brackets.
    let half = MAX_LINE_CHARS / 2 - 3;
    assert_eq!(
        error_of(&parens(half)),
        ParseError::TooDeep { max: MAX_DEPTH }
    );
}

#[test]
fn a_degree_above_the_limit_is_an_error() {
    assert!(parse_line(&format!("x^{MAX_DEGREE} = 1")).is_ok());
    let over = ParseError::TooComplex { max: MAX_DEGREE };
    assert_eq!(error_of(&format!("x^{MAX_DEGREE} * x = 1")), over);
    assert_eq!(error_of("((x+1)^8)^9 = 0"), over);
    // A denominator counts as well as a numerator.
    assert_eq!(error_of(&format!("1/(x^{MAX_DEGREE} * x) = 1")), over);
    // Each power of a power is refused before it is computed: this one
    // has degree 4096.
    let t = Instant::now();
    assert_eq!(error_of("((x+1)^64)^64 = 0"), over);
    assert!(t.elapsed() < Duration::from_secs(1), "{:?}", t.elapsed());
}

#[test]
fn more_alternatives_than_the_limit_are_an_error() {
    let answers = |n: usize| {
        (1..=n)
            .map(|k| format!("x = {k}"))
            .collect::<Vec<_>>()
            .join(" or ")
    };
    assert!(parse_line(&answers(MAX_ALTERNATIVES)).is_ok());
    assert_eq!(
        error_of(&answers(MAX_ALTERNATIVES + 1)),
        ParseError::TooManyAlternatives {
            max: MAX_ALTERNATIVES
        }
    );
}

#[test]
fn the_limits_are_named_in_the_messages() {
    let message = |line: &str| check_step("x = 1", line).unwrap_err().to_string();
    assert_eq!(
        message(&"1".repeat(MAX_LINE_CHARS + 1)),
        "line 2: the line is longer than 500 characters"
    );
    assert_eq!(
        message(&format!("{}x", "-".repeat(MAX_DEPTH + 1))),
        "line 2: brackets and signs are nested more than 64 deep"
    );
    assert_eq!(
        message("x^64 * x = 1"),
        "line 2: the line is too complex to check: its degree is above 64"
    );
    assert_eq!(
        message(&format!("x = 1{}", " or x = 1".repeat(MAX_ALTERNATIVES))),
        "line 2: more than 12 alternatives"
    );
}
