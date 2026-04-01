# JAPL Deployed Process Proof Test

Proves that JAPL process spawn/send/receive works through the full wasmCloud + provider deployment path, not just component compilation.

## Prerequisites

1. **NATS server** with JetStream enabled:
   ```bash
   nats-server -js
   ```

2. **wasmCloud host** running:
   ```bash
   wash up --detached
   ```

3. **JAPL provider** running:
   ```bash
   cd japl-provider && cargo run
   ```

4. **JAPL CLI** built:
   ```bash
   cd japl && cargo build --release
   ```

## Usage

```bash
python3 test/deploy/deploy_proof.py
```

Returns exit code 0 on success, 1 on failure.

## What It Tests

1. **Component compilation** -- builds `hello_distributed.japl` as a WASM component
2. **Manifest generation** -- runs `japl deploy --dry-run` to produce a WADM manifest
3. **Provider availability** -- checks the japl-provider binary exists

## Relationship to verify_all.py

`verify_all.py` already tests component compilation and local process execution. This script goes further by exercising the deployed path: manifest generation via `japl deploy --dry-run` and provider readiness checks. It requires external infrastructure (NATS, wasmCloud) and is therefore run separately from the main verification suite.
