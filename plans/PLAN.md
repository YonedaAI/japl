# JAPL Development Plan

> **JAPL = Erlang processes + Rust safety instincts + Go simplicity + TypeScript reach**
>
> A pure-by-default, actor-oriented, resource-safe language that compiles to TypeScript.

---

## Table of Contents

1. [Identity](#identity)
2. [Architecture Overview](#architecture-overview)
3. [Phase Map](#phase-map)
4. [Phase 1: Compiler Core](#phase-1-compiler-core)
5. [Phase 2: TypeScript Runtime](#phase-2-typescript-runtime)
6. [Phase 3: Standard Library](#phase-3-standard-library)
7. [Phase 4: Tooling](#phase-4-tooling)
8. [Phase 5: Self-Hosting](#phase-5-self-hosting)
9. [Phase 6: Proof App — TimeTracker](#phase-6-proof-app--timetracker)
10. [Team Structure](#team-structure)
11. [Acceptance Criteria](#acceptance-criteria)
12. [File-Level Implementation Guide](#file-level-implementation-guide)

---

## Identity

**JAPL** — Just Another Programming Language

- **Compiles to**: TypeScript (readable, inspectable output)
- **Runs on**: Node.js, Deno, Bun, browsers, edge functions, Lambda
- **Feels like**: ML/OCaml/Gleam (logic) + Erlang (processes) + Rust (safety) + Go (simplicity)
- **Package ecosystem**: npm (generated TS packages are standard npm modules)

### Core Principles

1. **Values are primary** — immutable by default, algebraic data types, pattern matching
2. **Mutation is local and explicit** — controlled effects, resource tracking
3. **Concurrency is process-based** — lightweight actors, typed mailboxes, message passing
4. **Failures are normal and typed** — Result/Option + crash/restart supervision
5. **Distribution is native** — processes can be local or remote
6. **Functions are the unit of composition** — no classes, no inheritance, pipe-first
7. **Runtime simplicity = type power** — fast compiler, simple tooling, static binaries

### First Version Feature Set (MVP)

- [x] Immutable `let` bindings
- [x] First-class functions / closures
- [x] Algebraic data types (enums / tagged unions)
- [x] Pattern matching (exhaustive)
- [x] `Result<T,E>` / `Option<T>` error handling
- [x] Modules with public/private visibility
- [x] Lightweight processes (actors)
- [x] Async message send/receive
- [x] Supervised workers
- [x] Basic collections (List, Map, Set)
- [x] Effect boundary for IO
- [x] Pipe operator `|>`
- [x] Traits / type classes
- [x] Type inference (bidirectional, local)

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────┐
│                    JAPL Source (.japl)                       │
└─────────────────────────┬───────────────────────────────────┘
                          │
┌─────────────────────────▼───────────────────────────────────┐
│  COMPILER (TypeScript — self-hosting after Phase 5)         │
│                                                             │
│  ┌─────────┐  ┌─────────┐  ┌───────────┐  ┌────────────┐  │
│  │  Lexer  │→ │ Parser  │→ │Type Check │→ │Effect Check│  │
│  └─────────┘  └─────────┘  └───────────┘  └─────┬──────┘  │
│                                                   │         │
│  ┌──────────────┐  ┌────────────┐  ┌─────────────▼──────┐  │
│  │  Linearity   │← │     IR     │← │    Lowering        │  │
│  │  Check       │  │            │  │                    │  │
│  └──────┬───────┘  └─────┬──────┘  └────────────────────┘  │
│         │                │                                  │
│         └────────────────▼──────────────────────────┐       │
│                    ┌────────────┐                    │       │
│                    │  Codegen   │                    │       │
│                    │  (to TS)   │                    │       │
│                    └─────┬──────┘                    │       │
└──────────────────────────┼──────────────────────────────────┘
                           │
┌──────────────────────────▼──────────────────────────────────┐
│              Generated TypeScript (.ts)                      │
│                                                             │
│  import { spawn, send, receive } from "@japl/runtime"       │
│  import { Result, Ok, Err } from "@japl/runtime"            │
│                                                             │
│  // Clean, readable, idiomatic TypeScript                   │
└─────────────────────────┬───────────────────────────────────┘
                          │
┌─────────────────────────▼───────────────────────────────────┐
│  RUNTIME (@japl/runtime — npm package)                      │
│                                                             │
│  ┌───────────┐ ┌──────────┐ ┌────────────┐ ┌───────────┐   │
│  │ Scheduler │ │ Mailbox  │ │ Supervisor │ │  Effects  │   │
│  │ (event    │ │ (typed   │ │ (restart   │ │ (IO       │   │
│  │  loop)    │ │  queues) │ │  trees)    │ │  boundary)│   │
│  └───────────┘ └──────────┘ └────────────┘ └───────────┘   │
└─────────────────────────────────────────────────────────────┘
                          │
┌─────────────────────────▼───────────────────────────────────┐
│  DEPLOYMENT TARGET                                          │
│                                                             │
│  Node.js │ Deno │ Bun │ Browser │ Edge │ Lambda │ Vercel   │
└─────────────────────────────────────────────────────────────┘
```

### Generated TypeScript Example

JAPL source:
```
type Msg =
  | Tick(UserId)
  | Stop(UserId)

fn timer_loop(state: TimerState) -> Process<Unit> {
  receive {
    Tick(uid) => timer_loop(tick(state, uid))
    Stop(uid) => timer_loop(stop(state, uid))
  }
}

fn main() -> ![IO] Unit {
  let pid = spawn(timer_loop(initial_state()))
  send(pid, Tick("user-1"))
}
```

Generated TypeScript:
```typescript
import { spawn, send, receive, Process } from "@japl/runtime";

type Msg =
  | { _tag: "Tick"; _0: string }
  | { _tag: "Stop"; _0: string };

const Tick = (_0: string): Msg => ({ _tag: "Tick", _0 });
const Stop = (_0: string): Msg => ({ _tag: "Stop", _0 });

async function timer_loop(state: TimerState): Promise<void> {
  const msg = await receive<Msg>();
  switch (msg._tag) {
    case "Tick": return timer_loop(tick(state, msg._0));
    case "Stop": return timer_loop(stop(state, msg._0));
  }
}

async function main(): Promise<void> {
  const pid = spawn(() => timer_loop(initial_state()));
  send(pid, Tick("user-1"));
}
```

---

## Phase Map

```
Phase 1: Compiler Core (TypeScript)              ~2 weeks
  └─ Lexer → Parser → Type Checker → Codegen → CLI
  └─ Output: japl build produces .ts files

Phase 2: Runtime (@japl/runtime)                  ~1 week
  └─ Process scheduler, mailbox, supervisor, effects
  └─ Output: npm package, works with generated code

Phase 3: Standard Library                         ~2 weeks
  └─ Core, String, List, Map, Option, Result, IO, Process,
     Json, Http, Time, File, Test, Db
  └─ Output: stdlib modules importable from JAPL

Phase 4: Tooling                                  ~1 week
  └─ CLI (build/run/test/fmt/new), formatter, test runner
  └─ Output: japl CLI that feels like go/gleam

Phase 5: Self-Hosting                             ~2 weeks
  └─ Rewrite compiler in JAPL, compile to TS, run on Node
  └─ Output: JAPL compiler compiles itself

Phase 6: Proof App — TimeTracker                  ~2 weeks
  └─ Full distributed time tracking app
  └─ Output: running app proving JAPL works
```

**Total: ~10 weeks for a fully self-hosting language with a proof application.**

---

## Phase 1: Compiler Core

**Goal**: `japl build` takes `.japl` files and produces `.ts` files.

**Language**: TypeScript (later self-hosted in Phase 5)

**Location**: `/compiler/ts/` (new TS-based compiler)

### Directory Structure

```
compiler/ts/
  package.json
  tsconfig.json
  src/
    index.ts              # CLI entry point
    lexer/
      token.ts            # Token type definitions
      lexer.ts            # Tokenizer (DFA-based)
      lexer.test.ts       # Tests
    parser/
      ast.ts              # AST node definitions
      parser.ts           # Pratt + recursive descent parser
      parser.test.ts      # Tests
    checker/
      types.ts            # Type representations
      env.ts              # Type environment
      unify.ts            # Unification engine
      infer.ts            # Bidirectional type inference
      effects.ts          # Effect checking
      linearity.ts        # Resource tracking
      checker.test.ts     # Tests
    ir/
      ir.ts               # IR node definitions
      lower.ts            # AST → IR lowering
    codegen/
      emit.ts             # IR → TypeScript code generation
      runtime-imports.ts  # @japl/runtime import generation
      codegen.test.ts     # Tests
    cli/
      build.ts            # japl build command
      run.ts              # japl run command (build + execute)
      init.ts             # japl new command
```

### Token Types (lexer/token.ts)

```typescript
export enum TokenKind {
  // Literals
  Int, Float, String, True, False,

  // Identifiers
  Ident, UpperIdent,  // foo vs Foo (constructors)

  // Keywords
  Fn, Let, Type, Match, If, Else, Then,
  Trait, Impl, Module, Import, Pub, Opaque,
  Spawn, Send, Receive, Supervisor, Process,
  Test, Assert, Foreign, Unsafe,
  Use, Return, Done, Fail, Panic,

  // Operators
  Plus, Minus, Star, Slash, Percent,
  Eq, NotEq, Lt, Gt, LtEq, GtEq,
  And, Or, Not,
  Pipe,       // |>
  Arrow,      // ->
  FatArrow,   // =>
  Question,   // ?
  Concat,     // <>
  Assign,     // =
  Colon, ColonColon, Dot, DotDot,
  Comma, Semicolon,

  // Delimiters
  LParen, RParen, LBrace, RBrace, LBracket, RBracket,

  // Special
  Newline, EOF, Comment,
}
```

### AST Nodes (parser/ast.ts)

```typescript
// Top-level declarations
export type Decl =
  | FnDecl          // fn name(params) -> RetType { body }
  | TypeDecl        // type Name = | A(T) | B(T)
  | TraitDecl       // trait Name(a) { fn method(a) -> T }
  | ImplDecl        // impl Trait for Type { ... }
  | ModuleDecl      // module Name { ... }
  | ImportDecl      // import Module.{a, b}
  | TestDecl        // test "name" { ... }
  | SupervisorDecl  // supervisor Name { strategy: ..., child ... }
  | ForeignDecl     // foreign fn name(...) -> T

// Expressions
export type Expr =
  | Literal         // 42, "hello", true
  | Var             // x
  | Constructor     // Some(x), Err(e)
  | App             // f(x, y)
  | Lambda          // fn(x, y) { body }
  | Let             // let x = expr
  | Match           // match expr { pat => body, ... }
  | If              // if cond { then } else { else }
  | Pipe            // expr |> fn
  | BinOp           // a + b
  | UnaryOp         // !x, -x
  | Record          // { name: "alice", age: 30 }
  | FieldAccess     // record.field
  | RecordUpdate    // { record | field: newval }
  | List            // [1, 2, 3]
  | Block           // { expr1; expr2 }
  | Spawn           // spawn(fn)
  | Send            // send(pid, msg)
  | Receive         // receive { pat => body }
  | Try             // expr?
  | Do              // do { effectful }

// Patterns
export type Pattern =
  | PVar            // x
  | PConstructor    // Some(x)
  | PLiteral        // 42, "hello"
  | PWildcard       // _
  | PRecord         // { name, age }
  | PList           // [head, ...tail]
  | PGuard          // pat if condition
```

### Type System (checker/types.ts)

```typescript
export type Type =
  | { kind: "int" }
  | { kind: "float" }
  | { kind: "string" }
  | { kind: "bool" }
  | { kind: "unit" }
  | { kind: "never" }
  | { kind: "var"; id: number }                          // Unification variable
  | { kind: "named"; name: string; args: Type[] }        // User type
  | { kind: "fn"; params: Type[]; ret: Type; effects: EffectRow }
  | { kind: "record"; fields: [string, Type][]; row?: number }
  | { kind: "tuple"; elements: Type[] }
  | { kind: "process"; msg: Type }                       // Process<Msg>
  | { kind: "pid"; msg: Type }                           // Pid<Msg>

export type Effect =
  | { kind: "pure" }
  | { kind: "io" }
  | { kind: "async" }
  | { kind: "process"; msg: Type }
  | { kind: "fail"; err: Type }

export type EffectRow = {
  effects: Effect[];
  row?: number;  // Row variable for polymorphic effects
}
```

### CodeGen Rules (codegen/emit.ts)

| JAPL | TypeScript |
|------|-----------|
| `type Msg = \| A(T) \| B(T)` | `type Msg = { _tag: "A"; _0: T } \| { _tag: "B"; _0: T }` + constructor functions |
| `fn f(x: Int) -> Int` | `function f(x: number): number` |
| `let x = expr` | `const x = expr` |
| `match expr { A(x) => ... }` | `switch (expr._tag) { case "A": ... }` |
| `expr \|> f` | `f(expr)` |
| `expr \|> f(a)` | `f(a)(expr)` or `f(expr, a)` (configurable) |
| `spawn(f)` | `spawn(() => f())` |
| `send(pid, msg)` | `send(pid, msg)` |
| `receive { ... }` | `await receive()` + switch |
| `expr?` | Early return pattern (Result unwrap) |
| `{ x \| field: val }` | `{ ...x, field: val }` |
| `[1, 2, 3]` | `[1, 2, 3]` (immutable enforced by types) |
| `module M { ... }` | Separate file / namespace |
| `trait Show(a) { ... }` | Interface + dictionary passing or TypeScript interface |
| `impl Show for User { ... }` | Implementation object |
| `supervisor { ... }` | `new Supervisor({ ... })` |

### Tests Required (Compiler Core)

- [ ] Lexer: all token types, edge cases (nested strings, comments, operators)
- [ ] Parser: every AST node type, error recovery, precedence
- [ ] Type checker: inference for all expression forms, unification, generalization
- [ ] Effect checker: IO propagation, purity enforcement, effect composition
- [ ] Codegen: round-trip tests (JAPL → TS → execute → verify output)
- [ ] Integration: full programs compile and run correctly

---

## Phase 2: TypeScript Runtime

**Goal**: `@japl/runtime` npm package that generated code imports.

**Location**: `/runtime/`

### Directory Structure

```
runtime/
  package.json          # @japl/runtime
  tsconfig.json
  src/
    index.ts            # Public API exports
    process.ts          # Process abstraction
    pid.ts              # Process ID type
    scheduler.ts        # Cooperative scheduler on event loop
    mailbox.ts          # Typed message queue per process
    supervisor.ts       # Supervision tree implementation
    result.ts           # Result<T,E>, Option<T>
    effect.ts           # IO effect boundary
    ref.ts              # Process-local mutable reference
  test/
    process.test.ts
    scheduler.test.ts
    mailbox.test.ts
    supervisor.test.ts
```

### Process Model (process.ts)

```typescript
export type ProcessId = string;  // UUID

export type ProcessState = "running" | "waiting" | "done" | "failed";

export interface Process<Msg> {
  id: ProcessId;
  state: ProcessState;
  mailbox: Mailbox<Msg>;
  parent: ProcessId | null;
  links: Set<ProcessId>;
  monitors: Set<ProcessId>;
}

export function spawn<Msg>(fn: () => Promise<void>): ProcessId;
export function send<Msg>(pid: ProcessId, msg: Msg): void;
export function receive<Msg>(): Promise<Msg>;
export function self(): ProcessId;
export function link(pid: ProcessId): void;
export function monitor(pid: ProcessId): void;
```

### Scheduler (scheduler.ts)

The scheduler runs on the JavaScript event loop using `Promise` and `setTimeout(0)`:

```typescript
// Each process is a suspended async function
// The scheduler round-robins through runnable processes
// yield points: receive(), send(), spawn()
// Integrates with Node.js event loop (not blocking)

export class Scheduler {
  private processes: Map<ProcessId, ProcessContext>;
  private runQueue: ProcessId[];

  spawn(fn: () => Promise<void>): ProcessId;
  schedule(): void;  // Called on microtask queue
  deliver(pid: ProcessId, msg: unknown): void;
}
```

### Supervisor (supervisor.ts)

```typescript
export type Strategy = "one_for_one" | "all_for_one" | "rest_for_one";

export interface ChildSpec {
  id: string;
  start: () => ProcessId;
  restart: "permanent" | "transient" | "temporary";
}

export interface SupervisorSpec {
  strategy: Strategy;
  maxRestarts: number;
  maxSeconds: number;
  children: ChildSpec[];
}

export function startSupervisor(spec: SupervisorSpec): ProcessId;
```

### Result / Option (result.ts)

```typescript
export type Result<T, E> =
  | { _tag: "Ok"; value: T }
  | { _tag: "Err"; error: E };

export type Option<T> =
  | { _tag: "Some"; value: T }
  | { _tag: "None" };

export const Ok = <T>(value: T): Result<T, never> => ({ _tag: "Ok", value });
export const Err = <E>(error: E): Result<never, E> => ({ _tag: "Err", error });
export const Some = <T>(value: T): Option<T> => ({ _tag: "Some", value });
export const None: Option<never> = { _tag: "None" };

// Monadic operations
export function map<T, U, E>(r: Result<T, E>, f: (t: T) => U): Result<U, E>;
export function flatMap<T, U, E>(r: Result<T, E>, f: (t: T) => Result<U, E>): Result<U, E>;
export function unwrapOr<T, E>(r: Result<T, E>, def: T): T;
```

---

## Phase 3: Standard Library

**Goal**: JAPL modules that cover common programming needs.

**Location**: `/stdlib/` (written in JAPL, compiles to TS)

### Modules

| Module | Functions | Priority |
|--------|-----------|----------|
| `Core` | `identity`, `compose`, `pipe`, `const`, `flip`, `show`, `debug` | P0 |
| `String` | `length`, `concat`, `split`, `trim`, `contains`, `replace`, `to_upper`, `to_lower`, `starts_with`, `ends_with`, `slice` | P0 |
| `List` | `map`, `filter`, `fold`, `reduce`, `head`, `tail`, `append`, `concat`, `reverse`, `sort`, `find`, `any`, `all`, `zip`, `flat_map`, `length`, `take`, `drop`, `chunk` | P0 |
| `Map` | `new`, `get`, `put`, `delete`, `keys`, `values`, `entries`, `merge`, `map`, `filter`, `size`, `has` | P0 |
| `Set` | `new`, `add`, `remove`, `has`, `union`, `intersect`, `diff`, `size`, `to_list` | P1 |
| `Option` | `Some`, `None`, `map`, `flat_map`, `unwrap_or`, `is_some`, `is_none`, `to_result` | P0 |
| `Result` | `Ok`, `Err`, `map`, `flat_map`, `map_err`, `unwrap_or`, `is_ok`, `is_err`, `to_option` | P0 |
| `IO` | `println`, `print`, `read_line`, `read_file`, `write_file`, `env_var` | P0 |
| `Process` | `spawn`, `send`, `receive`, `self`, `link`, `monitor`, `exit`, `sleep` | P0 |
| `Supervisor` | `start`, `one_for_one`, `all_for_one`, `child_spec` | P0 |
| `Json` | `encode`, `decode`, `parse`, `stringify` | P1 |
| `Http` | `get`, `post`, `put`, `delete`, `serve`, `request`, `response`, `router` | P1 |
| `Time` | `now`, `utc`, `duration`, `add`, `diff`, `format`, `parse`, `sleep` | P1 |
| `File` | `read`, `write`, `append`, `exists`, `delete`, `list_dir`, `mkdir` | P1 |
| `Crypto` | `hash_sha256`, `random_bytes`, `uuid`, `jwt_sign`, `jwt_verify` | P2 |
| `Db` | `connect`, `query`, `execute`, `transaction`, `pool` | P2 |
| `Test` | `assert`, `assert_eq`, `assert_err`, `describe`, `it`, `property` | P0 |
| `Debug` | `inspect`, `trace`, `log`, `time` | P1 |

### Example: List module (stdlib/List.japl)

```
module List

pub fn map(list: List<a>, f: fn(a) -> b) -> List<b> {
  match list {
    [] => []
    [head, ...tail] => [f(head), ...map(tail, f)]
  }
}

pub fn filter(list: List<a>, pred: fn(a) -> Bool) -> List<a> {
  match list {
    [] => []
    [head, ...tail] =>
      if pred(head) { [head, ...filter(tail, pred)] }
      else { filter(tail, pred) }
  }
}

pub fn fold(list: List<a>, init: b, f: fn(b, a) -> b) -> b {
  match list {
    [] => init
    [head, ...tail] => fold(tail, f(init, head), f)
  }
}
```

---

## Phase 4: Tooling

**Goal**: `japl` CLI that feels like `go` or `gleam`.

### CLI Commands

```
japl new <name>         # Create new project
japl build              # Compile .japl → .ts → .js
japl run                # Build and execute
japl test               # Run tests
japl fmt                # Format all .japl files
japl add <pkg>          # Add dependency
japl publish            # Publish to registry
japl repl               # Interactive REPL
japl check              # Type check only (no codegen)
japl doc                # Generate documentation
```

### Project Structure (japl new myapp)

```
myapp/
  japl.toml              # Project manifest
  src/
    main.japl            # Entry point
  test/
    main_test.japl       # Tests
  .gitignore
  README.md
```

### japl.toml

```toml
[package]
name = "myapp"
version = "0.1.0"
entry = "src/main.japl"

[dependencies]
http = "0.1.0"
json = "0.1.0"

[dev-dependencies]
test = "0.1.0"
```

---

## Phase 5: Self-Hosting

**Goal**: JAPL compiler written in JAPL, compiles to TypeScript, runs on Node.

### Bootstrap Chain

```
Step 1: TypeScript compiler (Phase 1) compiles JAPL stdlib → TS
Step 2: TypeScript compiler compiles JAPL compiler source → TS
Step 3: Run generated TS compiler on Node.js
Step 4: Generated compiler compiles its own source → TS (stage 1)
Step 5: Verify stage 0 output == stage 1 output
Step 6: JAPL is self-hosting ✓
```

### Files to Write in JAPL

```
compiler/japl/
  src/
    lexer.japl          # Tokenizer
    token.japl          # Token types
    parser.japl         # Parser
    ast.japl            # AST nodes
    types.japl          # Type representations
    checker.japl        # Type checker
    effects.japl        # Effect checker
    ir.japl             # IR nodes
    lower.japl          # AST → IR
    codegen.japl        # IR → TypeScript
    emit.japl           # String builder / code emission
    driver.japl         # CLI entry point
```

---

## Phase 6: Proof App — TimeTracker

**Goal**: A distributed time tracking application (like Harvest) built entirely in JAPL.

### Features

1. **User Management** — registration, login, JWT auth
2. **Projects** — create, list, archive projects
3. **Time Entries** — start/stop timer, manual entry, edit, delete
4. **Reports** — daily/weekly/monthly summaries, per-project breakdown
5. **Teams** — invite members, assign to projects, view team time
6. **Real-time** — live timer sync via WebSocket
7. **Notifications** — timer reminders, weekly summaries
8. **Export** — CSV export of time entries
9. **API** — RESTful JSON API (no frontend required, but API-complete)

### Architecture

```
timetracker/
  japl.toml
  src/
    main.japl                 # Entry point, supervisor tree
    config.japl               # Configuration loading

    # Domain
    domain/
      user.japl               # User type, validation
      project.japl            # Project type
      time_entry.japl         # TimeEntry type, timer logic
      team.japl               # Team type

    # API Layer
    api/
      router.japl             # HTTP router
      middleware/
        auth.japl             # JWT authentication
        cors.japl             # CORS headers
        logger.japl           # Request logging
      handlers/
        users.japl            # User endpoints
        projects.japl         # Project endpoints
        entries.japl          # Time entry endpoints
        reports.japl          # Report endpoints
        teams.japl            # Team endpoints
        ws.japl               # WebSocket handler

    # Data Layer
    db/
      connection.japl         # PostgreSQL connection pool
      migrations.japl         # Schema migrations
      queries/
        user_queries.japl     # User SQL queries
        project_queries.japl  # Project SQL queries
        entry_queries.japl    # Time entry SQL queries

    # Process Layer
    processes/
      timer_server.japl       # Active timer process (per user)
      notification_worker.japl # Notification sender
      report_generator.japl   # Background report generation
      ws_broadcaster.japl     # WebSocket broadcast process

    # Supervision
    supervisor.japl           # App supervisor tree

  test/
    domain_test.japl          # Unit tests
    api_test.japl             # Integration tests
    process_test.japl         # Process tests

  migrations/
    001_create_users.sql
    002_create_projects.sql
    003_create_entries.sql
    004_create_teams.sql
```

### Supervisor Tree

```
App Supervisor (one_for_one)
├── HTTP Server Process
├── WebSocket Server Process
├── DB Connection Pool Supervisor (one_for_one)
│   ├── Connection 1
│   ├── Connection 2
│   └── Connection N
├── Timer Supervisor (one_for_one)
│   ├── TimerServer(user-1)
│   ├── TimerServer(user-2)
│   └── TimerServer(user-N)
├── Notification Worker
└── Report Generator
```

### Key JAPL Code (timetracker/src/main.japl)

```
module Main

import Http.{serve}
import Supervisor.{start, one_for_one, child_spec}
import Db.{connect_pool}
import Process.{spawn}

import Api.Router.{router}
import Processes.TimerServer
import Processes.NotificationWorker
import Processes.ReportGenerator

fn main() -> ![IO] Unit {
  let db = connect_pool("postgres://localhost/timetracker", 10)?
  let timer_sup = start(one_for_one(3, 60), [])

  let app = start(one_for_one(5, 60), [
    child_spec("http", fn() { serve(router(db, timer_sup), 8080) }),
    child_spec("notifications", fn() { NotificationWorker.start(db) }),
    child_spec("reports", fn() { ReportGenerator.start(db) }),
  ])

  println("TimeTracker running on :8080")
  Process.sleep_forever()
}
```

### API Endpoints

| Method | Path | Handler |
|--------|------|---------|
| POST | /api/auth/register | Register user |
| POST | /api/auth/login | Login, get JWT |
| GET | /api/users/me | Get current user |
| GET | /api/projects | List projects |
| POST | /api/projects | Create project |
| GET | /api/projects/:id | Get project |
| POST | /api/entries/start | Start timer |
| POST | /api/entries/stop | Stop timer |
| GET | /api/entries | List entries (with filters) |
| POST | /api/entries | Manual entry |
| PUT | /api/entries/:id | Edit entry |
| DELETE | /api/entries/:id | Delete entry |
| GET | /api/reports/daily | Daily report |
| GET | /api/reports/weekly | Weekly report |
| GET | /api/reports/project/:id | Project report |
| GET | /api/teams | List teams |
| POST | /api/teams | Create team |
| POST | /api/teams/:id/invite | Invite member |
| GET | /ws | WebSocket for live updates |

### Database Schema

```sql
CREATE TABLE users (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  email TEXT UNIQUE NOT NULL,
  password_hash TEXT NOT NULL,
  name TEXT NOT NULL,
  created_at TIMESTAMPTZ DEFAULT now()
);

CREATE TABLE projects (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  name TEXT NOT NULL,
  description TEXT,
  color TEXT DEFAULT '#4f8ff7',
  owner_id UUID REFERENCES users(id),
  archived BOOLEAN DEFAULT false,
  created_at TIMESTAMPTZ DEFAULT now()
);

CREATE TABLE time_entries (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  user_id UUID REFERENCES users(id) NOT NULL,
  project_id UUID REFERENCES projects(id) NOT NULL,
  description TEXT,
  started_at TIMESTAMPTZ NOT NULL,
  stopped_at TIMESTAMPTZ,  -- NULL = timer running
  duration_seconds INT,
  created_at TIMESTAMPTZ DEFAULT now()
);

CREATE TABLE teams (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  name TEXT NOT NULL,
  created_at TIMESTAMPTZ DEFAULT now()
);

CREATE TABLE team_members (
  team_id UUID REFERENCES teams(id),
  user_id UUID REFERENCES users(id),
  role TEXT DEFAULT 'member',
  PRIMARY KEY (team_id, user_id)
);

CREATE TABLE project_teams (
  project_id UUID REFERENCES projects(id),
  team_id UUID REFERENCES teams(id),
  PRIMARY KEY (project_id, team_id)
);
```

---

## Team Structure

| Role | Responsibilities | Phase |
|------|-----------------|-------|
| **Compiler Engineer** | Lexer, parser, type checker, codegen (TS target) | 1, 5 |
| **Runtime Engineer** | @japl/runtime (scheduler, mailbox, supervisor) | 2 |
| **Stdlib Developer** | Standard library modules in JAPL | 3 |
| **Tooling Engineer** | CLI, formatter, test runner, LSP | 4 |
| **App Developer** | TimeTracker application in JAPL | 6 |
| **DevOps** | CI/CD, npm publishing, deployment | 4, 6 |

### Parallel Execution

```
Week 1-2:  Compiler Engineer → Phase 1
           Runtime Engineer  → Phase 2 (parallel)

Week 3-4:  Stdlib Developer  → Phase 3 (needs Phase 1+2)
           Tooling Engineer  → Phase 4 (needs Phase 1)

Week 5-6:  Compiler Engineer → Phase 5 (self-hosting)

Week 7-10: App Developer     → Phase 6 (needs Phase 1-4)
           All hands on bugs + polish
```

---

## Acceptance Criteria

### Phase 1 Complete When:
- [ ] `japl build src/main.japl` produces valid TypeScript
- [ ] All JAPL syntax features parse correctly
- [ ] Type inference works for all expression forms
- [ ] Effect checking catches IO in pure functions
- [ ] Generated TypeScript compiles with `tsc --strict`
- [ ] 50+ compiler tests passing

### Phase 2 Complete When:
- [ ] `@japl/runtime` published to npm
- [ ] Can spawn 10,000 processes without OOM
- [ ] Message send/receive works across processes
- [ ] Supervisor restarts crashed children
- [ ] Integrates with Node.js event loop (non-blocking)
- [ ] 30+ runtime tests passing

### Phase 3 Complete When:
- [ ] All P0 stdlib modules implemented and tested
- [ ] Can import stdlib from JAPL programs
- [ ] Generated TS for stdlib is clean and readable
- [ ] 100+ stdlib tests passing

### Phase 4 Complete When:
- [ ] `japl new`, `japl build`, `japl run`, `japl test`, `japl fmt` all work
- [ ] `japl.toml` project format defined and parsed
- [ ] Formatter produces canonical output
- [ ] Test runner discovers and runs test blocks

### Phase 5 Complete When:
- [ ] Compiler source is 100% JAPL (no TypeScript)
- [ ] Stage-0 (TS compiler) compiles JAPL compiler → TS
- [ ] Stage-1 (generated compiler) compiles JAPL compiler → TS
- [ ] Stage-0 output == Stage-1 output (bootstrap verified)
- [ ] Old TS compiler can be deleted

### Phase 6 Complete When:
- [ ] TimeTracker API runs and serves all endpoints
- [ ] PostgreSQL integration works (CRUD for all entities)
- [ ] Timer start/stop works with process-per-user model
- [ ] WebSocket broadcasts timer updates in real-time
- [ ] Supervisor tree restarts crashed processes
- [ ] JWT authentication works end-to-end
- [ ] Reports generate correct summaries
- [ ] 50+ app tests passing
- [ ] Deployed and accessible

### JAPL "Done" When:
- [ ] All 6 phases complete
- [ ] Compiler self-hosts (bootstrap verified)
- [ ] TimeTracker app running in production
- [ ] `japl` CLI installable (`npm install -g japl`)
- [ ] Documentation site live
- [ ] At least one real user has built something with JAPL

---

## File-Level Implementation Guide

This section provides enough detail for any AI agent or developer to implement each file.

### How to Start (for an AI agent picking this up)

1. Read this PLAN.md
2. Read `/spec/japl-spec.md` for the full language specification
3. Read `/spec/compiler-architecture.md` for compiler design details
4. Start with Phase 1 — the compiler is the foundation
5. Work through phases sequentially (1 → 2 → 3 → 4 → 5 → 6)
6. Run tests after every change
7. The existing Rust compiler at `/compiler/` is reference implementation — use it to understand semantics

### Key Design Decisions

1. **Tagged unions as discriminated unions**: JAPL `type Msg = | A(T)` becomes TypeScript `type Msg = { _tag: "A"; _0: T }` with constructor functions
2. **Effects as async**: IO/Process effects compile to `async/await` in TypeScript
3. **Processes as async functions**: Each process is an async function managed by the scheduler
4. **Pipe as function application**: `x |> f` compiles to `f(x)`
5. **Pattern matching as switch**: On the `_tag` field for tagged unions
6. **Traits as dictionaries**: Trait instances are objects passed explicitly (dictionary passing)
7. **Modules as files**: Each JAPL module is a separate `.ts` file
8. **Result/Option**: Reuse runtime implementations, not try/catch

### Reference Implementations

- **Gleam** (gleam-lang/gleam): Erlang/JS target, similar philosophy
- **ReScript** (rescript-lang/rescript): ML → JS, clean output
- **PureScript** (purescript/purescript): Haskell-like → JS
- **Elm** (elm/compiler): ML → JS, excellent error messages
- **Roc** (roc-lang/roc): Fast FP, value semantics

Study Gleam's JS codegen especially — it's the closest precedent to what JAPL does.
