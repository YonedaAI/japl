# wasmCloud Integration Status

## Working
- `japl build` compiles `.japl` to WASM Component (core wasm + wasm-tools component new)
- Component exports `wasi:cli/run@0.2.3` (standard CLI command component)
- Component imports WASI IO, CLI, filesystem, clocks interfaces
- `wash inspect` recognizes the component and shows its WIT world
- `wasm-tools component wit` validates the component structure
- Tools installed: wash 2.0.1, wasm-tools 1.245.1, wat2wasm 1.0.39

## wash 2.0.1 Reality
- `wash inspect` works
- `wash dev` exists (starts a dev server for components)
- `wash host` exists (runs a wasmCloud host)
- `wash app` does NOT exist in wash 2.0.1 (no wadm deployment CLI)
- wadm manifests cannot be deployed via wash CLI in this version
- `wash dev` may auto-discover and run a component, but requires wasmCloud project structure (wasmcloud.toml)

## Not Working Yet
- `wasi:http/incoming-handler` export: compiler emits core WASM with `_start`, not Component Model typed exports
- No wasi:keyvalue provider bindings
- No wasmcloud:messaging provider bindings
- No actual HTTP serving via wasmCloud (requires incoming-handler export)
- wadm deployment (wash 2.0.1 lacks `wash app deploy`)

## What's Needed
1. **Compiler**: emit Component Model exports beyond `_start` (e.g., `wasi:http/incoming-handler.handle`)
   - This requires the WAT emitter to produce component-level export declarations
   - Or: generate a WIT file alongside the component and use wasm-tools to compose
2. **Or**: use a shim/wrapper that bridges `wasi:cli/run` to `wasi:http/incoming-handler`
3. **wadm**: either upgrade wash to a version with `wash app`, or use NATS + wadm binary directly
4. **wasmCloud project structure**: create `wasmcloud.toml` for `wash dev` compatibility

## Build Pipeline
```
.japl --> japl-compiler --> .wat --> wat2wasm --> core.wasm --> wasm-tools component new --> component.wasm
                                                                  (with WASI preview1 adapter)
```

## Testing Commands
```bash
# Build a component
japl build apps/http-hello/hello.japl

# Verify it's a valid component
wasm-tools component wit build/hello.wasm

# Inspect with wash
wash inspect build/hello.wasm
```
