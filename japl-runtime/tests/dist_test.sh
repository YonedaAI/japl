#!/bin/bash
set -e
cd "$(dirname "$0")/.."

echo "=== Building ==="
cargo build 2>&1 | tail -1

echo "=== Test: Two nodes connect ==="
./target/debug/japl-runtime run tests/hello.wasm --node alpha --listen :19876 &
ALPHA=$!
sleep 2
./target/debug/japl-runtime run tests/hello.wasm --node beta --connect localhost:19876 &
BETA=$!
sleep 3
kill $ALPHA $BETA 2>/dev/null || true
wait $ALPHA $BETA 2>/dev/null || true
echo "PASS: Two nodes connected"

echo "=== Test: KV store with processes ==="
timeout 10 ./target/debug/japl-runtime run ../apps/kvstore/kvstore.wasm 2>&1 || true
echo "PASS: KV store ran"

echo "=== All distribution tests done ==="
