# JAPL Distributed Tests

Tests that verify JAPL's distributed runtime works across separate containers on different networks.

## Prerequisites

- Docker
- Docker Compose

## Run Tests

```bash
./run-tests.sh
```

## Test 1: Two-Node Communication

- **alpha** (172.28.0.x): Runs a counter process, listens on :9000
- **beta** (172.29.0.x): Connects to alpha, sends messages
- Networks are separate, connected via a bridge network
- Verifies: connection, handshake, message serialization, cross-node delivery

## Test 2: Chaos (Node Failure)

- **alpha**: Counter with `restart: unless-stopped`
- **beta**: Client that sends messages
- **chaos**: Kills alpha after 15s, verifies restart + recovery
- Verifies: node-down detection, supervisor restart, reconnection
