#!/usr/bin/env python3
"""JAPL Performance Benchmark Suite"""
import subprocess, time, os

JAPL = os.path.join(os.path.dirname(__file__), "..", "..", "japl/target/release/japl")

def bench(name, japl_file, iterations=3):
    """Run a benchmark, report avg time"""
    times = []
    for _ in range(iterations):
        start = time.time()
        r = subprocess.run([JAPL, "run", japl_file], capture_output=True, timeout=30)
        elapsed = time.time() - start
        if r.returncode == 0:
            times.append(elapsed)
    if times:
        avg = sum(times) / len(times)
        print(f"  {name}: {avg*1000:.1f}ms avg ({len(times)} runs)")
    else:
        print(f"  {name}: FAILED")

print("=== JAPL Benchmark Suite ===\n")
print("--- Process Spawning ---")
bench("kvstore", "apps/kvstore/kvstore.japl")
bench("scheduler", "apps/scheduler/scheduler.japl")
bench("agents", "apps/agents/agents.japl")
print("\n--- Stdlib ---")
bench("genome_pipeline", "apps/genome/pipeline.japl")
print("\n--- Compile Only ---")
# Measure just compile time
for name in ["Math", "String", "Option", "List", "Map", "Set"]:
    start = time.time()
    r = subprocess.run([JAPL, "build", f"stdlib/{name}.japl", "--out", "/tmp"], capture_output=True)
    elapsed = time.time() - start
    print(f"  compile {name}: {elapsed*1000:.1f}ms")

print("\n=== Done ===")
