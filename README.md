# themis-algebra

[![CI](https://github.com/Valkyra-Labs/themis-algebra/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/Valkyra-Labs/themis-algebra/actions/workflows/ci.yml)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/License-MIT%20OR%20Apache--2.0-blue.svg)](#license)
[![Tests](https://img.shields.io/endpoint?url=https://raw.githubusercontent.com/Valkyra-Labs/themis-algebra/badges/tests.json)](https://github.com/Valkyra-Labs/themis-algebra/actions/workflows/ci.yml)
[![wasm gzip](https://img.shields.io/endpoint?url=https://raw.githubusercontent.com/Valkyra-Labs/themis-algebra/badges/wasm-size.json)](https://github.com/Valkyra-Labs/themis-algebra/actions/workflows/ci.yml)
[![MSRV 1.85](https://img.shields.io/badge/MSRV-1.85-blue.svg)](Cargo.toml)

The tests and wasm badges are published by CI from each green run on
`main`: tests passed in `cargo test --release` on Linux (unit,
integration and doc tests), and the gzip size (level 9) of the
WebAssembly module that CI builds with wasm-pack (`--features wasm`).
CI checks the MSRV with `cargo +1.85 check`.

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
  Rational roots are found among the fractions the rational root theorem
  allows, unless the leading and constant coefficients have so many
  divisors that the search would be long; those roots are then
  approximated too.
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

## Limits

A line is checked on the device that shows it (in a browser, as
WebAssembly), so what one line may ask for is bounded. A line over a
limit gets a `ParseError` that names the limit, never a stack overflow
or a search without end:

| Limit | Value | Error |
|---|---|---|
| Characters in a line | 500 (`MAX_LINE_CHARS`) | `TooLong` |
| Brackets and signs nested in each other | 64 (`MAX_DEPTH`) | `TooDeep` |
| Degree of a numerator, a denominator or the excluded points while a line is read | 64 (`MAX_DEGREE`), the largest exponent | `TooComplex` |
| Alternatives in an answer line | 12 (`MAX_ALTERNATIVES`) | `TooManyAlternatives` |

Within these limits a line made to be hard (a polynomial of a high degree
with coefficients of many digits) can still take seconds, so an
application should check lines away from its interface thread, with a
time limit of its own.

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
