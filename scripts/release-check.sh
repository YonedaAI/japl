#!/bin/bash
set -e

echo "=== JAPL Release Verification ==="
echo ""

# Step 1: Build
echo "Step 1: Building compiler + runtime..."
cd japl && cargo build --release
cd ..

# Step 2: Run tests in release mode
echo ""
echo "Step 2: Running verification suite (release mode)..."
python3 test/verify/verify_all.py --release

# Step 3: Build provider
echo ""
echo "Step 3: Building provider..."
if [ -d "japl-provider" ]; then
    cd japl-provider && cargo build 2>&1 | tail -3
    cd ..
    echo "Provider: OK"
else
    echo "Provider: MISSING (japl-provider/ not found)"
    exit 1
fi

echo ""
echo "=== Release Verification Complete ==="
