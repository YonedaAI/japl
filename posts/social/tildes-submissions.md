# Tildes Submissions

## Submission 1: JAPL (to ~comp)

**Title:** JAPL: A self-hosting functional language compiling to TypeScript and C, backed by 7 research papers

**URL:** https://japl-nine.vercel.app

**Introductory comment:**

JAPL is a research programming language designed from category-theoretic first principles. It combines Erlang-style lightweight processes with Rust-inspired linear resource types and compiles to both TypeScript (for ecosystem access) and C (for systems performance).

The self-hosting milestone — 1,495 lines of JAPL compiling the JAPL compiler — was our core validation that the language design is expressive and complete enough for non-trivial work. The project includes a full compiler (hand-written lexer, recursive descent parser, Hindley-Milner type inference with extensions, dual code generators), 251 tests, and a distributed proof-of-concept app (TimeTracker).

All 7 papers are available on the site. Particularly interested in feedback on the dual-backend architecture and the type system extensions for process types.

---

## Submission 2: Yoneda Constraint (to ~science)

**Title:** The Yoneda Constraint: A categorical proof that unifies Godel, quantum measurement, bootstrap, and alignment

**URL:** https://yoneda-constraint.vercel.app

**Introductory comment:**

This paper proves that four problems from different fields — Godel's incompleteness in logic, the measurement problem in physics, the compiler bootstrap paradox, and the AI alignment problem — are all instances of a single axiom from category theory.

The Yoneda Constraint states that no system embedded within a larger structure can construct a complete, faithful representation of that structure from within. The proof uses the Yoneda Lemma applied to self-referential functors.

The AI alignment implication is the most immediately practical: self-verified alignment is mathematically impossible, which constrains the space of viable alignment strategies to external verification and layered oversight.
