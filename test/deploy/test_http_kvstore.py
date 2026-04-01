#!/usr/bin/env python3
"""External HTTP client test for the JAPL KV Store Service.

Proves that an external application can interact with a running JAPL
distributed service via HTTP — no NATS knowledge needed.

The JAPL service runs with:
  japl run --distributed --http-port 8090 apps/kvstore-service/kvstore_service.japl

This script talks to it via standard HTTP:
  PUT /kv/{key}/{val}  → store a value
  GET /kv/{key}        → retrieve a value
  DELETE /kv/{key}     → delete a value
  GET /health          → check service health

Usage:
  python3 test/deploy/test_http_kvstore.py [port]
  Default port: 8090
"""
import sys, json
from urllib.request import urlopen, Request
from urllib.error import URLError

PASS = 0
FAIL = 0

def check(name, result, detail=""):
    global PASS, FAIL
    if result:
        print(f"  PASS  {name}")
        PASS += 1
    else:
        print(f"  FAIL  {name}" + (f": {detail}" if detail else ""))
        FAIL += 1

def http_request(method, url, timeout=15):
    try:
        req = Request(url, method=method)
        resp = urlopen(req, timeout=timeout)
        body = resp.read().decode()
        return json.loads(body)
    except URLError as e:
        return {"error": str(e)}
    except json.JSONDecodeError:
        return {"error": "invalid json"}

def main():
    port = int(sys.argv[1]) if len(sys.argv) > 1 else 8090
    base = f"http://localhost:{port}"

    print("=" * 60)
    print("  JAPL KV Store — HTTP Client Test")
    print(f"  Endpoint: {base}")
    print("=" * 60)

    # Health check
    print("\n--- Health Check ---")
    h = http_request("GET", f"{base}/health")
    check("GET /health", h.get("status") == "ok", f"got {h}")

    # PUT operations
    print("\n--- PUT Operations ---")
    for key, val in [(1, 111), (2, 222), (3, 333), (10, 1000), (20, 2000)]:
        r = http_request("PUT", f"{base}/kv/{key}/{val}")
        check(f"PUT /kv/{key}/{val}", r.get("status") == "ok", f"got {r}")

    # GET operations
    print("\n--- GET Operations ---")
    for key, expected in [(1, 111), (2, 222), (3, 333), (10, 1000), (20, 2000)]:
        r = http_request("GET", f"{base}/kv/{key}")
        check(f"GET /kv/{key} -> {expected}", r.get("value") == expected, f"got {r}")

    # Missing key
    print("\n--- Missing Key ---")
    r = http_request("GET", f"{base}/kv/9999")
    check("GET /kv/9999 -> not_found", r.get("error") == "not_found", f"got {r}")

    # DELETE + verify
    print("\n--- DELETE + Verify ---")
    r = http_request("DELETE", f"{base}/kv/1")
    check("DELETE /kv/1", r.get("status") == "ok" or r.get("deleted") == True, f"got {r}")

    r = http_request("GET", f"{base}/kv/1")
    check("GET /kv/1 after DELETE -> not_found", r.get("error") == "not_found", f"got {r}")

    # Summary
    print(f"\n{'=' * 60}")
    print(f"  Results: {PASS} pass, {FAIL} fail")
    if FAIL == 0:
        print("  HTTP CLIENT TEST: PASS")
        print(f"  External HTTP client used JAPL KV store at {base}")
    else:
        print("  HTTP CLIENT TEST: FAIL")
    print("=" * 60)
    sys.exit(0 if FAIL == 0 else 1)

if __name__ == "__main__":
    main()
