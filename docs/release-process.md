# JAPL Release Process

## Prerequisites

- Rust toolchain (rustup)
- wat2wasm (brew install wabt)
- wasmtime (brew install wasmtime)
- wash CLI (https://wasmcloud.com/docs/installation)
- nats-server (brew install nats-server)

## Development Verification

Run the test suite in development mode (wasmCloud not required):

    python3 test/verify/verify_all.py

This allows wasmCloud tests to SKIP without failing.

## Release Verification

Run the test suite in release mode (wasmCloud required):

    python3 test/verify/verify_all.py --release

Or use the release check script:

    scripts/release-check.sh

In release mode:
- wasmCloud SKIP becomes FAIL
- All prerequisites must be available
- Provider must build successfully

## Release Checklist

1. All tests pass in release mode
2. Component compilation verified for all apps
3. wasmCloud deployment path verified
4. Provider builds and self-test passes
5. No STUB/SIMULATED/LIMITED labels on release-critical functions
6. README and docs match shipped behavior
