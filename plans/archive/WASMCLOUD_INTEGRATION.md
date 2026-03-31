# wasmCloud Integration Plan for JAPL

## What Works Today

The full pipeline from JAPL source to WASM Component is operational:

1. **JAPL -> WAT**: Self-hosted compiler (`compiler/self/compiler.wasm`) compiles `.japl` files to WebAssembly Text format
2. **WAT -> core WASM**: `wat2wasm` converts WAT to a core WASM module (613 bytes for hello world)
3. **core WASM -> WASM Component**: `wasm-tools component new` wraps the core module using the WASI preview1-to-preview2 adapter, producing a valid WASM Component (~20KB)
4. **Component execution**: The resulting component exports `wasi:cli/run@0.2.3` and runs correctly under `wasmtime`

### Verified Pipeline

```
japl source -> compiler.wasm -> .wat -> wat2wasm -> core.wasm -> wasm-tools -> component.wasm
```

### Installed Tooling

- `wash` v2.0.1 (wasmCloud CLI)
- `wasm-tools` v1.245.1 (component tooling)
- `wat2wasm` (wabt)
- WASI adapters stored in `deploy/adapters/`

### wash 2.x Notes

wash 2.0 has a different command structure than 1.x:
- `wash host` replaces `wash up` (runs the wasmCloud host directly)
- `wash dev` starts a development server for iterating on components
- `wash build` builds components from a project
- No separate `wash app deploy` -- deployment model has changed
- See https://wasmcloud.com/docs for current docs

## What's Missing

### 1. HTTP Handler Interface

JAPL currently compiles to CLI-style WASM that exports `_start` (wrapped as `wasi:cli/run`). For HTTP-accessible services on wasmCloud, components must export `wasi:http/incoming-handler@0.2.0`:

```wit
export wasi:http/incoming-handler@0.2.0 {
  handle: func(request: incoming-request, response-outparam: response-outparam)
}
```

The self-hosted compiler has no concept of HTTP request/response types or the component model's resource types.

### 2. Component Model Type System

The WASM Component Model uses a richer type system than core WASM:
- Records, variants, enums, flags
- Resources (linear handles)
- `result<T, E>`, `option<T>`
- `list<u8>` for byte buffers

JAPL's type system would need to map to these types for proper WIT interface compliance.

### 3. WIT Bindings Generation

Components that interact with wasmCloud providers (HTTP server, KV store, messaging) need WIT bindings. Currently there is no JAPL -> WIT binding generator.

### 4. Provider Bindings

wasmCloud providers that JAPL components would need:
- `wasmcloud:http` -- HTTP server/client capability
- `wasmcloud:keyvalue` -- Key-value storage
- `wasmcloud:messaging` -- NATS messaging
- `wasmcloud:secrets` -- Secret management

## Path Forward

### Phase 1: CLI Components (DONE)

JAPL programs that use `println`, basic I/O, and computation can already be deployed as WASM Components. The `japl deploy` command handles this pipeline.

### Phase 2: HTTP Handler Template

Generate a minimal WAT template that:
1. Imports `wasi:http/incoming-handler` types
2. Wraps JAPL-compiled logic in the handler function
3. Returns hardcoded or computed responses

This could be done by having the compiler emit a different WAT preamble when targeting HTTP:

```
japl build --target http myapp.japl
```

### Phase 3: WIT Bindings in JAPL

Add JAPL language support for:
- `@http_handler` annotation on functions
- Automatic WIT binding generation
- Request/response type mapping

Example future syntax:
```japl
use wasmcloud::http::{Request, Response}

@http_handler
fn handle(req: Request) -> Response {
  Response::ok("Hello from JAPL on wasmCloud!")
}
```

### Phase 4: Full wasmCloud Integration

- KV provider bindings
- Messaging provider bindings
- Multi-component applications via wadm manifests
- OCI registry publishing via `wash oci push`

## Quick Start

```bash
# Install tooling
brew install wasmcloud/wasmcloud/wash
cargo install wasm-tools

# Build and deploy a JAPL component
bin/japl deploy examples/hello.japl

# Or manually:
bin/japl build hello.japl
wasm-tools component new build/hello.wasm \
  --adapt "wasi_snapshot_preview1=deploy/adapters/wasi_snapshot_preview1.command.wasm" \
  -o build/hello_component.wasm

# Run locally
wasmtime build/hello_component.wasm

# Start wasmCloud host (wash 2.x)
wash host
```
