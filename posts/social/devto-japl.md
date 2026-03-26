---
title: "JAPL: A Self-Hosting Language That Compiles to TypeScript and C"
published: false
tags: programming-languages, compilers, functional-programming, typescript
---

# JAPL: A Self-Hosting Language That Compiles to TypeScript and C

JAPL (Just Another Programming Language) is a statically-typed functional language with Erlang-style concurrency and Rust-inspired resource safety. It compiles to both TypeScript and C from the same source, and it can compile itself.

## The Design

JAPL's identity: **Pure by default, concurrent by design, resource-safe by construction, distributed without apology.**

It draws from four languages:

| Influence | What We Took |
|-----------|-------------|
| Erlang | Lightweight processes, message passing, supervisor trees, fault tolerance |
| Rust | Ownership model, linear types, resource safety without GC |
| Go | Clean syntax, simplicity, readability |
| TypeScript | Ecosystem access, compilation target, tooling |

## Language Features

### Pure Functions by Default

```
fn add(x: Int, y: Int) -> Int {
  x + y
}
```

Side effects are tracked in the type system. A function that performs IO has a different type than one that doesn't.

### Process Model

```
process Timer {
  receive {
    Start(duration) -> {
      sleep(duration)
      send(self(), Tick)
    }
    Tick -> {
      log("tick")
      send(self(), Tick)
    }
  }
}
```

Processes are lightweight, isolated, and supervised. They communicate exclusively through message passing.

### Resource Safety

```
fn readFile(path: String) -> Result<String, IOError> {
  use file = open(path)  // acquired here
  read(file)             // used here
}                        // released here, guaranteed
```

Linear types ensure resources are acquired, used, and released. The type checker enforces this at compile time.

### Pattern Matching

```
fn describe(shape: Shape) -> String {
  match shape {
    Circle(r) -> "circle with radius " ++ show(r)
    Rect(w, h) -> "rectangle " ++ show(w) ++ "x" ++ show(h)
    Triangle(a, b, c) -> "triangle with sides " ++ show(a) ++ ", " ++ show(b) ++ ", " ++ show(c)
  }
}
```

Exhaustive pattern matching checked at compile time.

## Compiler Architecture

```
Source → Lexer → Parser → Type Checker → Code Generator
                                              ↓
                                     ┌────────┴────────┐
                                     │                  │
                                TypeScript              C
```

Four stages, all hand-written:

1. **Lexer:** Hand-written tokenizer with precise source location tracking
2. **Parser:** Recursive descent, no parser generators
3. **Type Checker:** Hindley-Milner inference + linear types + effect types + process types
4. **Code Generator:** Two backends producing idiomatic TypeScript or portable C99

251 tests cover every stage.

## Self-Hosting

The self-hosting compiler is 1,495 lines of JAPL that compile JAPL. This validates that the language is expressive enough to handle real, complex code — its own compiler.

## TimeTracker: The Proof App

TimeTracker is a distributed time-tracking application built in JAPL. It demonstrates:

- Supervisor trees managing worker processes
- Message passing between concurrent processes
- Fault-tolerant recovery from process failures
- Persistent state management

## The Research

JAPL is backed by 7 research papers (160+ pages) covering:

1. Language design and specification
2. Type system (Hindley-Milner + linear types + effects)
3. Process model and concurrency
4. Compiler architecture
5. Self-hosting and bootstrap
6. Dual-backend code generation
7. TimeTracker case study

## Try It

- **Site:** https://japl-nine.vercel.app
- **GitHub:** https://github.com/YonedaAI/japl

The compiler, all papers, the language specification, and the TimeTracker application are open source.

## Related Projects

- [The Minimal Runtime Axiom](https://minimal-runtime-axiom.vercel.app) — the theoretical foundation for JAPL's "minimize runtime" design philosophy
- [The Yoneda Constraint](https://yoneda-constraint.vercel.app) — the capstone paper unifying JAPL's bootstrap paradox with Godel, quantum measurement, and AI alignment
