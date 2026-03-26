#!/bin/bash
# Bootstrap build script for the JAPL self-hosted compiler
# 1. Compiles compiler.japl to TypeScript using the TS compiler
# 2. Prepends the runtime imports
# 3. Runs the result on a test file

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
TS_COMPILER="$SCRIPT_DIR/../ts"
SRC="$SCRIPT_DIR/src/compiler.japl"
BUNDLE="$SCRIPT_DIR/dist/compiler_bundle.ts"

mkdir -p "$SCRIPT_DIR/dist"

echo "Step 1: Compiling compiler.japl with TS compiler..."
cd "$TS_COMPILER"
node dist/index.js build "$SRC" --target ts
# The TS compiler outputs alongside the source file
OUT="$SCRIPT_DIR/src/compiler.ts"

echo "Step 2: Creating bundle with runtime..."
cat > "$BUNDLE" << 'RUNTIME'
import * as fs from 'node:fs';
function cons(x: any, xs: any[]): any[] { return [x, ...xs]; }
function append(xs: any[], ys: any[]): any[] { return [...xs, ...ys]; }
function char_at(s: string, i: number): string { return s[i] ?? ''; }
function string_length(s: string): number { return s.length; }
function substring(s: string, start: number, end: number): string { return s.slice(start, end); }
function read_file(filepath: string): string { return fs.readFileSync(filepath, 'utf-8'); }
function get_arg(n: number): string { return process.argv[n + 1] ?? ''; }
function println(s: string): void { console.log(s); }
function show(x: any): string { return String(x); }
RUNTIME

# Append the compiled code (skip any import lines since runtime is inlined)
cat "$OUT" >> "$BUNDLE"

# Add main() call at the end
echo "" >> "$BUNDLE"
echo "main();" >> "$BUNDLE"

echo "Step 3: Bundle created at $BUNDLE"
echo "Run with: npx tsx $BUNDLE <input.japl>"
