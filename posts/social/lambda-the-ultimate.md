# Lambda the Ultimate Submissions

## Submission 1: JAPL Language Design

**Title:** JAPL: Combining Erlang processes, Rust ownership, and Hindley-Milner inference in one type system

**Body:**

We present JAPL, a statically-typed functional language that combines three features rarely found together: Erlang-style lightweight processes with typed message passing, Rust-inspired linear types for resource safety, and Hindley-Milner type inference extended with algebraic effects.

The key technical contribution is a unified type system where process types, linear types, and effect types coexist. A process has a type `Process<Msg>` that specifies the messages it can receive. Resources have linear types that the checker enforces are used exactly once. Effects are tracked algebraically, allowing composition without monadic lifting.

The compiler targets both TypeScript and C from the same source, which forced every design decision to be target-independent. The type checker is shared between backends.

The language is self-hosting: 1,495 lines of JAPL compile the JAPL compiler. 7 research papers formalize the design.

Papers and source: https://japl-nine.vercel.app
GitHub: https://github.com/YonedaAI/japl

Interested in feedback from the LtU community, particularly on the interaction between linear types and the process model, and on the dual-backend architecture.

---

## Submission 2: The Minimal Runtime Axiom

**Title:** The Minimal Runtime Axiom: Formalizing the compile-time/runtime boundary with category theory

**Body:**

We propose the Minimal Runtime Axiom (MRA): for any program, there exists a minimal runtime kernel — the set of decisions that provably require runtime information — and every decision outside this kernel represents incomplete static analysis.

The formalization uses functors between the category of types and the category of runtime behaviors. The compile-time type system is a functor mapping source programs to behavioral specifications. The faithfulness of this functor determines the size of the runtime kernel. A perfectly faithful functor would eliminate all runtime decisions except those depending on genuinely external input.

We apply this framework to classify familiar runtime constructs: null checks (eliminated by option types), bounds checks (eliminated by dependent-length types), resource cleanup (eliminated by linear types), dynamic dispatch (eliminated by monomorphization or type classes), and garbage collection (partially eliminated by region inference).

The practical implementation is JAPL, a language designed to minimize the runtime kernel: https://github.com/YonedaAI/japl

Paper: https://minimal-runtime-axiom.vercel.app

This is related to work on total functional programming (Turner 2004) and Chlipala's certified programming, but approaches the question from a different angle — asking not "can we eliminate runtime entirely?" but "what is the provably minimal runtime?"
