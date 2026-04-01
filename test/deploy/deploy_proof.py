#!/usr/bin/env python3
"""JAPL Deploy Functional Test

Exercises the FULL deployed process path:
  1. Component compilation (--target component)
  2. WADM manifest generation (--dry-run)
  3. Provider self-test (spawn/send/receive over NATS)
  4. Direct NATS process messaging verification

Prerequisites:
  - nats-server running with JetStream: nats-server -js
  - japl-provider running: cd japl-provider && cargo run --release

Usage:
  python3 test/deploy/deploy_proof.py

Returns exit code 0 on success, 1 on failure.
"""
import subprocess, os, sys, json, time

JAPL_HOME = os.path.join(os.path.dirname(__file__), "..", "..")
JAPL = os.path.join(JAPL_HOME, "japl/target/release/japl")
PASS = 0
FAIL = 0

def check(name, result, detail=""):
    global PASS, FAIL
    if result:
        print(f"  PASS  {name}")
        PASS += 1
        return True
    else:
        msg = f"  FAIL  {name}"
        if detail:
            msg += f": {detail}"
        print(msg)
        FAIL += 1
        return False

def nats_request(subject, payload="", timeout=5):
    """Send a NATS request/reply and return the response."""
    try:
        cmd = ["nats", "request", subject, payload, "--timeout", f"{timeout}s"]
        r = subprocess.run(cmd, capture_output=True, text=True, timeout=timeout+2)
        if r.returncode == 0:
            # nats cli prints response after header lines
            lines = r.stdout.strip().split('\n')
            # Find the actual response body (last non-empty line)
            for line in reversed(lines):
                line = line.strip()
                if line:
                    return line
        return None
    except (FileNotFoundError, subprocess.TimeoutExpired):
        return None

def main():
    global PASS, FAIL
    print("=== JAPL Deploy Functional Test ===\n")

    # Check prerequisites
    has_nats_cli = subprocess.run(["nats", "--version"], capture_output=True).returncode == 0 \
        if os.popen("which nats").read().strip() else False

    # 1. Component compilation
    print("--- Component Build ---")
    app = os.path.join(JAPL_HOME, "apps/distributed/hello_distributed.japl")
    r = subprocess.run([JAPL, "build", app, "--target", "component", "--out", "/tmp"],
                       capture_output=True, text=True)
    check("component compilation", r.returncode == 0,
          r.stderr.strip() if r.returncode != 0 else "")

    # 2. Manifest generation
    print("\n--- Manifest Generation ---")
    r = subprocess.run([JAPL, "deploy", "--dry-run", app],
                       capture_output=True, text=True)
    has_manifest = r.returncode == 0 and "apiVersion" in r.stdout
    check("WADM manifest (--dry-run)", has_manifest)

    # 3. Provider health check via NATS
    print("\n--- Provider Functional Tests ---")
    if not has_nats_cli:
        print("  SKIP nats CLI not found — install with: brew install nats-io/nats-tools/nats")
        print("  (provider tests require nats CLI for request/reply)")
    else:
        # Health check
        health = nats_request("japl.runtime.health", "{}")
        if health:
            try:
                h = json.loads(health)
                check("provider health", h.get("status") == "ok",
                      f"got: {health}")
            except json.JSONDecodeError:
                check("provider health", False, f"invalid JSON: {health}")
        else:
            check("provider health", False, "no response (is japl-provider running?)")

        # Spawn a process
        spawn_resp = nats_request("japl.runtime.spawn", '{"closure_data":[]}')
        spawned_pid = None
        if spawn_resp:
            try:
                s = json.loads(spawn_resp)
                spawned_pid = s.get("pid")
                check("spawn process", spawned_pid is not None and spawned_pid > 0,
                      f"got pid={spawned_pid}")
            except json.JSONDecodeError:
                check("spawn process", False, f"invalid JSON: {spawn_resp}")
        else:
            check("spawn process", False, "no response")

        # Send a message to the spawned process
        if spawned_pid:
            send_resp = nats_request(f"japl.runtime.send.{spawned_pid}",
                                     '{"message":[72,101,108,108,111]}')
            check("send message", send_resp is not None and "ok" in str(send_resp).lower(),
                  f"got: {send_resp}")

            # Receive the message back
            recv_resp = nats_request(f"japl.runtime.receive.{spawned_pid}", "{}", timeout=3)
            if recv_resp:
                try:
                    rv = json.loads(recv_resp)
                    msg_bytes = rv.get("message", [])
                    check("receive message", len(msg_bytes) > 0,
                          f"got {len(msg_bytes)} bytes")
                except json.JSONDecodeError:
                    check("receive message", "message" in recv_resp, f"got: {recv_resp}")
            else:
                check("receive message", False, "timeout (expected — message already delivered)")

    # Summary
    print(f"\n=== Results: {PASS} pass, {FAIL} fail ===")
    if FAIL == 0:
        print("DEPLOY FUNCTIONAL TEST: PASS")
    else:
        print("DEPLOY FUNCTIONAL TEST: FAIL")
    sys.exit(0 if FAIL == 0 else 1)

if __name__ == "__main__":
    main()
