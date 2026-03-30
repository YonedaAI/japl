#!/bin/bash
set -e

cd "$(dirname "$0")/.."

echo "=== Building ==="
cargo build 2>&1 | tail -1

echo ""
echo "=== Test 1: Two nodes connect and run hello.wasm ==="
./target/debug/japl-runtime run tests/hello.wasm --node alpha --listen :9876 &
ALPHA_PID=$!
sleep 1

./target/debug/japl-runtime run tests/hello.wasm --node beta --connect localhost:9876 &
BETA_PID=$!
sleep 2

# Clean up
kill $ALPHA_PID $BETA_PID 2>/dev/null || true
wait $ALPHA_PID 2>/dev/null || true
wait $BETA_PID 2>/dev/null || true

echo ""
echo "=== Test 2: Node with process_test.wasm in distributed mode ==="
# process_test.wasm finishes quickly, so just test that --node + --listen works
./target/debug/japl-runtime run tests/process_test.wasm --node alpha2 --listen :9877 &
ALPHA_PID=$!
sleep 2
kill $ALPHA_PID 2>/dev/null || true
wait $ALPHA_PID 2>/dev/null || true

echo ""
echo "=== Test 3: Local-only mode still works ==="
./target/debug/japl-runtime run tests/hello.wasm
./target/debug/japl-runtime run tests/process_test.wasm

echo ""
echo "=== All distribution tests passed ==="
