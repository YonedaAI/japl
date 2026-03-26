# We Designed a Programming Language From First Principles — Here's What We Learned

Most programming languages are born from frustration. Someone gets tired of a language's limitations, forks a compiler, and adds what they want. That's a valid approach. It's not what we did.

JAPL (Just Another Programming Language) started as a research question: what happens when you design a language from mathematical first principles — category theory, type theory, process algebra — and refuse to compromise on the theory even when the engineering gets hard?

The answer: you get 7 research papers, a self-hosting compiler, dual backends targeting TypeScript and C, and a distributed proof-of-concept application. You also get some hard-won lessons about language design that we want to share.

## The Design Space

Every language makes tradeoffs. We chose ours deliberately:

**Pure by default.** Functions in JAPL are pure unless explicitly marked otherwise. Side effects are tracked in the type system. This is not the Haskell approach of monadic IO — it is closer to algebraic effects, where effectful operations are declared, composed, and handled at well-defined boundaries.

**Concurrent by design.** JAPL processes are lightweight, Erlang-style actors. They communicate by message passing, fail independently, and are supervised by fault-tolerance trees. This is not bolted on. The process model is part of the type system: the types tell you what messages a process can receive.

**Resource-safe by construction.** Inspired by Rust's ownership model, JAPL tracks resource lifetimes statically. Files, connections, and memory are acquired and released according to rules the type checker enforces. No garbage collector for resources. No forgotten close() calls.

**Distributed without apology.** JAPL processes can run on different nodes. The supervisor tree model extends to distributed supervision. The same code that runs on one machine runs on a cluster, with the type system ensuring message compatibility across node boundaries.

## Building the Compiler

We wrote the compiler in TypeScript — not because it's the ideal compiler implementation language, but because it gave us immediate access to the JavaScript ecosystem for testing, tooling, and the first compilation target.

The compiler has four stages, each built from scratch:

1. **Lexer:** Converts source text to tokens. Regular-expression-free, hand-written, with precise source location tracking for error messages.

2. **Parser:** Recursive descent. No parser generators. We wanted complete control over error messages and recovery.

3. **Type Checker:** Hindley-Milner type inference extended with linear types, effect types, and process types. This is where most of the research effort went.

4. **Code Generator:** Two backends. The TypeScript backend produces readable, idiomatic TypeScript. The C backend produces portable C99. Same type checker validates both targets.

251 tests cover every stage. We test not just happy paths but error messages, because a language is only as good as its errors.

## The Self-Hosting Milestone

A self-hosting compiler is a rite of passage for a language. It proves the language is expressive enough to handle serious, real-world code — specifically, its own compiler.

JAPL's self-hosting compiler is 1,495 lines of JAPL. It implements enough of the language to compile itself: lexing, parsing, type checking, and TypeScript code generation. The C backend is not yet self-hosted.

The experience taught us something we later formalized in a separate paper (The Yoneda Constraint): self-hosting always leaves a residual kernel. There is a minimal bootstrap — the first compiler must be compiled by something else. This is not a deficiency. It is a mathematical inevitability.

## The Proof App: TimeTracker

A language without applications is a notation. We built TimeTracker, a distributed time-tracking application, to prove that JAPL's design works for real systems.

TimeTracker uses supervisor trees to manage fault-tolerant worker processes. It demonstrates: message passing between processes, supervised failure recovery, persistent state management, and hot code reloading. It is not a large application, but it exercises every major language feature.

## What We Learned

**Lesson 1: The type system is the language.** Everything else — syntax, stdlib, tooling — is negotiable. The type system defines what the language can express and what it can prevent. Get the type system right and the rest follows.

**Lesson 2: Dual backends force clarity.** When you compile to two different targets, every design decision must be target-independent. This eliminates hidden assumptions. If something works in TypeScript but not in C, the design is wrong — not the backend.

**Lesson 3: Error messages are a feature, not an afterthought.** We spent as much time on error messages as on the type checker itself. A type error that says "type mismatch on line 47" is useless. A type error that says "expected Process<TimerMsg> but got Process<LogMsg> — did you mean to send this message to the timer process instead?" is a feature.

**Lesson 4: Research papers force rigor.** Writing 7 papers about the language while building it forced us to justify every decision formally. Several design choices that "felt right" turned out to be provably wrong when we tried to write the proofs. The papers made the language better.

**Lesson 5: Self-hosting reveals everything.** Nothing tests a language like using it to build its own compiler. Every weakness, every awkwardness, every missing feature becomes visible immediately. Self-hosting is the most honest benchmark.

## Try It

JAPL is open source. The compiler, all 7 papers, the language specification, and the TimeTracker application are available:

- Site: https://japl-nine.vercel.app
- Source: https://github.com/YonedaAI/japl

Pure by default. Concurrent by design. Resource-safe by construction. Distributed without apology.
