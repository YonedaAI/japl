import subprocess, os, sys, time

JAPL_HOME = os.path.join(os.path.dirname(__file__), "..", "..")
JAPL = os.path.join(JAPL_HOME, "japl/target/release/japl")
PASS = 0
FAIL = 0

def run_test(name, japl_file, expected_contains, use_runtime=False, retries=1):
    global PASS, FAIL
    japl_path = os.path.join(JAPL_HOME, japl_file)

    # Compile
    r = subprocess.run([JAPL, "build", japl_path, "--out", "/tmp"],
                       capture_output=True, text=True)
    if r.returncode != 0:
        print(f"  FAIL {name}: compile error: {r.stderr.strip()}")
        FAIL += 1
        return

    wasm = f"/tmp/{os.path.basename(japl_file).replace('.japl', '.wasm')}"

    # Run — use unified binary for process tests, wasmtime for pure tests
    if use_runtime:
        cmd = [JAPL, "run", japl_path]
    else:
        cmd = ["wasmtime", wasm]

    for attempt in range(retries):
        try:
            r = subprocess.run(cmd, capture_output=True, text=True, timeout=15)
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
    r = subprocess.run([JAPL, "build", japl_path, "--out", "/tmp"],
                       capture_output=True, text=True)
    if r.returncode == 0:
        print(f"  PASS {name}")
        PASS += 1
    else:
        print(f"  FAIL {name}: compile error: {r.stderr.strip()}")
        FAIL += 1

print("=== JAPL Verification Suite ===\n")

print("--- Core Language ---")
run_test("hello", "test/programs/hello.japl", ["Hello!"])
run_test("fibonacci", "test/programs/fib.japl", ["55"])
run_test("calculator", "test/programs/calculator.japl", ["42"])
run_test("state_machine", "test/programs/state_machine.japl", ["red", "blue"])
run_test("records", "test/programs/records.japl", ["Alice", "30", "31"])
run_test("closures", "test/programs/closures.japl", ["8", "13"])
run_test("higher_order", "test/programs/higher_order.japl", ["10"])
run_test("pipes", "test/programs/pipes.japl", ["20"])
run_test("countdown", "test/programs/countdown.japl", ["done"])
run_test("greet", "test/programs/greet.japl", ["Hello JAPL!"])
run_test("bool", "test/programs/bool.japl", ["yes", "false"])
run_test("constants", "test/programs/constants.japl", ["100"])
run_test("guards", "test/programs/guards.japl", [])
run_test("generics", "test/programs/generics_fn.japl", ["42", "hello"])
run_test("exhaustive_match", "test/programs/exhaustive.japl", ["red", "blue"])
run_test("http_handler", "test/programs/http_handler.japl", ["HTTP 200 OK: GET /hello"])
run_test("dist_test", "test/programs/dist_test.japl", ["remote got: 42"], use_runtime=True, retries=3)

print("\n--- Processes ---")
run_test("processes", "test/programs/process.japl", ["got"], use_runtime=True, retries=3)
run_test("kvstore", "test/programs/kvstore.japl", ["stored", "value"], use_runtime=True, retries=3)

print("\n--- Stdlib ---")
run_test("stdlib/Math", "stdlib/Math.japl", ["abs(-5)=5", "max(3,7)=7", "min(3,7)=3", "clamp(10,0,5)=5", "pow(2,8)=256", "gcd(12,8)=4"])
run_test("stdlib/Option", "stdlib/Option.japl", ["is_some(Some(42))=1", "is_some(None)=0", "unwrap_or(Some(42),0)=42", "unwrap_or(None,0)=0"])
run_test("stdlib/Result", "stdlib/Result.japl", ["is_ok(Ok(42))=1", "is_ok(Err)=0", "unwrap_or(Ok(42),0)=42", "unwrap_or(Err,0)=0"])
run_test("stdlib/String", "stdlib/String.japl", ["concat: hello world", "repeat: ababab", "join: foo, bar", "contains hello world: 1", "starts_with hello: 1", "ends_with world: 1", "trim: [hello]", "to_upper: HELLO", "to_lower: hello", "replace: hello JAPL", "index_of: 6", "parse_int: 42", "length: 5"])
run_test("stdlib/IO", "stdlib/IO.japl", ["IO module loaded", "42", "[debug] test=99"])
run_test("stdlib/List", "stdlib/List.japl", ["length: 3", "sum: 6", "Contains 2: 1", "Contains 5: 0"])
run_test("stdlib/Json", "stdlib/Json.japl", ["Int:  42", "Bool: true", "Null: null"])
run_test("stdlib/Http", "stdlib/Http.japl", ["Method: GET", "Method: POST", "Status: 200 OK", "Status: 404 Not Found"])
run_test("stdlib/Process", "stdlib/Process.japl", ["Process module loaded", "done"], use_runtime=True, retries=3)
run_test("stdlib/Supervisor", "stdlib/Supervisor.japl", ["Supervisor module loaded", "done"], use_runtime=True, retries=3)
run_test("stdlib/Registry", "stdlib/Registry.japl", ["Registry module loaded", "Registry size: 3", "Lookup 1: 100", "After overwrite, Lookup 2: 999", "After overwrite, size: 3", "done"])
compile_only_test("stdlib/Net", "stdlib/Net.japl")
run_test("stdlib/Map", "stdlib/Map.japl", ["Map size: 3", "Get 1: 100", "Get 2: 200", "Contains 2: 1", "Contains 5: 0"])
run_test("stdlib/Set", "stdlib/Set.japl", ["Set size: 3", "Contains 1: 1", "Contains 2: 1", "Contains 5: 0", "Union size: 4"])
run_test("stdlib/Test", "stdlib/Test.japl", ["PASS", "FAIL"])
run_test("stdlib/Time", "stdlib/Time.japl", ["Time module loaded"], use_runtime=True)
run_test("stdlib/Env", "stdlib/Env.japl", ["Env module loaded"], use_runtime=True)
run_test("stdlib/Crypto", "stdlib/Crypto.japl", ["Crypto module loaded"], use_runtime=True)
compile_only_test("stdlib/File", "stdlib/File.japl")
run_test("stdlib/Bytes", "stdlib/Bytes.japl", ["Bytes length: 5", "Bytes to_string: hello", "Byte 0: 104", "After set: Hello", "Slice 1..4: ell", "Concat: hello world", "Bytes module loaded"])
run_test("stdlib/Codec", "stdlib/Codec.japl", ["encode_int(42)=42", "encode_str=hello", "tag(IntVal)=0", "tag(StrVal)=1", "tag(PairVal)=2", "roundtrip int: 99", "roundtrip str: world"])
run_test("stdlib/Retry", "stdlib/Retry.japl", ["max_retries=3", "exp delay 0=100", "exp delay 1=200", "exp delay 2=400", "const delay 0=500", "const delay 1=500", "const delay 2=500"])
run_test("stdlib/Log", "stdlib/Log.japl", ["[DEBUG] debug message", "[INFO] info message", "[WARN] warn message", "[ERROR] error message", "level_name=INFO"])
run_test("stdlib/Config", "stdlib/Config.japl", ["get_or=8080", "get_int=4", "require=required:APP_NAME", "Config module loaded"])
compile_only_test("stdlib/LLM", "stdlib/LLM.japl")

