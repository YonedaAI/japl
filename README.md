# JAPL -- Just Another Programming Language

**A strict, typed, effect-aware functional language combining Rust's ownership, Go's simplicity, Erlang's processes, and FP semantics.**

> *Pure by default, concurrent by design, resource-safe by construction, distributed without apology.*

---

## Language Design DNA

JAPL draws from four programming traditions, unified by a coherent type theory:

| Tradition | Contribution |
|---|---|
| **ML / OCaml / Gleam** | Algebraic types, pattern matching, parametric polymorphism, type inference |
| **Rust** | Ownership, borrowing, linear types, memory safety without GC |
| **Erlang / OTP** | Lightweight processes, message passing, supervision trees, "let it crash" |
| **Go** | Fast compilation, static binaries, simple tooling, pragmatic deployment |

---

## Core Principles

1. **Values Are Primary** -- Data is immutable by default. Values flow through functions without hidden state.
2. **Mutation Is Local and Explicit** -- When mutation is needed, it is confined to explicit scopes with linear ownership.
3. **Concurrency Is Process-Based** -- Lightweight, supervised processes communicate through typed messages. No threads, no locks.
4. **Failures Are Normal and Typed** -- Errors are values. Recovery strategies are declared in types. Supervision handles the unexpected.
5. **Distribution Is Native** -- Processes can span nodes. Location transparency and typed protocols make networked systems first-class.
6. **Functions Are the Unit of Composition** -- Pipelines, higher-order functions, and algebraic effects compose cleanly.
7. **Runtime Simplicity = Type Power** -- The type system does the heavy lifting at compile time so the runtime stays minimal and fast.

---

## Code Example

```
-- A typed web server with process-based concurrency

type Request =
  | Get(String)
  | Post(String, Body)

type Response =
  | Ok(Body)
  | NotFound
  | Error(String)

fn handle(req: Request) -> Response = match req with
  | Get("/health")    -> Ok("ok")
  | Get(path)         -> lookup(path)
  | Post(path, body)  -> store(path, body)

fn counter(count: Int) -> Int = receive
  | Increment(n) -> counter(count + n)
  | GetCount     -> count
  | Shutdown     -> count

fn main() -> Result[Unit, Error] =
  let pid = spawn(counter, 0)
  let server = listen(8080, handle)
  supervise([pid, server], OneForOne)
```

```
-- Fibonacci with pattern matching

fn fib(n: Int) -> Int = match n with
  | 0 -> 0
  | 1 -> 1
  | n -> fib(n - 1) + fib(n - 2)

fn main() =
  println(int_to_string(fib(10)))
```

---

## Research Papers

Seven papers developing the theoretical and practical foundations of the language.

| # | Title | PDF |
|---|---|---|
| I | Values Are Primary | [PDF](papers/pdf/values-are-primary.pdf) |
| II | Mutation Is Local and Explicit | [PDF](papers/pdf/mutation-is-local.pdf) |
| III | Process-Based Concurrency | [PDF](papers/pdf/process-concurrency.pdf) |
| IV | Typed Failures | [PDF](papers/pdf/typed-failures.pdf) |
| V | Native Distribution | [PDF](papers/pdf/native-distribution.pdf) |
| VI | Function Composition | [PDF](papers/pdf/function-composition.pdf) |
| VII | Runtime Simplicity | [PDF](papers/pdf/runtime-simplicity.pdf) |

---

## Compiler Architecture

The compiler is implemented as a **Rust workspace** with modular crates:

| Crate | Role |
|---|---|
| `japl-common` | Shared types and utilities |
| `japl-lexer` | Tokenization (logos-based) |
| `japl-parser` | Recursive-descent parser producing AST |
| `japl-ast` | Abstract syntax tree definitions |
| `japl-types` | Type representations and type environment |
| `japl-checker` | Type inference, unification, linearity checking, effect tracking |
| `japl-ir` | Intermediate representation and lowering |
| `japl-codegen` | Code generation / tree-walking interpreter |
| `japl-driver` | CLI driver orchestrating the pipeline |
| `japl-runtime` | Process scheduler, mailboxes, supervision, GC |
| `japl-stdlib` | Standard library primitives |

**167 tests** pass across all crates.

---

## Build and Run

```bash
# Build the compiler
cd compiler
cargo build --release

# Run tests
cargo test

# Run a JAPL program
cargo run -- run tests/fibonacci.japl
```

---

## Project Structure

```
japl/
  compiler/          Rust workspace (lexer, parser, checker, codegen, runtime)
  docs/              Project homepage (HTML/CSS)
  papers/
    latex/           LaTeX sources for the seven research papers
    pdf/             Compiled PDF deliverables
  spec/              Language specification and compiler architecture docs
  scripts/           Build and utility scripts
  posts/             Blog posts and writeups
  reviews/           Paper reviews and notes
```

---

## Author

**Matthew Long**
YonedaAI Research Collective
Chicago, IL
matthew@yonedaai.com

---

[Homepage](https://yonedaai.github.io/japl/) | [GitHub](https://github.com/YonedaAI/japl)
