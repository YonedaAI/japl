# wasmCloud End-to-End Blockers

Date: 2026-03-31

## Status: Architecture correct, deployment blocked by wash 2.0.1

---

## Blocker 1: wash 2.0.1 HTTP Routing Bug

**What happens**: `wash dev` loads the JAPL Component, starts HTTP listener on port 8000, but returns HTTP 404 for ALL requests. The component never receives the request.

**Evidence**:
```
wash dev --non-interactive
# Logs show:
#   HTTP server listening addr=0.0.0.0:8000
#   listening for HTTP requests address=http://0.0.0.0:8000
#   component loaded

curl -v http://localhost:8000/health
# < HTTP/1.1 404 Not Found
# < content-length: 0
```

**Root cause**: wash 2.0.1 trace shows `"No workload bound to host header or wildcard '*'"`. The component is loaded but never registered as a workload handler. The HTTP server's routing logic uses Host header matching, but no workload gets bound.

**Workaround**: `wasmtime serve -S cli component.wasm` works correctly — same Component, same HTTP requests, correct responses.

**Fix needed**: Upgrade wash to a version where `wash dev` correctly routes HTTP to loaded components, OR file a bug with wasmCloud team.

---

## Blocker 2: Process Apps on wasmCloud

**What works**:
- Compiler emits `--target component` with `cm32p2|japl:runtime/processes@0.1` canonical ABI imports
- `wasm-tools component embed` + `new` produces valid Component importing `japl:runtime/processes@0.1.0`
- japl-provider implements spawn/send/receive over NATS (self-test passes)

**What doesn't work**:
- No runtime currently satisfies `japl:runtime/processes` imports when running on wasmtime serve or wasmCloud
- wasmtime serve only provides WASI interfaces, not custom WIT interfaces
- wasmCloud could provide it via the japl-provider, but Blocker 1 prevents testing

**Fix needed**:
1. Resolve Blocker 1 (wash HTTP routing)
2. Register japl-provider as a wasmCloud capability provider
3. Link the provider to the JAPL component via wadm manifest or wash dev config
4. Test: HTTP request → Component → japl:runtime/processes → provider → NATS → response

---

## Blocker 3: Closure + HTTP Handler + Process in Same App

**What happens**: When a JAPL app uses both closures (for `spawn(fn() { ... })`) AND `handle_request`, the WAT output has a `call_indirect (type $closure_0)` reference that fails validation because the function table type isn't declared.

**Evidence**:
```
wat2wasm failed:
/tmp/process_http.wat:452:29: error: undefined type variable "$closure_0"
        call_indirect (type $closure_0)
```

**Root cause**: The canonical ABI handler exports change the function table layout. The `$closure_0` type declaration is emitted but the function table reference is broken when both HTTP handler exports and closure tables coexist.

**Fix needed**: In `emit_wat.rs`, ensure the function table type declarations are emitted correctly when both `has_http_handler` and closure functions are present. This is a compiler bug, not an architecture issue.

---

## What IS Working (verified)

| Path | Status |
|------|--------|
| `japl build app.japl` → core WASM | 43 tests pass |
| `japl build --target component` → canonical ABI imports | Verified |
| `japl run app.japl` → local processes (OS threads) | Verified, 256% CPU |
| `japl serve app.japl` → HTTP | Verified, curl tests pass |
| Component → `wasmtime serve` → HTTP | Verified: `curl /health → "ok"` |
| Component imports `japl:runtime/processes@0.1.0` | Verified via `wasm-tools component wit` |
| japl-provider spawn/send/receive over NATS | Self-test passes |
| HTTP adapter exports `wasi:http/incoming-handler@0.2.3` | Verified |
| `wac plug` composes JAPL + adapter into single Component | Verified |

## Priority Order for Fixes

1. **Blocker 3** (compiler bug) — Fixable in emit_wat.rs, unblocks process+HTTP apps
2. **Blocker 1** (wash 2.0.1) — Check for wash update, or build a minimal test harness that loads Component + provides japl:runtime imports without wash
3. **Blocker 2** (provider integration) — Depends on Blocker 1 resolution

## Architectural Decision Record

The architecture is: **language defines semantics, backends implement them.**

- `spawn`, `send`, `receive` are language primitives with stable semantics
- `--target local`: embedded wasmtime provides them via OS threads
- `--target component`: WIT interface `japl:runtime/processes@0.1.0`, implemented by japl-provider over NATS
- The language code is IDENTICAL regardless of target — only the compiler backend differs
- This is the Erlang model: BEAM is one implementation, the language is target-independent
