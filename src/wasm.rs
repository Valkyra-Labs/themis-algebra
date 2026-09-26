//! JavaScript bindings (feature `wasm`).

use crate::check::{check_step, explain, solve, Solutions, Verdict};
use crate::poly::{fmt_q, Root};
use wasm_bindgen::prelude::*;

fn texts(v: &[Root]) -> Vec<String> {
    v.iter().map(|r| r.to_string()).collect()
}

/// The result of checking one step, flattened for JavaScript.
#[wasm_bindgen(getter_with_clone)]
pub struct StepResult {
    /// "equivalent", "changed", "not_equal" or "error".
    pub kind: String,
    pub lost: Vec<String>,
    pub gained: Vec<String>,
    #[wasm_bindgen(js_name = lostInfinitelyMany)]
    pub lost_infinitely_many: bool,
    #[wasm_bindgen(js_name = gainedInfinitelyMany)]
    pub gained_infinitely_many: bool,
    #[wasm_bindgen(js_name = domainWidenedAt)]
    pub domain_widened_at: Vec<String>,
    #[wasm_bindgen(js_name = domainNarrowedAt)]
    pub domain_narrowed_at: Vec<String>,
    /// For "not_equal": an x where both are defined and differ, and the
    /// two values there.
    pub witness: String,
    pub before: String,
    pub after: String,
    /// A plain-English sentence for the learner, or the error.
    pub explanation: String,
}

/// Check the step from `before` to `after`.
#[wasm_bindgen(js_name = checkStep)]
pub fn check_step_js(before: &str, after: &str) -> StepResult {
    let mut r = StepResult {
        kind: String::new(),
        lost: vec![],
        gained: vec![],
        lost_infinitely_many: false,
        gained_infinitely_many: false,
        domain_widened_at: vec![],
        domain_narrowed_at: vec![],
        witness: String::new(),
        before: String::new(),
        after: String::new(),
        explanation: String::new(),
    };
    match check_step(before, after) {
        Err(e) => {
            r.kind = "error".into();
            r.explanation = e.to_string();
        }
        Ok(c) => {
            r.explanation = explain(&c);
            r.domain_widened_at = texts(&c.domain_widened_at);
            r.domain_narrowed_at = texts(&c.domain_narrowed_at);
            match c.verdict {
                Verdict::Equivalent => r.kind = "equivalent".into(),
                Verdict::Changed {
                    lost,
                    gained,
                    lost_infinitely_many,
                    gained_infinitely_many,
                } => {
                    r.kind = "changed".into();
                    r.lost = texts(&lost);
                    r.gained = texts(&gained);
                    r.lost_infinitely_many = lost_infinitely_many;
                    r.gained_infinitely_many = gained_infinitely_many;
                }
                Verdict::NotEqual {
                    witness,
                    before,
                    after,
                } => {
                    r.kind = "not_equal".into();
                    r.witness = fmt_q(&witness);
                    r.before = fmt_q(&before);
                    r.after = fmt_q(&after);
                }
            }
        }
    }
    r
}

/// Solutions of one equation: `["2", "3"]`, `[]` for none, or `["*"]`
/// for every x of the domain. Throws on a parse error.
#[wasm_bindgen(js_name = solve)]
pub fn solve_js(line: &str) -> Result<Vec<String>, JsError> {
    match solve(line).map_err(|e| JsError::new(&e.to_string()))? {
        Solutions::AllExcept(_) => Ok(vec!["*".into()]),
        s => Ok(texts(&s.roots().unwrap_or_default())),
    }
}
