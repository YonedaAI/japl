#!/usr/bin/env bash
# JAPL WASM Verification - Run All 12 Tests
# Usage: bash run_all.sh

set -euo pipefail

COMPILER_DIR="$(cd "$(dirname "$0")/../../compiler/ts" && pwd)"
TEST_DIR="$(cd "$(dirname "$0")" && pwd)"

PASS=0
FAIL=0
TOTAL=0

run_test() {
  local num="$1"
  local name="$2"
  local file="$3"
  local expected="$4"

  TOTAL=$((TOTAL + 1))

  if [ ! -f "$TEST_DIR/$file" ]; then
    echo "[$num] $name: SKIP (file not found)"
    FAIL=$((FAIL + 1))
    return
  fi

  local output
  if output=$(cd "$COMPILER_DIR" && node dist/index.js run "$TEST_DIR/$file" 2>&1); then
    if [ "$output" = "$expected" ]; then
      echo "[$num] $name: PASS"
      PASS=$((PASS + 1))
    else
      echo "[$num] $name: FAIL (wrong output)"
      echo "  Expected: $(echo "$expected" | head -3)"
      echo "  Got:      $(echo "$output" | head -3)"
      FAIL=$((FAIL + 1))
    fi
  else
    echo "[$num] $name: FAIL (compilation/runtime error)"
    echo "  Error: $(echo "$output" | head -3)"
    FAIL=$((FAIL + 1))
  fi
}

echo "=== JAPL WASM Verification Suite ==="
echo ""

run_test 1  "hello"         "hello.japl"         "Hello from JAPL!"
run_test 2  "fibonacci"     "fibonacci.japl"     "$(printf '0\n1\n5\n55')"
run_test 3  "calculator"    "calculator.japl"    "7"
run_test 4  "state_machine" "state_machine.japl" "$(printf 'RED\nGREEN\nYELLOW\nRED\nGREEN\nYELLOW\ndone')"
run_test 5  "higher_order"  "higher_order.japl"  "$(printf '10\n16')"
run_test 6  "pipes"         "pipes.japl"         "20"
run_test 7  "closures"      "closures.japl"      "8"
run_test 8  "records"       "records.japl"       "30"
run_test 9  "string_concat" "string_concat.japl" "Hello JAPL!"
run_test 10 "errors"        "errors.japl"        "$(printf '5\n0')"
run_test 11 "countdown"     "countdown.japl"     "$(printf '5\n4\n3\n2\n1\ndone')"
run_test 12 "nested_trees"  "nested_trees.japl"  "10"

echo ""
echo "=== Results: $PASS/$TOTAL passed, $FAIL failed ==="

if [ "$FAIL" -gt 0 ]; then
  exit 1
fi
