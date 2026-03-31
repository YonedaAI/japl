#!/usr/bin/env python3
"""Generate API documentation from JAPL stdlib modules"""
import os, re, glob

STDLIB = os.path.join(os.path.dirname(__file__), "..", "stdlib")

print("# JAPL Standard Library API Reference\n")

for path in sorted(glob.glob(os.path.join(STDLIB, "*.japl"))):
    name = os.path.basename(path).replace(".japl", "")
    print(f"## {name}\n")

    with open(path) as f:
        lines = f.readlines()

    # Extract types
    for line in lines:
        line = line.strip()
        if line.startswith("type "):
            print(f"```\n{line}\n```\n")

    # Extract pub functions
    for line in lines:
        line = line.strip()
        if line.startswith("pub fn "):
            # Extract just the signature (up to the opening brace)
            sig = line.split("{")[0].strip()
            print(f"- `{sig}`")

    print()

print("---\n*Generated from stdlib source*")
