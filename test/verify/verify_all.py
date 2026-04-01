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
# Distribution tests:
#   dist_test         – basic remote spawn simulation (single worker, one message)
#   dist_cluster_test – multi-worker pipeline pattern (two workers, reply-back routing)
#   True multi-node tests require `japl run --node-name --peer` flags (Agent J scope)
run_test("dist_test", "test/programs/dist_test.japl", ["remote got: 42"], use_runtime=True, retries=3)
run_test("dist_cluster", "test/programs/dist_cluster_test.japl", ["Cluster test starting", "worker1:", "worker2:", "Cluster test done"], use_runtime=True, retries=3)

print("\n--- Processes ---")
run_test("processes", "test/programs/process.japl", ["got"], use_runtime=True, retries=3)
run_test("kvstore", "test/programs/kvstore.japl", ["stored", "value"], use_runtime=True, retries=3)

print("\n--- Stdlib ---")
run_test("stdlib/Math", "stdlib/Math.japl", ["abs(-5)=5", "max(3,7)=7", "min(3,7)=3", "clamp(10,0,5)=5", "pow(2,8)=256", "gcd(12,8)=4"])
run_test("stdlib/Option", "stdlib/Option.japl", ["is_some(Some(42))=1", "is_some(None)=0", "unwrap_or(Some(42),0)=42", "unwrap_or(None,0)=0", "and_then(Some(10),+1)=11", "and_then(None,+1)=0", "filter(Some(10),>5)=10", "filter(Some(3),>5)=0", "get_or_else(Some(42))=42", "get_or_else(None)=99"])
run_test("stdlib/Result", "stdlib/Result.japl", ["is_ok(Ok(42))=1", "is_ok(Err)=0", "unwrap_or(Ok(42),0)=42", "unwrap_or(Err,0)=0", "and_then(Ok(10),+1)=11", "and_then(Err,+1)=0", "map_err(Err)=1", "map_err(Ok)=1", "or_else(Err)=99", "or_else(Ok)=42"])
run_test("stdlib/String", "stdlib/String.japl", ["concat: hello world", "repeat: ababab", "join: foo, bar", "contains hello world: 1", "starts_with hello: 1", "ends_with world: 1", "trim: [hello]", "to_upper: HELLO", "to_lower: hello", "replace: hello JAPL", "index_of: 6", "parse_int: 42", "length: 5"])
run_test("stdlib/IO", "stdlib/IO.japl", ["IO module loaded", "42", "[debug] test=99"])
run_test("stdlib/List", "stdlib/List.japl", ["length: 3", "sum: 6", "Contains 2: 1", "Contains 5: 0", "map(*2) head: 2", "filter(==2) len: 1", "fold(*) product: 6", "head: 1", "tail head: 2"])
run_test("stdlib/Json", "stdlib/Json.japl", ["Int:  42", "Bool: true", "Null: null"])
run_test("stdlib/Http", "stdlib/Http.japl", ["Method: GET", "Method: POST", "Status: 200 OK", "Status: 404 Not Found", "request=GET /api/users HTTP/1.1", "content_type=Content-Type: text/html", "header=X-Custom: value"])
run_test("stdlib/Process", "stdlib/Process.japl", ["Process module loaded", "own mailbox: 0", "alive: ", "worker alive: ", "done"], use_runtime=True, retries=3)
run_test("stdlib/Supervisor", "stdlib/Supervisor.japl", ["Supervisor module loaded", "child_spec ok", "restart_count: 0", "after inc_restart: 1", "done"], use_runtime=True, retries=3)
run_test("stdlib/Registry", "stdlib/Registry.japl", ["Registry module loaded", "Registry size: 3", "Lookup 1: 100", "After overwrite, Lookup 2: 999", "After overwrite, size: 3", "reg_count: 3", "After update 1: 150", "done"])
compile_only_test("stdlib/Net", "stdlib/Net.japl")
run_test("stdlib/Map", "stdlib/Map.japl", ["Map size: 3", "Get 1: 100", "Get 2: 200", "Contains 2: 1", "Contains 5: 0", "StrMap size: 3", "StrMap get alice: 42", "StrMap get bob: 99", "StrMap get unknown: -1", "StrMap contains bob: 1", "StrMap contains dave: 0", "StrMap after remove bob, size: 2", "StrMap after remove bob, contains bob: 0"])
run_test("stdlib/Set", "stdlib/Set.japl", ["Set size: 3", "Contains 1: 1", "Contains 2: 1", "Contains 5: 0", "Union size: 4", "StrSet size: 3", "StrSet contains apple: 1", "StrSet contains banana: 1", "StrSet contains grape: 0", "StrSet after remove banana, size: 2", "StrSet after remove banana, contains banana: 0"])
run_test("stdlib/Test", "stdlib/Test.japl", ["PASS", "FAIL"])
run_test("stdlib/Time", "stdlib/Time.japl", ["Time module loaded", "elapsed(100,350)=250", "now_positive=1"], use_runtime=True)
run_test("stdlib/Env", "stdlib/Env.japl", ["Env module loaded", "HOME_set=1", "missing_default=fallback"], use_runtime=True)
run_test("stdlib/Crypto", "stdlib/Crypto.japl", ["Crypto module loaded", "alloc_valid=1"], use_runtime=True)
compile_only_test("stdlib/File", "stdlib/File.japl")
run_test("stdlib/Bytes", "stdlib/Bytes.japl", ["Bytes length: 5", "Bytes to_string: hello", "Byte 0: 104", "After set: Hello", "Slice 1..4: ell", "Concat: hello world", "Bytes module loaded"])
run_test("stdlib/Codec", "stdlib/Codec.japl", ["encode_int(42)=42", "encode_str=hello", "tag(IntVal)=0", "tag(StrVal)=1", "tag(PairVal)=2", "roundtrip int: 99", "roundtrip str: world", "tag(ListVal)=3", "is_int(IntVal)=1", "is_int(StrVal)=0", "is_str(StrVal)=1", "is_str(IntVal)=0"])
run_test("stdlib/Retry", "stdlib/Retry.japl", ["max_retries=3", "exp delay 0=100", "exp delay 1=200", "exp delay 2=400", "const delay 0=500", "const delay 1=500", "const delay 2=500"])
run_test("stdlib/Log", "stdlib/Log.japl", ["[DEBUG] debug message", "[INFO] info message", "[WARN] warn message", "[ERROR] error message", "level_name=INFO"])
run_test("stdlib/Config", "stdlib/Config.japl", ["get_or=8080", "get_int=4", "require=required:APP_NAME", "Config module loaded"], use_runtime=True)
compile_only_test("stdlib/LLM", "stdlib/LLM.japl")
run_test("stdlib/Core", "stdlib/Core.japl", ["identity(42)=42", "const_(10,20)=10", "pipe(5,*3)=15", "apply(+1,9)=10", "Core module loaded"])
run_test("stdlib/Tool", "stdlib/Tool.japl", ["tool_name: search", "tool_desc: Search the web", "call_ok: 1", "call_result: search({\"query\": \"hello\"})", "err_ok: 0", "exec_ok: 1", "exec_result: search(test_args) => ok", "tool_error_ok: 0", "tool_error_val: search: not found"])
run_test("stdlib/Budget", "stdlib/Budget.japl", ["remaining: 100", "max: 100", "after_spend: 70", "exhausted: 0", "check_50: 1", "exhausted_after: 1", "try_spend_ok: 70", "try_spend_fail: 70", "status_full: ok", "status_low: low", "status_exhausted: exhausted"])
run_test("stdlib/Replay", "stdlib/Replay.japl", ["empty_size: 0", "empty_latest: -1", "size: 3", "latest: 3", "event_size: 3", "latest_action: spawn", "empty_action: none"])
run_test("stdlib/Provenance", "stdlib/Provenance.japl", ["human: human", "model: model:claude", "tool: tool:search", "composed: human+model:claude", "hash: abc123", "timestamp: 1000", "llm_source: model:gpt4", "llm_ts: 2000", "tool_source: tool:bash", "tool_ts: 3000"])