print("\n--- Apps ---")
run_test("kvstore_app", "apps/kvstore/kvstore.japl", ["PUT key=0", "GET key=0", "DEL key=", "NOT FOUND", "SIZE partition"], use_runtime=True, retries=3)
run_test("msgqueue", "apps/msgqueue/queue.japl", ["enqueued", "dequeued", "acked", "Queue Complete"], use_runtime=True, retries=3)
run_test("scheduler", "apps/scheduler/scheduler.japl", ["Assigning task", "Finished task", "completed (12/12)"], use_runtime=True, retries=3)
run_test("genome_pipeline", "apps/genome/pipeline.japl", ["Regulatory", "Structural", "NonCoding", "GC Content", "Pipeline Complete"], use_runtime=True, retries=3)

print("\n--- HTTP Serving ---")
# Use unified binary's serve subcommand
kv_path = os.path.join(JAPL_HOME, "apps/http-kv/kv_server.japl")
srv = subprocess.Popen([JAPL, "serve", kv_path, "--port", "18923"],
                       stdout=subprocess.PIPE, stderr=subprocess.PIPE)
time.sleep(3)
try:
    for path, expected in [("/health", "ok"), ("/", "JAPL KV Store"), ("/put/x/1", "OK")]:
        r = subprocess.run(["curl", "-s", f"http://127.0.0.1:18923{path}"],
                           capture_output=True, text=True, timeout=5)
        if expected in r.stdout:
            print(f"  PASS http_serve GET {path}")
            PASS += 1
        else:
            print(f"  FAIL http_serve GET {path}: expected '{expected}' in '{r.stdout}'")
            FAIL += 1
finally:
    srv.terminate()
    srv.wait()

print("\n--- Type Checker ---")
r = subprocess.run([JAPL, "check",
                     os.path.join(JAPL_HOME, "test/programs/type_error.japl")],
                    capture_output=True, text=True)
if "error" in r.stdout.lower() or "error" in r.stderr.lower():
    print("  PASS type_checker")
    PASS += 1
else:
    print("  FAIL type_checker")
    FAIL += 1

print("\n--- Negative Checker Tests ---")
neg_dir = os.path.join(JAPL_HOME, "test/checker-negative")
if os.path.isdir(neg_dir):
    neg_files = sorted([f for f in os.listdir(neg_dir) if f.endswith(".japl")])
    neg_pass = 0
    neg_total = len(neg_files)
    for f in neg_files:
        r = subprocess.run([JAPL, "check", os.path.join(neg_dir, f)],
                           capture_output=True, text=True, timeout=10)
        has_error = "error" in r.stdout.lower() or "error" in r.stderr.lower()
        if has_error:
            neg_pass += 1
        else:
            print(f"  FAIL neg/{f}: expected type error but got none")
            FAIL += 1
    if neg_pass == neg_total:
        print(f"  PASS {neg_total}/{neg_total} negative tests reject correctly")
        PASS += 1
    else:
        print(f"  {neg_pass}/{neg_total} negative tests passed")

print(f"\n=== Results: {PASS} pass, {FAIL} fail ===")
sys.exit(0 if FAIL == 0 else 1)
