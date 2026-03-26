# Gleam Discord Post

**Channel:** #off-topic or #other-languages

Hey Gleam community — sharing a language project you might find interesting given the overlap in design space.

JAPL is a statically-typed FP language that, like Gleam, combines ML-family type inference with Erlang-style concurrency. Some similarities and differences:

**Shared goals:**
- Type safety + BEAM-style concurrency
- Algebraic data types + pattern matching
- Friendly error messages as a first-class concern

**Where we diverge:**
- JAPL compiles to TypeScript + C (not BEAM/JS)
- Linear types for resource safety (Rust-inspired)
- Algebraic effects for side effect tracking
- Pure by default (effects in the type system)
- Self-hosting compiler

The Erlang process model is a core part of the language, not a target runtime — so JAPL implements its own lightweight process scheduler rather than running on BEAM.

7 research papers behind it, 251 compiler tests, and a distributed proof app (TimeTracker).

Site: https://japl-nine.vercel.app
GitHub: https://github.com/YonedaAI/japl

Would love to hear thoughts from people who've thought deeply about the typed-processes design space.
