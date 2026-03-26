# Discourse Forum Posts

## Rust Users Forum (discuss.rust-lang.org)

**Title:** Applying Rust's ownership ideas to a pure functional language — lessons learned

**Category:** General

**Body:**

We built JAPL, a statically-typed functional language that takes direct inspiration from Rust's ownership model. We wanted to bring linear resource safety to an FP context with Erlang-style concurrency.

What we kept from Rust:
- Ownership semantics: every resource has a single owner
- Linear types: resources must be used exactly once (acquired, used, released)
- Compile-time enforcement: no runtime garbage collector for resources

What we changed:
- No borrow checker. In a pure functional language with immutable-by-default values, most of Rust's borrowing complexity goes away. We track linearity instead of lifetimes.
- Process isolation replaces shared-memory concurrency. Each process owns its resources. No Send/Sync traits needed because processes never share memory.
- Algebraic effects instead of Result chains. Effect handlers compose more cleanly than chained Results for our use case.

The biggest lesson: Rust's ownership model is more general than Rust. The core idea — track who owns what, enforce it statically, release deterministically — works beautifully in a pure FP context without most of the complexity that shared mutability introduces.

JAPL compiles to both TypeScript and C, is self-hosting, and has 7 research papers behind it.

Paper and source: https://japl-nine.vercel.app
GitHub: https://github.com/YonedaAI/japl

Curious whether anyone in the Rust community has explored similar crossover designs.

---

## Elixir Forum (elixirforum.com)

**Title:** Building Erlang-style fault tolerance into a statically-typed FP language

**Category:** General Discussion

**Body:**

As fans of the BEAM ecosystem, we built JAPL — a language that brings Erlang/Elixir-style concurrency to a statically-typed setting with Hindley-Milner type inference.

What we took from Erlang/Elixir:
- Lightweight processes as the unit of concurrency
- Message passing (no shared memory)
- Supervisor trees for fault tolerance
- Let-it-crash philosophy
- Hot code reloading

What static types add:
- Process types: `Process<Msg>` specifies exactly what messages a process accepts. Sending the wrong message type is a compile-time error, not a runtime crash.
- Exhaustive receive: the type checker ensures every possible message type is handled.
- Typed supervision: the supervisor tree structure is part of the type system.

Our proof app, TimeTracker, is a distributed time-tracking system with supervisor trees, typed message passing, and fault recovery — basically a small OTP application, but statically typed.

The tradeoff we wrestled with most: Elixir's dynamic typing makes hot code reloading and protocol evolution much easier. Static process types make refactoring safer but protocol changes harder. We don't claim to have solved this perfectly.

JAPL is self-hosting and compiles to both TypeScript and C.

Site: https://japl-nine.vercel.app
GitHub: https://github.com/YonedaAI/japl

Would love to hear from the Elixir community about this approach.

---

## OCaml Discuss

**Title:** JAPL: An ML-family language with linear types and Erlang-style processes

**Category:** Community

**Body:**

Sharing a project that may interest the OCaml community. JAPL is a statically-typed functional language in the ML family tradition — Hindley-Milner type inference, algebraic data types, exhaustive pattern matching — extended with linear types for resource safety and Erlang-style process types for concurrency.

Connections to OCaml:
- HM type inference as the foundation
- ADTs and pattern matching
- Module-like organization
- Effect tracking (related to OCaml 5's effect handlers, though our approach is algebraic)

Where we diverge:
- Pure by default (effects tracked in types, not implicit)
- Linear types for resources (closer to Rust than OCaml)
- Process model for concurrency (closer to Erlang than OCaml's multicore approach)
- Dual compilation targets (TypeScript + C)

The language is self-hosting (1,495 lines of JAPL compile the JAPL compiler) and backed by 7 research papers.

Site: https://japl-nine.vercel.app
GitHub: https://github.com/YonedaAI/japl
