# JAPL Deep Dive: Language Design & Functionality Gaps

> Honest assessment of where JAPL stands as of 2026-03-30.
> 514 tests passing. Docker distributed tests green.
> But a language is more than tests.

---

## I. Language Design Gaps

### 1. No Module System (CRITICAL)

**Status:** Parsed but not functional.

Every real program needs multiple files. JAPL currently compiles one file at a time with no import resolution.

```
// This parses but doesn't work:
import List.{map, filter}
import Http.{serve, router}
```

**What's missing:**
- Import resolution (find the file, parse it, check types)
- Dependency graph (topological sort, detect cycles)
- Module-qualified names (`List.map` vs local `map`)
- Public/private visibility enforcement
- Separate compilation (compile each module to .ts, link at the end)
- Package paths (`import "github.com/user/pkg"` or similar)

**Impact:** Without this, every JAPL program must be a single file. No real software can be built.

**Fix:** 2 agents, ~1 session. This is the #1 priority after distribution.

---

### 2. Effect System Not Enforced (MAJOR)

**Status:** Checker tracks effects but codegen ignores them.

```
// This type-checks with a warning but generates working code:
fn pure_function(x: Int) -> Int {
  println("side effect!")  // Should be rejected — IO in pure function
  x + 1
}
```

**What's missing:**
- Codegen should refuse to emit code that violates effect annotations
- Effect handlers (handling Fail, catching Process crashes)
- Effect polymorphism in practice (functions generic over effects)

**Impact:** JAPL claims "effect-aware" but doesn't enforce it. The compiler checks and warns but generates the code anyway.

**Fix:** 1 agent. Wire effect checker verdict into codegen — reject compilation if effects are violated.

---

### 3. Linearity/Ownership Not Enforced (MAJOR)

**Status:** Checker has linearity checking but codegen ignores it.

```
// This should be rejected — double use of a linear resource:
fn bad(file: Owned<File>) {
  read(file)
  read(file)  // Should be compile error: file already consumed
}
```

**What's missing:**
- Codegen should refuse to emit code that violates linearity
- Runtime enforcement for the C backend (resource tracking)
- The `Owned<T>` and `Ref<T>` types don't affect generated code at all

**Impact:** JAPL claims "resource-safe by construction" but the construction doesn't actually enforce safety.

**Fix:** 1 agent. Wire linearity checker into codegen. For TS target: add runtime ownership tracking wrapper. For C target: already has resource tracking in the C runtime.

---

### 4. Pattern Matching Not Exhaustive (MODERATE)

**Status:** Works but doesn't check if all cases are covered.

```
type Shape =
  | Circle(Float)
  | Rectangle(Float, Float)
  | Triangle(Float, Float, Float)

fn area(shape: Shape) -> Float {
  match shape {
    Circle(r) => 3.14 * r * r
    Rectangle(w, h) => w * h
    // Missing Triangle — compiles fine, crashes at runtime
  }
}
```