print("\n--- Stdlib Import Tests ---")
run_test("stdlib_option_import", "test/programs/stdlib_option_test.japl", ["is_some=1", "unwrap=99"])
run_test("multi_import", "test/programs/multi_import_test.japl", ["unwrap=99", "is_ok=1"])
run_test("result_import", "test/programs/result_import_test.japl", ["is_ok=1", "is_ok_err=0", "unwrap=42", "unwrap_err=0"])
run_test("list_import", "test/programs/list_import_test.japl", ["length=3", "sum=6"])

print("\n--- Apps ---")
run_test("kvstore_app", "apps/kvstore/kvstore.japl", ["PUT key=0", "GET key=0", "DEL key=", "NOT FOUND", "SIZE partition"], use_runtime=True, retries=3)
run_test("msgqueue", "apps/msgqueue/queue.japl", ["enqueued", "dequeued", "acked", "Queue Complete"], use_runtime=True, retries=3)
run_test("scheduler", "apps/scheduler/scheduler.japl", ["Assigning task", "Finished task", "completed (12/12)"], use_runtime=True, retries=3)
run_test("genome_pipeline", "apps/genome/pipeline.japl", ["Regulatory", "Structural", "NonCoding", "GC Content", "Pipeline Complete"], use_runtime=True, retries=3)
run_test("agents_app", "apps/agents/agents.japl", ["Agent system starting", "Spawned classifier", "Spawned summarizer", "Sending tasks", "Classifier result:", "Summarizer result:", "All tasks complete (2/2)", "Agent system done"], use_runtime=True, retries=3)
run_test("distributed_hello", "apps/distributed/hello_distributed.japl",
    ["Distributed Hello starting", "Spawned 2 workers", "Result 1:", "Result 2:", "Distributed Hello complete"],
    use_runtime=True, retries=3)

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

print("\n--- Stdlib Completeness ---")
import glob
stdlib_files = set(
    os.path.basename(f).replace('.japl', '')
    for f in glob.glob(os.path.join(JAPL_HOME, 'stdlib/*.japl'))
)
# Build set of tested modules from the test entries above (stdlib/* test names)
tested_modules = set()
# Re-scan the test output isn't feasible, so hardcode from the test list.
# This list must be kept in sync with the stdlib test entries above.
_tested = [
    "Math", "Option", "Result", "String", "IO", "List", "Json", "Http",
    "Process", "Supervisor", "Registry", "Net", "Map", "Set", "Test",
    "Time", "Env", "Crypto", "File", "Bytes", "Codec", "Retry", "Log",
    "Config", "LLM", "Tool", "Budget", "Replay", "Provenance", "Core",
]
tested_modules = set(_tested)
untested = stdlib_files - tested_modules
if untested:
    print(f"  FAIL stdlib completeness: untested modules: {sorted(untested)}")
    FAIL += 1
else:
    print(f"  PASS all {len(stdlib_files)} stdlib modules have test coverage")
    PASS += 1

print(f"\n=== Results: {PASS} pass, {FAIL} fail ===")
sys.exit(0 if FAIL == 0 else 1)
