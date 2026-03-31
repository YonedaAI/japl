#!/usr/bin/env bash
# JAPL Checker Negative Tests
# Every file in this directory should FAIL type checking.
# This script verifies that each one produces the expected error.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
JAPL_BIN="${SCRIPT_DIR}/../../japl/target/debug/japl"

if [ ! -f "$JAPL_BIN" ]; then
  echo "ERROR: japl binary not found at $JAPL_BIN"
  echo "Run 'cargo build' in japl/ first."
  exit 1
fi

PASS=0
FAIL=0
TOTAL=0

for f in "$SCRIPT_DIR"/*.japl; do
  TOTAL=$((TOTAL + 1))
  name=$(basename "$f")
  output=$("$JAPL_BIN" check "$f" 2>&1) && {
    echo "FAIL $name: expected error but got success"
    FAIL=$((FAIL + 1))
    continue
  }
  # Check that the output contains "type error" or "effect error"
  if echo "$output" | grep -qE "(type error|effect error)"; then
    echo "PASS $name"
    PASS=$((PASS + 1))
  else
    echo "FAIL $name: failed but without type/effect error: $output"
    FAIL=$((FAIL + 1))
  fi
done

echo ""
echo "=== Negative Test Results: $PASS/$TOTAL passed, $FAIL failed ==="

if [ "$FAIL" -gt 0 ]; then
  exit 1
fi
