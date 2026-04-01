#!/usr/bin/env python3
"""External Python client for the JAPL KV Store Service.

Proves that an external application (Python) can connect to a running
JAPL distributed service via NATS and perform real operations.

The JAPL service stays alive as a persistent distributed process system.
This Python script acts as an external client — NOT part of the JAPL runtime.

Prerequisites:
  - nats-server running: nats-server -js
  - japl-provider running: cd japl-provider && cargo run --release
  - kvstore service running:
      japl run --distributed apps/kvstore-service/kvstore_service.japl
    (note the coordinator PID from the output)

Usage:
  python3 test/deploy/test_kvstore_service.py <coordinator_pid>
"""
import subprocess, sys, json, time, struct

PASS = 0
FAIL = 0

# JAPL variant tags matching the KVCmd ADT:
#   CmdPut = 0, CmdGet = 1, CmdDel = 2, CmdSize = 3
# Reply tags matching KVReply ADT:
#   ReplyOk = 0, ReplyValue = 1, ReplyNotFound = 2, ReplyCount = 3

def nats_request(subject, payload, timeout=5):
    """Send a NATS request/reply."""
    try:
        r = subprocess.run(
            ["nats", "request", subject, payload, "--timeout", f"{timeout}s"],
            capture_output=True, text=True, timeout=timeout + 2
        )
        if r.returncode == 0:
            lines = r.stdout.strip().split('\n')
            for line in reversed(lines):
                line = line.strip()
                if line:
                    return line
        return None
    except (FileNotFoundError, subprocess.TimeoutExpired):
        return None

def spawn_client_pid():
    """Allocate a PID in the provider for receiving replies."""
    resp = nats_request("japl.runtime.spawn", '{"closure_data":[]}')
    if resp:
        try:
            return json.loads(resp).get("pid", 0)
        except json.JSONDecodeError:
            pass
    return 0

def send_msg(pid, msg_bytes):
    """Send message bytes to a JAPL process."""
    payload = json.dumps({"message": msg_bytes})
    return nats_request(f"japl.runtime.send.{pid}", payload)

def recv_msg(pid, timeout=10):
    """Receive message bytes from a JAPL process mailbox."""
    resp = nats_request(f"japl.runtime.receive.{pid}", "{}", timeout=timeout)
    if resp:
        try:
            return json.loads(resp).get("message", [])
        except json.JSONDecodeError:
            pass
    return None

def build_cmd_put(key, val, reply_pid):
    """CmdPut(key, val, reply_pid) — tag=0, 3 fields."""
    return list(struct.pack('<IIqqq', 0, 3, key, val, reply_pid))

def build_cmd_get(key, reply_pid):
    """CmdGet(key, reply_pid) — tag=1, 2 fields."""
    return list(struct.pack('<IIqq', 1, 2, key, reply_pid))

def build_cmd_del(key, reply_pid):
    """CmdDel(key, reply_pid) — tag=2, 2 fields."""
    return list(struct.pack('<IIqq', 2, 2, key, reply_pid))

def build_cmd_size(reply_pid):
    """CmdSize(reply_pid) — tag=3, 1 field."""
    return list(struct.pack('<IIq', 3, 1, reply_pid))

def parse_reply(msg_bytes):
    """Parse a KVReply: ReplyOk=0, ReplyValue=1, ReplyNotFound=2, ReplyCount=3."""
    if not msg_bytes or len(msg_bytes) < 8:
        return ("error", "no data")
    raw = bytes(msg_bytes)
    tag, fc = struct.unpack('<II', raw[:8])
    if tag == 0:  # ReplyOk(Int)
        val = struct.unpack('<q', raw[8:16])[0] if fc >= 1 and len(raw) >= 16 else 0
        return ("ok", val)
    elif tag == 1:  # ReplyValue(Int)
        val = struct.unpack('<q', raw[8:16])[0] if fc >= 1 and len(raw) >= 16 else 0
        return ("value", val)
    elif tag == 2:  # ReplyNotFound
        return ("not_found", None)
    elif tag == 3:  # ReplyCount(Int)
        val = struct.unpack('<q', raw[8:16])[0] if fc >= 1 and len(raw) >= 16 else 0
        return ("count", val)
    return ("unknown", tag)

def check(name, result, detail=""):
    global PASS, FAIL
    if result:
        print(f"  PASS  {name}")
        PASS += 1
    else:
        print(f"  FAIL  {name}" + (f": {detail}" if detail else ""))
        FAIL += 1

def main():
    print("=" * 60)
    print("  JAPL KV Store — External Python Client Test")
    print("=" * 60)

    if len(sys.argv) < 2:
        print("\nUsage: python3 test_kvstore_service.py <coordinator_pid>")
        print("\nStart the service first:")
        print("  japl run --distributed apps/kvstore-service/kvstore_service.japl")
        sys.exit(1)

    svc_pid = int(sys.argv[1])
    print(f"\nService coordinator PID: {svc_pid}")

    # Allocate a client PID for receiving replies
    client_pid = spawn_client_pid()
    if client_pid == 0:
        print("FAIL: Cannot allocate client PID (is provider running?)")
        sys.exit(1)
    print(f"Client reply PID: {client_pid}\n")

    # --- PUT Operations ---
    print("--- PUT Operations ---")
    for key, val in [(42, 100), (43, 200), (44, 300), (99, 999)]:
        send_msg(svc_pid, build_cmd_put(key, val, client_pid))
        time.sleep(0.3)
        reply = recv_msg(client_pid, timeout=5)
        result = parse_reply(reply)
        check(f"PUT key={key} val={val}", result[0] == "ok", f"got {result}")

    # --- GET Operations ---
    print("\n--- GET Operations ---")
    for key, expected in [(42, 100), (43, 200), (44, 300), (99, 999)]:
        send_msg(svc_pid, build_cmd_get(key, client_pid))
        time.sleep(0.3)
        reply = recv_msg(client_pid, timeout=5)
        result = parse_reply(reply)
        check(f"GET key={key} -> {expected}", result == ("value", expected), f"got {result}")

    # --- GET Missing Key ---
    print("\n--- Missing Key ---")
    send_msg(svc_pid, build_cmd_get(777, client_pid))
    time.sleep(0.3)
    reply = recv_msg(client_pid, timeout=5)
    result = parse_reply(reply)
    check("GET key=777 -> NOT FOUND", result[0] == "not_found", f"got {result}")

    # --- DEL + Verify ---
    print("\n--- DEL + Verify ---")
    send_msg(svc_pid, build_cmd_del(42, client_pid))
    time.sleep(0.3)
    reply = recv_msg(client_pid, timeout=5)
    result = parse_reply(reply)
    check("DEL key=42", result[0] == "ok", f"got {result}")

    send_msg(svc_pid, build_cmd_get(42, client_pid))
    time.sleep(0.3)
    reply = recv_msg(client_pid, timeout=5)
    result = parse_reply(reply)
    check("GET key=42 after DEL -> NOT FOUND", result[0] == "not_found", f"got {result}")

    # --- Summary ---
    print(f"\n{'=' * 60}")
    print(f"  Results: {PASS} pass, {FAIL} fail")
    if FAIL == 0:
        print("  EXTERNAL CLIENT TEST: PASS")
        print("  Python client successfully used JAPL KV store over NATS")
    else:
        print("  EXTERNAL CLIENT TEST: FAIL")
    print("=" * 60)
    sys.exit(0 if FAIL == 0 else 1)

if __name__ == "__main__":
    main()
