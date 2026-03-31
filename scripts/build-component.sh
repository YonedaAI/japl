#!/bin/bash
# Build a JAPL HTTP app as a wasmCloud-ready Component
#
# Usage: ./scripts/build-component.sh <file.japl> [--out <dir>]
#
# Pipeline:
#   1. Compile JAPL -> core WASM (via japl build)
#   2. Wrap core WASM as WASI preview2 Component (wasm-tools component new --adapt)
#   3. Compose with HTTP adapter (wasm-tools compose) to get final Component
#      that exports wasi:http/incoming-handler
#
# Requirements:
#   - japl compiler built (japl/target/release/japl)
#   - wasm-tools (cargo install wasm-tools)
#   - WASI adapter at deploy/adapters/wasi_snapshot_preview1.reactor.wasm
#   - HTTP adapter at japl-http-adapter/target/wasm32-wasip2/release/japl_http_adapter.wasm

set -euo pipefail

JAPL_HOME="$(cd "$(dirname "$0")/.." && pwd)"
JAPL="$JAPL_HOME/japl/target/release/japl"
ADAPTER="$JAPL_HOME/deploy/adapters/wasi_snapshot_preview1.reactor.wasm"
HTTP_ADAPTER="$JAPL_HOME/japl-http-adapter/target/wasm32-wasip2/release/japl_http_adapter.wasm"

# Parse args
JAPL_FILE="${1:?Usage: build-component.sh <file.japl> [--out <dir>]}"
shift
OUT_DIR="build"
while [[ $# -gt 0 ]]; do
    case "$1" in
        --out) OUT_DIR="$2"; shift 2;;
        *) echo "Unknown arg: $1"; exit 1;;
    esac
done

BASENAME=$(basename "$JAPL_FILE" .japl)

# Verify tools exist
for tool in wasm-tools; do
    if ! command -v "$tool" &>/dev/null; then
        echo "ERROR: $tool not found. Install with: cargo install $tool"
        exit 1
    fi
done

if [[ ! -f "$JAPL" ]]; then
    echo "ERROR: japl compiler not found at $JAPL"
    echo "Build with: cargo build --release --manifest-path $JAPL_HOME/japl/Cargo.toml"
    exit 1
fi

if [[ ! -f "$ADAPTER" ]]; then
    echo "ERROR: WASI adapter not found at $ADAPTER"
    echo "Download from: https://github.com/bytecodealliance/wasmtime/releases"
    exit 1
fi

mkdir -p "$OUT_DIR"
TMPDIR=$(mktemp -d)
trap "rm -rf $TMPDIR" EXIT

echo "=== JAPL Component Build ==="
echo ""

# Step 1: Compile JAPL to core WASM
echo "[1/3] Compiling $JAPL_FILE to core WASM..."
WASM_PATH=$("$JAPL" build "$JAPL_FILE" --out "$TMPDIR")
echo "      -> $WASM_PATH"

# Step 2: Wrap as WASI preview2 Component
echo "[2/3] Wrapping as WASI preview2 Component..."
COMPONENT_PATH="$TMPDIR/${BASENAME}_component.wasm"
wasm-tools component new "$WASM_PATH" \
    --adapt "wasi_snapshot_preview1=$ADAPTER" \
    -o "$COMPONENT_PATH" 2>&1 || {
    echo "ERROR: wasm-tools component new failed."
    echo "The core WASM may have imports beyond wasi_snapshot_preview1."
    echo "Check with: wasm-tools print $WASM_PATH | grep '(import'"
    exit 1
}
echo "      -> $COMPONENT_PATH"

# Step 3: Compose with HTTP adapter (if available)
FINAL_PATH="$OUT_DIR/${BASENAME}_component.wasm"
if [[ -f "$HTTP_ADAPTER" ]]; then
    echo "[3/3] Composing with HTTP adapter..."
    # The HTTP adapter imports japl:app/handler, the JAPL component exports it
    # wasm-tools compose merges them into one component
    if wasm-tools compose "$HTTP_ADAPTER" \
        --definitions "$COMPONENT_PATH" \
        -o "$FINAL_PATH" 2>&1; then
        echo "      -> $FINAL_PATH (wasmCloud-ready)"
    else
        echo "WARNING: Composition failed. Outputting standalone component."
        cp "$COMPONENT_PATH" "$FINAL_PATH"
        echo "      -> $FINAL_PATH (standalone, no wasi:http export)"
    fi
else
    echo "[3/3] HTTP adapter not built, outputting standalone component..."
    cp "$COMPONENT_PATH" "$FINAL_PATH"
    echo "      -> $FINAL_PATH (standalone)"
fi

echo ""
echo "=== Build Complete ==="
echo "Component: $FINAL_PATH"
SIZE=$(wc -c < "$FINAL_PATH" | tr -d ' ')
echo "Size: $SIZE bytes"
echo ""
echo "Inspect with:"
echo "  wasm-tools component wit $FINAL_PATH"
echo ""
echo "Run with japl deploy:"
echo "  japl deploy $JAPL_FILE --port 8080"
echo ""
echo "Or serve directly with wasmtime:"
echo "  wasmtime serve $FINAL_PATH --addr 0.0.0.0:8080"
