# JAPL -- Just Another Programming Language

A typed actor language with immutable values, supervision, and explicit resource safety.

> Pure functions handle logic. Supervised processes handle time and failure. Ownership handles resources.

---

## What JAPL Is

JAPL compiles to WebAssembly and runs on a custom Rust runtime with real OS-thread processes, typed message passing, and supervision trees.

```japl
type KVCommand =
  | Put(Int, Int, Pid)
  | Get(Int, Pid)
  | Size(Pid)

type KVResponse =
  | Found(Int)
  | NotFound
  | Stored
  | Count(Int)

fn partition(data_count: Int) {
  receive {
    Put(key, val, reply) =>
      let _ = send(reply, Stored)
      partition(data_count + 1),
    Get(key, reply) =>
      let _ = send(reply, Found(key * 10))
      partition(data_count),
    Size(reply) =>
      let _ = send(reply, Count(data_count))
      partition(data_count)
  }
}

fn main() {
  let p1 = spawn(fn() { partition(0) })
  let _ = send(p1, Put(1, 100, self()))
  let _ = receive { Stored => println("Stored key=1") }
  let _ = send(p1, Get(1, self()))
  let _ = receive {
    Found(v) => println("Got key=1: " <> show(v)),
    NotFound => println("key=1 not found")
  }
}
```

This is a real program. It compiles to WASM and runs on the JAPL runtime with actual OS-thread processes.

---

## Quick Start

**Prerequisites:** Rust toolchain (`rustup`), wat2wasm (`brew install wabt`), wasmtime (`brew install wasmtime`)

Build the runtime: `cd japl-runtime && cargo build --release`

```bash
# Hello world
echo 'fn main() { println("Hello from JAPL!") }' > hello.japl
japl run hello.japl

# Build to WASM
japl build hello.japl
wasmtime build/hello.wasm

# With processes (requires japl-runtime)
japl run --runtime apps/kvstore/kvstore.japl
```

---

## Features

### Working

- Immutable values, algebraic data types, pattern matching
- First-class functions, closures, higher-order functions
- Pipe operator (`|>`)
- Records (creation, field access, update)
- Type inference (bidirectional)
- Effect tracking (`Pure`, `IO`, `LLM`, `Process`, `Fail`)
- Exhaustive pattern matching (`--strict` mode)
- Tail call optimization
- Module system with imports
- Foreign function interface (WASI)
- String interpolation
- Checked integer arithmetic (no silent overflow)
- Byte type (`u8`)
- Code formatter (`japl fmt`)
- Process spawn/send/receive (real OS threads via WASM)
- Supervision trees (`OneForOne`, `AllForOne`, `RestForOne`)
- TCP distribution between runtime instances

- AI-native abstractions: LLM as tracked effect, `llm_structured` for typed I/O
- Tool contracts (`ToolSpec`, `ToolResult`), budget tracking, replay logs, provenance
- Standard library: `Math`, `String`, `Option`, `Result`, `List`, `Map`, `Set`, `Json`, `Http`, `Net`, `Bytes`, `Codec`, `Retry`, `Log`, `Config`, `File`, `Env`, `Time`, `Crypto`, `Process`, `Supervisor`, `Registry`, `LLM`, `Tool`, `Budget`, `Replay`, `Provenance`

### Prototype

- Distributed typed message passing (local works, cross-machine in testing)

### Planned

- Package manager
- LSP / editor support
- REPL

---

## Architecture

```
.japl source
    |
    v
JAPL Compiler (self-hosted, 1557 lines of JAPL)
  Lexer -> Parser -> WAT Codegen
    |
    v
.wat (WebAssembly Text)
    |  wat2wasm
    v
.wasm (WebAssembly Binary)
    |
    v
+------------------------------------------+
| wasmtime          (simple programs)      |
| japl-runtime      (processes, TCP)       |
+------------------------------------------+
```



---

## Code Examples

**Pattern matching and algebraic types:**

```japl
type Light =
  | Red
  | Yellow
  | Green

type Action =
  | Next
  | Emergency

fn transition(light: Light, action: Action) -> Light {
  match action {
    Emergency => Red
    Next => match light {
      Red => Green
      Green => Yellow
      Yellow => Red
    }
  }
}
```

**Closures and pipes:**

```japl
fn make_adder(n: Int) {
  fn(x: Int) { x + n }
}

fn double(x: Int) -> Int { x * 2 }

fn main() {
  let add5 = make_adder(5)
  println(show(add5(3)))          -- 8
  println(show(5 |> double |> double))  -- 20
}
```

---

## Project Structure

```
compiler/self/      Self-hosted compiler (JAPL source + compiled WASM)
japl-runtime/       Runtime (Rust + wasmtime, processes, distribution)
stdlib/             Standard library (.japl files)
test/               Test programs and verification suite
apps/               Applications (KV store, message queue, scheduler, genome pipeline, multi-agent demo)
spec/               Language specification
plans/              Development plans and reviews
papers/             Research papers (7 JAPL papers)
docs/               Project website
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

## Author

**Matthew Long**
YonedaAI Research Collective
Chicago, IL
matthew@yonedaai.com

---

[Homepage](https://yonedaai.github.io/japl/) | [GitHub](https://github.com/YonedaAI/japl)
