# Discord & Zulip Posts

## Discord: Programming Language Design & Implementation

**Channel:** #showcase or #language-design

Hey all — sharing a language project: JAPL, a statically-typed FP language with Erlang-style processes and Rust-inspired linear types. Compiles to both TypeScript and C.

The interesting design bits:
- Process types (`Process<Msg>`) so message passing is type-checked
- Linear types for resource safety without GC
- Algebraic effects for tracked side effects
- Dual-backend compilation from one type checker

It's self-hosting (1,495 lines of JAPL compile JAPL) and has 7 papers behind it.

Site: https://japl-nine.vercel.app
GitHub: https://github.com/YonedaAI/japl

Happy to discuss any design decisions.

---

## Discord: Type Theory

**Channel:** #general or #research

Sharing two papers that may interest this community:

1. **The Minimal Runtime Axiom** — formalizes the idea that runtime decisions form a lattice, and proves there's a minimal runtime kernel for any program. Uses functors between the category of types and runtime behaviors. https://minimal-runtime-axiom.vercel.app

2. **The Yoneda Constraint** — proves that Godel's incompleteness, quantum measurement, compiler bootstrap, and AI alignment are all instances of one categorical axiom derived from the Yoneda Lemma. https://yoneda-constraint.vercel.app

Both are connected to JAPL, a language we built that implements these ideas: https://github.com/YonedaAI/japl

---

## Discord: Functional Programming

**Channel:** #projects or #show-and-tell

Built a pure-by-default FP language called JAPL. Think: Haskell's purity + Erlang's processes + Rust's ownership.

Key features:
- Pure by default, effects tracked algebraically
- Lightweight processes with typed message passing
- Linear types for resources
- Compiles to TypeScript AND C
- Self-hosting (compiles its own compiler)

It's not a toy — there's a distributed proof app (TimeTracker) and 7 research papers.

https://japl-nine.vercel.app
https://github.com/YonedaAI/japl

---

## Discord: Category Theory

**Channel:** #applications or #general

Sharing a paper that applies the Yoneda Lemma to self-referential systems: **The Yoneda Constraint**.

The main result: no endofunctor on a self-referential category can be simultaneously full and faithful. This gives a categorical proof that Godel's incompleteness, the measurement problem, compiler bootstrap, and AI alignment impossibility are structurally identical.

The proof constructs a diagonal argument within the functor category, showing that any natural transformation from a self-referential functor to the identity functor must have a non-trivial kernel.

23 pages, full proofs: https://yoneda-constraint.vercel.app
Source: https://github.com/YonedaAI/yoneda-constraint

---

## Zulip: Lean Community

**Stream:** #general or #papers

Sharing a paper relevant to the formal verification angle: **The Minimal Runtime Axiom**.

The core claim: for any program, there exists a provably minimal set of runtime decisions. Everything outside this set can, in principle, be resolved at compile time. The formalization uses categorical semantics — functors between type categories and behavioral categories.

The practical question for the Lean community: how far can dependent type theory push the boundary? The MRA suggests the theoretical limit is determined by information that genuinely requires external input. Everything else — null checks, bounds, resource management — is provable.

We'd be interested in whether anyone has formalized similar results in Lean.

Paper: https://minimal-runtime-axiom.vercel.app
Related language (JAPL): https://github.com/YonedaAI/japl

---

## Zulip: Agda Community

**Stream:** #general

Question for the Agda community: how far can you push compile-time guarantees without full dependent types?

We built JAPL, a language with Hindley-Milner inference + linear types + process types + algebraic effects. No dependent types. The question we kept hitting: where is the boundary between what HM+extensions can prove and what requires dependent types?

Our paper, The Minimal Runtime Axiom, formalizes this boundary categorically. The short answer: HM+linear types eliminates resource leaks and null errors. You need dependent types (or refinement types) to eliminate bounds checks and protocol violations.

Paper: https://minimal-runtime-axiom.vercel.app
Language: https://github.com/YonedaAI/japl

Curious about experiences from people working in Agda on similar questions.
