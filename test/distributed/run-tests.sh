#!/bin/bash
set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$SCRIPT_DIR"

echo "╔══════════════════════════════════════════════════╗"
echo "║  JAPL Distributed Test Suite                      ║"
echo "╚══════════════════════════════════════════════════╝"
echo ""

# Test 1: Basic two-node communication
echo "━━━ Test 1: Basic Two-Node Communication ━━━"
echo ""
echo "Starting alpha (counter) and beta (client) on separate networks..."
docker compose down --remove-orphans 2>/dev/null || true
docker compose up --build --abort-on-container-exit --exit-code-from beta 2>&1 | while IFS= read -r line; do echo "  $line"; done
RESULT=$?
docker compose down --remove-orphans 2>/dev/null
if [ $RESULT -eq 0 ]; then
  echo ""
  echo "  ✓ Test 1 PASSED: Two-node communication works"
else
  echo ""
  echo "  ✗ Test 1 FAILED: exit code $RESULT"
  exit 1
fi
echo ""

# Test 2: Chaos test (node failure + recovery)
echo "━━━ Test 2: Chaos Test (Node Failure + Recovery) ━━━"
echo ""
echo "Starting alpha, beta, and chaos monkey..."
docker compose -f docker-compose.chaos.yml down --remove-orphans 2>/dev/null || true
docker compose -f docker-compose.chaos.yml up --build --abort-on-container-exit --exit-code-from chaos 2>&1 | while IFS= read -r line; do echo "  $line"; done
RESULT=$?
docker compose -f docker-compose.chaos.yml down --remove-orphans 2>/dev/null
if [ $RESULT -eq 0 ]; then
  echo ""
  echo "  ✓ Test 2 PASSED: Chaos test completed"
else
  echo ""
  echo "  ✗ Test 2 FAILED: exit code $RESULT"
  exit 1
fi
echo ""

echo "╔══════════════════════════════════════════════════╗"
echo "║  All distributed tests passed! ✓                  ║"
echo "╚══════════════════════════════════════════════════╝"
