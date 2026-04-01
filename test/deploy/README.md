# JAPL Deploy Functional Test

Exercises the FULL deployed process path over NATS:

1. **Component compilation** — builds JAPL app as WASM component
2. **WADM manifest generation** — produces deployment manifest via `--dry-run`
3. **Provider health** — verifies japl-provider is running and responsive
4. **Process spawn** — spawns a new process through provider via NATS
5. **Message send** — sends a message to the spawned process mailbox
6. **Message receive** — receives the message back from the mailbox

## Prerequisites

```bash
# 1. Start NATS with JetStream
nats-server -js

# 2. Build and start the JAPL provider
cd japl-provider && cargo run --release

# 3. Build the JAPL compiler
cd japl && cargo build --release
```

## Usage

```bash
python3 test/deploy/deploy_proof.py
```

## Integration with verify_all.py

The functional deploy test is called automatically by `verify_all.py`.
- In dev mode: SKIP if NATS/provider not running
- In release mode (`--release`): FAIL if NATS/provider not running
