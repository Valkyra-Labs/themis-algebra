//! Step checking for school algebra.
//!
//! Each line of a learner's working is parsed into an exact rational
//! function of one unknown. Consecutive equations are compared by their
//! real solution sets (roots of the numerator, minus the points where the
//! line is undefined), so a step that divides by `x`, squares both sides,
//! drops a sign or forgets a root is caught and the lost or gained roots
//! are named exactly: `x = −3`, `x = 1/2 + √5/2`. Consecutive expressions
//! are compared as functions on their common domain. Changes of domain
//! (cancelling a factor, multiplying by an expression) are reported on
//! their own.
//!
//! ```
//! use themis_algebra::{check_step, explain};
//! let c = check_step("x^2 = 9", "x = 3").unwrap();
//! assert_eq!(explain(&c), "This step loses x = -3.");
//! ```

pub mod check;
pub mod expr;
pub mod poly;
#[cfg(feature = "wasm")]
pub mod wasm;

pub use check::{check_step, explain, solve, CheckError, Solutions, StepCheck, Verdict};
pub use expr::{
    parse_line, Line, ParseError, MAX_ALTERNATIVES, MAX_DEGREE, MAX_DEPTH, MAX_LINE_CHARS,
};
pub use poly::{Poly, Root, Q};
