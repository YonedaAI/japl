# JAPL -- Just Another Programming Language

A typed actor language with immutable values, supervision, and explicit resource safety.

> Pure functions handle logic. Supervised processes handle time and failure. Ownership handles resources.

---

## What JAPL Is

JAPL compiles to WebAssembly and runs on a unified Rust runtime with real OS-thread processes, typed message passing, supervision trees, HTTP serving, and wasmCloud deployment.

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

```bash
# Build the compiler + runtime
cd japl && cargo build --release

# Hello world
echo 'fn main() { println("Hello from JAPL!") }' > hello.japl
japl run hello.japl

# With processes
japl run apps/kvstore/kvstore.japl

# HTTP serving
japl serve apps/http-kv/kv_server.japl --port 8080

# Deploy to wasmCloud (requires NATS + wasmCloud)
# Prerequisites: nats-server, wash CLI, japl-provider
japl deploy apps/distributed/hello_distributed.japl

# Preview deployment manifest
japl deploy --dry-run apps/distributed/hello_distributed.japl

# Local-only mode (no wasmCloud needed)
japl deploy --local apps/distributed/hello_distributed.japl

# Type check
japl check myfile.japl

# Format
japl fmt myfile.japl

# Initialize a new package
japl init my-project
```

---

## Release Verification

To verify a release build, run:

    scripts/release-check.sh

This builds the compiler in release mode, runs the full verification suite with `--release` (where wasmCloud SKIPs become FAILs), and builds the provider. See [docs/release-process.md](docs/release-process.md) for the full release process and checklist.

---

## Features

### Working

- Immutable values, algebraic data types, pattern matching
- First-class functions, closures, higher-order functions
- Pipe operator (`|>`)
- Records (creation, field access, update)
- Type inference (bidirectional) with `Type::Pid` (Int backward compat)
- Effect tracking (`Pure`, `IO`, `LLM`, `Process`, `Fail`)
- Exhaustive pattern matching (`--strict` mode)
- Tail call optimization
- Module system with imports
- Foreign function interface (WASI)
- String interpolation
- Checked integer arithmetic (no silent overflow)
- Byte type (`u8`)
- Code formatter (`japl fmt`)
- Type checker (`japl check`, `--strict` for Pid warnings)
- Process spawn/send/receive (real OS threads via embedded wasmtime)
- HTTP serving (`japl serve` via tiny_http)
- Real env var reading, file I/O
- Standard library: 30 modules, 2400+ LOC (`Math`, `String`, `Option`, `Result`, `List`, `Map`, `Set`, `Json`, `Http`, `Net`, `Bytes`, `Codec`, `Retry`, `Log`, `Config`, `File`, `Env`, `Time`, `Crypto`, `Process`, `Supervisor`, `Registry`, `LLM`, `Tool`, `Budget`, `Replay`, `Provenance`, `Core`, `IO`, `Test`)
- Verification suite: 68+ tests, 28 negative checker tests, 2 strict mode tests
- Benchmark suite and stdlib API doc generator

### Partial

- Supervision trees (`OneForOne`, `AllForOne`, `RestForOne`) — polling-based, no automatic restart (requires monitor/link primitives)
- wasmCloud deployment (`japl deploy`) — component build works, full deploy requires NATS + japl-provider sidecar
- AI-native abstractions: LLM as tracked effect, `llm_structured` validates JSON prefix only (no schema enforcement)
- Tool contracts (`ToolSpec`, `ToolResult`), budget tracking, replay logs, provenance — simulated execution, no real dispatch backend
- Distributed typed message passing (local works; TCP cross-machine experimental via `japl run --node-name`)
- Package manager foundation (`japl init`, `japl deps`)

### Planned

- LSP / editor support
- REPL
- Full package registry
- Native wasmCloud capability provider (currently sidecar mode)

---

## Architecture

```
.japl source
    |  JAPL Compiler (Rust)
    v  Lexer -> Parser -> Checker -> Lower -> WAT Codegen
.wat -> wat2wasm -> .wasm
    |
    +-- japl run      (embedded wasmtime, local OS-thread processes)
    +-- japl serve    (HTTP via tiny_http)
    +-- japl deploy   (wasmCloud + JAPL NATS provider)
    +-- japl build    (compile to .wasm only)
    +-- japl check    (type check without compiling)
    +-- japl fmt      (code formatter)
    +-- japl init     (initialize package)
    +-- japl deps     (manage dependencies)
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
japl/                 Unified compiler + runtime (Rust)
  src/compiler/       Lexer, parser, checker, AST, IR, WAT codegen, formatter
  src/runtime/        Scheduler, host functions, process engine, distribution, wire protocol
  src/serve.rs        HTTP serving mode
  src/main.rs         CLI entry point (build, run, serve, deploy, check, fmt, init, deps)
  src/package.rs      Package manifest handling
stdlib/               Standard library (30 modules, 2400+ LOC)
apps/                 Demo applications (8 apps)
  kvstore/            Distributed key-value store
  msgqueue/           Message queue
  scheduler/          Task scheduler
  genome/             Genome analysis pipeline
  agents/             Multi-agent system
  distributed/        Distributed hello world
  http-kv/            HTTP key-value server
  kvstore-http/       HTTP-fronted KV store
japl-provider/        NATS-based process provider for wasmCloud
test/                 Verification suite
  verify/             Main test runner (verify_all.py)
  programs/           42 test programs
  checker-negative/   28 negative type checker tests
  checker-strict/     Strict mode tests
  bench/              Benchmark suite
docs/                 Architecture and integration docs
spec/                 Language specification
papers/               Research papers (7 JAPL papers)
```

---

## Known Limitations

- **Supervision**: Polling-based only. Supervisor can spawn children but cannot automatically restart crashed processes (requires runtime monitor/link primitives not yet implemented)
- **Distribution**: Custom TCP layer is experimental; wasmCloud provider runs as a NATS sidecar, not a native wasmCloud capability provider
- **Tool execution**: Tool.japl provides types and simulated execution, not real tool dispatch
- **LLM structured output**: Validates JSON prefix only, no schema enforcement
- **String FFI**: Some stdlib modules (Net) cannot pass Strings through FFI cleanly (compile-only tested)
- **Process messages**: String fields in ADT messages may be empty when received cross-process
- **Budget/Replay/Provenance**: Pure-JAPL wrappers tracking state in-process; no runtime-level enforcement

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
