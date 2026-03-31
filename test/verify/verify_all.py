import subprocess, os, sys, time

JAPL_HOME = os.path.join(os.path.dirname(__file__), "..", "..")
COMPILER = os.path.join(JAPL_HOME, "japl-compiler/target/release/japl-compiler")
RUNTIME = os.path.join(JAPL_HOME, "japl-runtime/target/release/japl-runtime")
PASS = 0
FAIL = 0

def run_test(name, japl_file, expected_contains, use_runtime=False, retries=1):
    global PASS, FAIL
    japl_path = os.path.join(JAPL_HOME, japl_file)

    # Compile
    r = subprocess.run([COMPILER, "build", japl_path, "--out", "/tmp"],
                       capture_output=True, text=True)
    if r.returncode != 0:
        print(f"  FAIL {name}: compile error: {r.stderr.strip()}")
        FAIL += 1
        return

    wasm = f"/tmp/{os.path.basename(japl_file).replace('.japl', '.wasm')}"

    # Run (with retries for non-deterministic process tests)
    cmd = [RUNTIME, "run", wasm] if use_runtime else ["wasmtime", wasm]
    for attempt in range(retries):
        try:
            r = subprocess.run(cmd, capture_output=True, text=True, timeout=10)
            output = r.stdout
            missing = [e for e in expected_contains if e not in output]
            if not missing:
                print(f"  PASS {name}")
                PASS += 1
                return
            if attempt < retries - 1:
                continue
            print(f"  FAIL {name}: missing {missing!r} in output: {output!r}")
            FAIL += 1
        except subprocess.TimeoutExpired:
            if attempt < retries - 1:
                continue
            print(f"  FAIL {name}: timeout")
            FAIL += 1

def compile_only_test(name, japl_file):
    global PASS, FAIL
    japl_path = os.path.join(JAPL_HOME, japl_file)
    r = subprocess.run([COMPILER, "build", japl_path, "--out", "/tmp"],
                       capture_output=True, text=True)
    if r.returncode == 0:
        print(f"  PASS {name}")
        PASS += 1
    else:
        print(f"  FAIL {name}: compile error: {r.stderr.strip()}")
        FAIL += 1

print("=== JAPL Verification Suite ===\n")

print("--- Core Language ---")
run_test("hello", "japl-compiler/tests/hello.japl", ["Hello!"])
run_test("fibonacci", "japl-compiler/tests/fib.japl", ["55"])
run_test("calculator", "japl-compiler/tests/calculator.japl", ["42"])
run_test("state_machine", "japl-compiler/tests/state_machine.japl", ["red", "blue"])
run_test("records", "japl-compiler/tests/records.japl", ["Alice", "30", "31"])
run_test("closures", "japl-compiler/tests/closures.japl", ["8", "13"])
run_test("higher_order", "japl-compiler/tests/higher_order.japl", ["10"])
run_test("pipes", "japl-compiler/tests/pipes.japl", ["20"])
run_test("countdown", "japl-compiler/tests/countdown.japl", ["done"])
run_test("greet", "japl-compiler/tests/greet.japl", ["Hello JAPL!"])
run_test("bool", "japl-compiler/tests/bool.japl", ["yes", "false"])
run_test("constants", "japl-compiler/tests/constants.japl", ["100"])
run_test("guards", "japl-compiler/tests/guards.japl", [])

print("\n--- Processes ---")
run_test("processes", "japl-compiler/tests/process.japl", ["got"], use_runtime=True, retries=3)
run_test("kvstore", "japl-compiler/tests/kvstore.japl", ["stored", "value"], use_runtime=True, retries=3)

print("\n--- Stdlib ---")
compile_only_test("stdlib/Math", "stdlib/Math.japl")
compile_only_test("stdlib/Option", "stdlib/Option.japl")
compile_only_test("stdlib/Result", "stdlib/Result.japl")
compile_only_test("stdlib/String", "stdlib/String.japl")
compile_only_test("stdlib/IO", "stdlib/IO.japl")
compile_only_test("stdlib/List", "stdlib/List.japl")
compile_only_test("stdlib/Json", "stdlib/Json.japl")
compile_only_test("stdlib/Http", "stdlib/Http.japl")
compile_only_test("stdlib/Process", "stdlib/Process.japl")
compile_only_test("stdlib/Net", "stdlib/Net.japl")

print("\n--- Apps ---")
run_test("scheduler", "apps/scheduler/scheduler.japl", ["Worker", "done"], use_runtime=True, retries=3)
run_test("msgqueue", "apps/msgqueue/queue.japl", ["Got"], use_runtime=True, retries=3)

print("\n--- Type Checker ---")
r = subprocess.run([COMPILER, "check",
                     os.path.join(JAPL_HOME, "japl-compiler/tests/type_error.japl")],
                    capture_output=True, text=True)
if "error" in r.stdout.lower() or "error" in r.stderr.lower():
    print("  PASS type_checker")
    PASS += 1
else:
    print("  FAIL type_checker")
    FAIL += 1

print(f"\n=== Results: {PASS} pass, {FAIL} fail ===")
sys.exit(0 if FAIL == 0 else 1)
