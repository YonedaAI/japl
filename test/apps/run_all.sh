#!/bin/bash
cd /Users/mlong/Documents/Development/japl/compiler/ts

APPS_DIR="/Users/mlong/Documents/Development/japl/test/apps"
PASS=0
FAIL=0

echo "=== JAPL App Verification Suite ==="
for app in "$APPS_DIR"/*.japl; do
  name=$(basename "$app" .japl)
  ts_file="${app%.japl}.ts"
  echo ""
  echo "--- $name.japl ---"

  # Build to TS (single-file path, avoids __dirname ESM bug in run command)
  if node dist/index.js build "$app" --target ts 2>&1; then
    echo "  [build] OK"
  else
    echo "  [build] FAILED"
    FAIL=$((FAIL + 1))
    continue
  fi

  # Run the generated TS
  if npx tsx "$ts_file" 2>&1; then
    PASS=$((PASS + 1))
  else
    echo "  [run] FAILED"
    FAIL=$((FAIL + 1))
  fi

  # Clean up generated .ts
  rm -f "$ts_file"
done

echo ""
echo "=== Results: $PASS passed, $FAIL failed ==="
