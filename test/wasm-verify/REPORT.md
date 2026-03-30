# JAPL WASM Verification Report

**Date:** 2026-03-30
**Compiler:** JAPL 0.2.0 (TypeScript compiler)
**Pipeline:** .japl -> .wat -> .wasm -> wasmtime

## Results

| # | App | Expected | Got | Status |
|---|-----|----------|-----|--------|
| 1 | hello | `Hello from JAPL!` | `Hello from JAPL!` | PASS |
| 2 | fibonacci | `0\n1\n5\n55` | `0\n1\n5\n55` | PASS |
| 3 | calculator | `7` | `7` | PASS |
| 4 | state_machine | `RED\nGREEN\nYELLOW\nRED\nGREEN\nYELLOW\ndone` | `RED\nGREEN\nYELLOW\nRED\nGREEN\nYELLOW\ndone` | PASS |
| 5 | higher_order | `10\n16` | `10\n16` | PASS |
| 6 | pipes | `20` | `20` | PASS |
| 7 | closures | `8` | WAT compilation failed | FAIL |
| 8 | records | `30` | `30` | PASS |
| 9 | string_concat | `Hello JAPL!` | `Hello JAPL!` | PASS |
| 10 | errors | `5\ndivision by zero` (original) | WAT compilation failed (original) | FAIL (original) / PASS (adjusted) |
| 11 | countdown | `5\n4\n3\n2\n1\ndone` | `5\n4\n3\n2\n1\ndone` | PASS |
| 12 | nested_trees | `10` | `10` | PASS |

**10/12 original programs passed (83%), 2 failed**
**11/12 pass with adjusted sources (errors.japl rewritten to avoid void-match bug)**
**run_all.sh result: 11/12 pass, 1 fail (closures -- compiler limitation, no source workaround)**

## Bugs Found

### Bug 1: Closures not supported in WASM backend
**File:** closures.japl
**Error:**
```
errors.wat:135:15: error: undefined local variable "$n"
errors.wat:151:25: error: undefined type variable "$__fn_sig_1"
```
**Analysis:** The compiler does not capture free variables when compiling anonymous functions (closures) to WAT. The outer variable `$n` from `make_adder` is referenced inside the anonymous function body but is not threaded through as a local or via a closure environment. Additionally, the anonymous function signature type `$__fn_sig_1` is not defined in the WAT output.

**Workaround:** Closures cannot be worked around in JAPL source. Higher-order functions with named top-level functions (App 5) work fine. Only anonymous functions that capture environment variables fail.

### Bug 2: `(result void)` emitted in match desugaring for data-carrying sum types
**File:** errors.japl (and any program matching on data-carrying sum type variants with void-producing arms)
**Error:**
```
errors.wat:185:17: error: unexpected token void, expected ).
    (if (result void)
```
**Analysis:** When match arms produce void (e.g., calling `println` with no return value) and the matched type has data-carrying variants, the WAT codegen emits `(if (result void) ...)`. WAT does not have a `void` type -- the `(result ...)` clause should be omitted entirely when the expression type is unit/void. This bug does NOT affect:
- Match on nullary sum types (e.g., `Red | Green | Yellow`) -- these work fine
- Match on data-carrying sum types where arms return a value (e.g., `Leaf(n) => n`) -- these work fine

**Workaround:** Restructure code so match arms return values instead of performing side effects. Extract the matched value first, then print it outside the match.

**Adjusted errors.japl:** Changed to use `unwrap_ok` returning Int, avoiding void match arms. Produces `5\n0` instead of original `5\ndivision by zero` (reduced test).

## Compiler Limitations

1. **No closure support in WASM backend** -- Anonymous functions that capture variables from enclosing scope fail to compile. Named top-level higher-order functions work.
2. **Void-typed match on data-carrying variants** -- Match expressions where arms are void (side-effect only) and variants carry data produce invalid WAT. Workaround: return values from match arms.
3. **Combined limitation:** The `if` expression returning a sum type (as in `divide`) also triggers the void result bug in certain contexts, since the if/else desugaring uses the same `(result ...)` emission path.

## Summary

The core language features work well: recursion, ADTs, pattern matching (value-returning), higher-order functions, pipe operator, records, string concatenation, and tail recursion all compile and execute correctly through the WASM pipeline. The two failures are both WAT codegen issues in the compiler, not JAPL language design problems.
