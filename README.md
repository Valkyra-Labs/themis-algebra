# themis-algebra

Step checking for school algebra, in Rust.

A learner writes a line of working; themis-algebra says whether the step
from the previous line is correct, and when it is not, exactly what it
changed: the roots it lost or gained, named exactly (`x = -3`,
`x = 1/2 + √5/2`), and any change of domain. It compares meaning, not
text: `x² − 9 = 0` and `(x−3)(x+3) = 0` are the same equation however
they are written.

```rust
use themis_algebra::{check_step, explain};

let c = check_step("x^2 = 5x", "x = 5").unwrap();
assert_eq!(explain(&c), "This step loses x = 0.");
```

Status: early. One unknown; polynomial and rational equations and
expressions with exact rational arithmetic. Roots and radicals in the
input (`√`), inequalities and systems are not supported yet. Answer lines
may list alternatives (`x = 2 or x = 3`, `x = 2, x = 3`, `x = ±3`); they
are read as the union of their solution sets.

## How it decides

- Each line is parsed into an exact rational function of the unknown
  (arbitrary-precision rationals, no floating point), remembering every
  divisor that can vanish, before any cancellation: `(x²−1)/(x−1)` is
  `x + 1` undefined at 1.
- An equation's solution set is the real roots of its numerator outside
  those points, or every point of the domain when the numerator is the
  zero polynomial.
- Two equations are compared as sets with polynomial GCDs; the roots of
  what differs are counted exactly with Sturm sequences and written
  exactly when rational or quadratic (approximated beyond degree two).
- Two expressions are compared as functions on their common domain; when
  they differ, a point where both are defined shows the difference.

## Mistakes it names

Forgetting the negative root, dividing by the unknown, squaring both
sides, moving a term without changing its sign, distributing to one term
only, a wrong factorisation, multiplying by an expression that can be
zero, `(a + b)² = a² + b²`, and cancelling a factor without keeping the
restriction. The test corpus (`tests/corpus.rs`, with its table of
equation steps in `tests/common/mod.rs`) holds each of them with the
verdict and the sentence a learner reads.

## Testing

Besides the pinned verdicts, `tests/substitution.rs` checks every
verdict on the corpus by substitution, with a small floating-point
evaluator that shares no code with the engine: a lost root must satisfy
the previous line and not the new one, a gained root the reverse, every
root of an equivalent step both lines, and a reported change of domain
must be undefined on one side and defined on the other. Exact roots are
allowed a relative error of 1e-9, approximate ones 1e-6. It checks the
roots and domain points the engine names; it does not prove that no
other root was missed. Only for 300 seeded, generated steps over
factorised polynomials, whose full solution sets are known by
construction, is the verdict also compared with the complete answer.

## License

MIT OR Apache-2.0, at your option.
