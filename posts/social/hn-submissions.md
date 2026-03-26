# Hacker News Submissions

## Submission 1: The Yoneda Constraint

**Title:** The Yoneda Constraint: One categorical axiom unifying Godel, the measurement problem, and AI alignment

**URL:** https://yoneda-constraint.vercel.app

**Optimal timing:** Tuesday-Thursday, 8-10am EST

**If asked to comment, opening comment:**

We prove that Godel's incompleteness, the quantum measurement problem, the compiler bootstrap paradox, and AI alignment impossibility are all instances of a single categorical axiom: no system embedded within a larger structure can construct a complete, faithful representation of that structure from within.

The proof uses the Yoneda Lemma applied to self-referential systems. The paper is 23 pages with full formal proofs.

For AI alignment specifically: this shows that self-verified alignment is mathematically impossible — not technically difficult, impossible. External verification and layered oversight remain viable.

Source: https://github.com/YonedaAI/yoneda-constraint

---

## Submission 2: JAPL

**Title:** Show HN: JAPL -- a self-hosting FP language that compiles to TypeScript and C

**URL:** https://japl-nine.vercel.app

**Optimal timing:** Tuesday-Thursday, 8-10am EST (submit 3-5 days after Yoneda Constraint)

**If asked to comment, opening comment:**

JAPL is a statically-typed, pure-by-default functional language with Erlang-style concurrency and Rust-inspired resource safety. It compiles to both TypeScript and C.

Key stats: 7 research papers (160+ pages), 251 compiler tests, 1,495 lines of self-hosting JAPL code, and a distributed proof app (TimeTracker).

The dual-backend architecture was the hardest part. Compiling to TypeScript gives ecosystem reach; compiling to C gives systems-level performance. The type checker is shared, so both targets get the same safety guarantees.

The self-hosting compiler can compile itself, which was our validation that the language design is complete enough for real-world use.

Source: https://github.com/YonedaAI/japl

---

## Submission 3: MRA

**Title:** The Minimal Runtime Axiom: Every runtime decision is a theorem you didn't prove

**URL:** https://minimal-runtime-axiom.vercel.app

**Optimal timing:** Tuesday-Thursday, 8-10am EST (submit 3-5 days after JAPL)

**If asked to comment, opening comment:**

We formalize the intuition that runtime overhead comes from incomplete static analysis. The Minimal Runtime Axiom defines a lattice of runtime decisions and proves that for any program, there exists a minimal runtime kernel — the set of decisions that provably require runtime information.

Practically: every null check is a missing option type. Every bounds error is a missing dependent type. Every resource leak is a missing linear type. The paper makes this precise using category theory.

This is the theoretical foundation for JAPL, our programming language that implements these ideas: https://github.com/YonedaAI/japl

Source: https://github.com/YonedaAI/minimal-runtime-axiom