**What's missing:**
- Exhaustiveness checking (Maranget's algorithm or simpler)
- Compiler error when not all variants are covered
- Suggestion of missing patterns in error message

**Impact:** Runtime crashes from missing match arms instead of compile-time errors.

**Fix:** 1 agent. Implement exhaustiveness checking in the type checker.

---

### 5. No String Interpolation (MINOR)

**Status:** Strings are concatenated with `<>` only.

```
// Current (verbose):
"Hello " <> name <> ", you are " <> show(age) <> " years old"

// Desired:
"Hello ${name}, you are ${age} years old"
```

**Fix:** 1 agent. Lexer + parser change, desugar to `<>` in IR.

---

### 6. No Generics in Practice (MODERATE)

**Status:** Type inference works but generated code doesn't monomorphize or use dictionary passing.

```
fn identity(x: a) -> a { x }
let n = identity(42)      // Works in type checker
let s = identity("hello") // Works in type checker
// But generated code just uses `any` in TypeScript
```

**What's missing:**
- Monomorphization (generate separate `identity_int`, `identity_string`)
- Or dictionary passing for trait constraints
- The generated TS code uses `unknown`/`any` instead of proper types

**Impact:** Type safety exists at check time but disappears in the generated code.

**Fix:** 2 agents. Monomorphization pass in IR, or emit TypeScript generics.

---

### 7. No Closures Over Mutable State (BY DESIGN — but needs documentation)

JAPL is pure-by-default. Closures capture values, not references. This is correct but needs clear documentation because developers from JS/Python will expect reference capture.

```
let x = 42
let f = fn() { x }  // Captures the VALUE 42, not a reference to x
// If x could be mutated (it can't, it's immutable), f still returns 42
```

---

### 8. Tail Call Optimization (MODERATE)

**Status:** Not implemented in either backend.

```
fn loop(n: Int) {
  if n <= 0 { 0 }
  else { loop(n - 1) }  // Stack overflow on large n
}
```

**What's missing:**
- TCO in TS codegen (convert tail calls to while loops)
- TCO in C codegen (already natural in C with optimization flags, but not guaranteed)

**Impact:** Recursive process loops (the Erlang pattern) will stack overflow. This is a serious gap for the actor model.

**Fix:** 1 agent. Transform tail-recursive calls to while loops in the IR.

---

## II. Functionality Gaps

### 9. Standard Library Is Not Runnable (CRITICAL)

**Status:** 7 .japl files exist but none compile through the pipeline and produce working code.

```
stdlib/
  Core.japl     — written but uses features the compiler doesn't support
  List.japl     — uses [head, ...tail] which the parser handles but codegen doesn't
  Option.japl   — depends on multi-file imports
  Result.japl   — depends on multi-file imports
  String.japl   — uses foreign functions not linked
  Process.japl  — uses spawn/send which are keywords not functions
  Test.japl     — uses assert which is a keyword
```

**Impact:** No stdlib = no real programs. Every JAPL program must redefine basic operations.

**Fix:** Blocked by module system (Gap #1). Once imports work, 4 agents to make stdlib compile and test.

---

### 10. Foreign Function Interface Incomplete (MAJOR)

**Status:** `foreign` declarations parse but don't connect to anything.

```
// This parses:
foreign fn read_file(path: String) -> String

// But the generated code doesn't link to any implementation
```

**What's missing:**
- TS target: emit `import` or `require` for foreign functions
- C target: emit `extern` declarations
- A way to specify which JS/C module provides the foreign function
- Safety wrapper generation

**Fix:** 1 agent. Add `foreign` annotations that map to JS modules or C headers.

---

### 11. No Real IO (CRITICAL for usefulness)

**Status:** `println` works via a hardcoded builtin. Nothing else.

**What's missing (for TS target):**
- File read/write (`fs` module)
- HTTP client/server (`http` module or fetch)
- JSON encode/decode
- Environment variables
- Command line arguments
- Process.exit
- Stdin reading
- TCP/UDP sockets
- Timers (setTimeout, setInterval)

**Impact:** Can't build anything useful beyond "hello world" and fibonacci.

**Fix:** Blocked by FFI (Gap #10). Once FFI works, implement as foreign bindings to Node.js APIs.

---

### 12. Distribution Is Simulation Only (HONEST ASSESSMENT)

**Status:** The wire protocol, serialization, connections, and routing are built (217 runtime tests). But the Docker test programs don't actually use distribution — they just print messages.

**What's actually working:**
- Wire protocol (frame encode/decode) ✓
- Value serialization (all types round-trip) ✓
- TCP connections with handshake ✓
- Heartbeat health monitoring ✓
- Distributed PID routing logic ✓
- Remote spawn/monitor logic ✓
- Distributed supervisor logic ✓

**What's NOT working:**
- Generated JAPL code can't call `spawn_remote()`, `send()` to remote PIDs, etc.
- The runtime is built but not wired into the codegen
- No integration between the compiler output and the distributed runtime
- The Docker test just runs two separate programs that don't actually communicate

**The gap:** The distributed runtime exists as a tested TypeScript library, but there's no way to USE it from JAPL code. The codegen generates `spawn()` and `send()` calls that hit the LOCAL scheduler, not the distributed router.

**Fix:** 1 agent. Wire the distributed runtime into the generated code:
- When `--node` flag is present, generated code imports `DistributedRuntime` instead of local `Scheduler`
- `spawn()` → `runtime.spawn()`
- `send(pid, msg)` → `runtime.send(pid, msg)`
- `receive()` → `runtime.receive()`
- Add `spawn_remote()`, `node_name()`, `register()`, `lookup()` as builtins

---

### 13. No Playground (NICE TO HAVE)

An online editor where you can write JAPL and see generated TypeScript + run it. Like play.golang.org.

**Fix:** Could be built as a Vercel serverless function that runs the compiler. Or a client-side WASM build of the compiler.

---

## III. Documentation Gaps

### 14. No Getting Started Guide

Someone clones the repo. Now what? There's no:
- Install instructions
- "Hello World" walkthrough
- "First project" tutorial
- "Build something real" guide

### 15. No Language Guide

No explanation of:
- How types work
- How pattern matching works
- How effects work
- How processes work
- How distribution works
- When to use Result vs panic

### 16. No API/Stdlib Reference

No generated documentation for:
- Standard library modules
- Type signatures
- Function descriptions
- Examples per function

### 17. No Architecture Guide

No explanation of:
- How the compiler works
- How the runtime works
- How to contribute
- How to add a new backend

---

## IV. Priority Order

```
MUST FIX (language is non-functional without these):
  1.  Module system                    — can't write real programs
  9.  Stdlib actually compiles         — blocked by #1
  10. FFI connects to real code        — can't do IO without it
  11. Real IO operations               — blocked by #10
  8.  Tail call optimization           — actor loops stack overflow
  12. Distribution wired to codegen    — the core promise

SHOULD FIX (language works but claims are broken):
  2.  Effect enforcement in codegen
  3.  Linearity enforcement in codegen
  4.  Exhaustive pattern matching
  6.  Generated code has real types (not any/unknown)

NICE TO HAVE:
  5.  String interpolation
  7.  Documentation of closure semantics
  13. Playground
  14-17. Documentation site
```

---

## V. Documentation Site Plan (japl-lang.dev)

### Architecture

```
japl-lang.dev/                   Static site (Vercel)
├── index.html                   Hero + overview
├── docs/
│   ├── getting-started.html     Install + hello world + first project
│   ├── tour/
│   │   ├── 01-values.html       Immutable values
│   │   ├── 02-types.html        ADTs and records
│   │   ├── 03-functions.html    Functions and pipes
│   │   ├── 04-matching.html     Pattern matching
│   │   ├── 05-errors.html       Result and Option
│   │   ├── 06-effects.html      Effect system
│   │   ├── 07-processes.html    Actors and mailboxes
│   │   ├── 08-supervision.html  Supervisor trees
│   │   ├── 09-distribution.html Remote nodes
│   │   └── 10-building.html     Build and deploy
│   ├── guide/                   Deep dive per topic
│   │   ├── values.html
│   │   ├── types.html
│   │   ├── functions.html
│   │   ├── pattern-matching.html
│   │   ├── error-handling.html
│   │   ├── effects.html
│   │   ├── processes.html
│   │   ├── supervision.html
│   │   ├── distribution.html
│   │   ├── modules.html
│   │   ├── ownership.html
│   │   └── numbers.html
│   ├── reference/
│   │   ├── spec.html            Full language spec
│   │   ├── grammar.html         EBNF grammar
│   │   ├── cli.html             CLI reference
│   │   └── wire-protocol.html   Distribution protocol
│   ├── stdlib/                  One page per module
│   └── examples/                Runnable examples
├── research/                    Paper links
├── blog/                        Blog posts
├── style.css                    Dark theme
└── og-image.png                 Social media image
```

### Design Spec

- **Framework:** Static HTML/CSS (no framework, like current sites)
- **Theme:** Dark (match existing sites), accent: purple (#7c5cfc)
- **Fonts:** Inter + JetBrains Mono
- **Code blocks:** Syntax highlighted JAPL with custom CSS
- **Navigation:** Sidebar on desktop, hamburger on mobile
- **Search:** Client-side search via FlexSearch or similar
- **Math:** MathJax for any formal notation
- **Mobile:** 100% responsive, 375px+

### Agent Team

```
Agent 1: Site Foundation + Getting Started        (index.html, getting-started, nav, styles)
Agent 2: Language Tour (10 pages)                 (tour/01-10)
Agent 3: Language Guide (12 deep-dive pages)      (guide/*)
Agent 4: Reference + Stdlib                       (reference/*, stdlib/*)
Agent 5: Examples + Research + Blog               (examples/, research/, blog/)
Agent 6: OG tags + deploy to Vercel               (japl-lang.dev domain)
```

### Execution

```
Wave 1: Agent 1 (foundation) ────────── must come first (nav, styles, layout)
Wave 2: Agent 2 + 3 + 4 (parallel) ─── content pages
Wave 3: Agent 5 + 6 (parallel) ──────── examples + deploy
```
