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
run_test("generics", "japl-compiler/tests/generics_fn.japl", ["42", "hello"])
run_test("exhaustive_match", "japl-compiler/tests/exhaustive.japl", ["red", "blue"])

print("\n--- Processes ---")
run_test("processes", "japl-compiler/tests/process.japl", ["got"], use_runtime=True, retries=3)
run_test("kvstore", "japl-compiler/tests/kvstore.japl", ["stored", "value"], use_runtime=True, retries=3)

print("\n--- Stdlib ---")
run_test("stdlib/Math", "stdlib/Math.japl", ["abs(-5)=5", "max(3,7)=7", "min(3,7)=3", "clamp(10,0,5)=5", "pow(2,8)=256", "gcd(12,8)=4"])
run_test("stdlib/Option", "stdlib/Option.japl", ["is_some(Some(42))=1", "is_some(None)=0", "unwrap_or(Some(42),0)=42", "unwrap_or(None,0)=0"])
run_test("stdlib/Result", "stdlib/Result.japl", ["is_ok(Ok(42))=1", "is_ok(Err)=0", "unwrap_or(Ok(42),0)=42", "unwrap_or(Err,0)=0"])
run_test("stdlib/String", "stdlib/String.japl", ["concat: hello world", "repeat: ababab", "join: foo, bar"])
run_test("stdlib/IO", "stdlib/IO.japl", ["IO module loaded", "42", "[debug] test=99"])
run_test("stdlib/List", "stdlib/List.japl", ["length: 3", "sum: 6", "Contains 2: 1", "Contains 5: 0"])
run_test("stdlib/Json", "stdlib/Json.japl", ["Int:  42", "Bool: true", "Null: null"])
run_test("stdlib/Http", "stdlib/Http.japl", ["Method: GET", "Method: POST", "Status: 200 OK", "Status: 404 Not Found"])
run_test("stdlib/Process", "stdlib/Process.japl", ["Process module loaded", "done"], use_runtime=True, retries=3)
compile_only_test("stdlib/Net", "stdlib/Net.japl")
run_test("stdlib/Map", "stdlib/Map.japl", ["Map size: 3", "Get 1: 100", "Get 2: 200", "Contains 2: 1", "Contains 5: 0"])
run_test("stdlib/Test", "stdlib/Test.japl", ["PASS", "FAIL"])
run_test("stdlib/Time", "stdlib/Time.japl", ["Time module loaded"], use_runtime=True)
run_test("stdlib/Env", "stdlib/Env.japl", ["Env module loaded"], use_runtime=True)
run_test("stdlib/Crypto", "stdlib/Crypto.japl", ["Crypto module loaded"], use_runtime=True)
compile_only_test("stdlib/File", "stdlib/File.japl")

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
