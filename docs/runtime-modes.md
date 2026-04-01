# JAPL Runtime Modes

## Overview

JAPL provides three execution modes, each targeting different use cases.

## `japl run` — Local Development
- Compiles JAPL to WASM and runs immediately
- Processes use OS threads via embedded wasmtime
- All stdlib modules work (except network-dependent stubs)
- Best for: development, testing, local execution

## `japl serve` — Local HTTP Server  
- Compiles JAPL and serves HTTP via tiny_http
- Provides host functions for HTTP request handling
- Process operations are stubbed (spawn returns -1)
- Best for: HTTP API development, local testing

## `japl deploy` — Distributed Deployment
- Compiles to WASM Component Model
- Generates WADM manifest for wasmCloud
- Primary distributed execution path (requires NATS + wasmCloud host + japl-provider)
- **Fails with an error if wasmCloud infrastructure is not available** (no silent fallback)
- Use `japl deploy --local` to explicitly opt in to local-only serving without wasmCloud
- Use `japl deploy --dry-run` to preview the generated deployment manifest
- Best for: production deployment, distributed systems

## Feature Support Matrix

| Feature | `run` | `serve` | `deploy` |
|---------|-------|---------|----------|
| Process spawn/send/receive | Yes | Stub | Via provider* |
| Supervision | Yes (local) | No | Via provider* |
| HTTP handling | No | Yes | Yes |
| File I/O | Yes | Yes | Sandboxed |
| Environment vars | Yes | Yes | Limited |
| LLM calls | Yes | Stub | Via provider* |
| Distribution (multi-node) | CLI flags | No | wasmCloud |
| Component Model output | --target component | N/A | Always |

*Via JAPL provider: requires japl-provider running as NATS sidecar

## Architecture Decision

The **primary distributed architecture** is:
JAPL source → compiler → WASM Component → wasmCloud host + JAPL provider → distributed execution

The local runtime (`run`, `serve`) is the **dev/reference path**, not the strategic distributed surface.
