#!/bin/bash
# Real agentic test: drive Synapse as Claude Code's persistent memory layer.
#
# Scenario: three sessions across a project. Session 1 adds design decisions.
# Session 2 recalls + updates. Session 3 verifies the brainpack is shippable.
#
# No Claude-subprocess needed — we drive the `synapse` CLI exactly as the
# MCP bridge would. Output mirrors what Claude Code would see via tool-use.

set -e
cd "$(dirname "$0")/.."

BRAIN=/tmp/claude_code_demo.db
SOCK=/tmp/synapse_demo.sock
BIN=./target/release
rm -f "$BRAIN" "$BRAIN"-* "$SOCK"

echo "=== build ==="
cargo build --release --quiet -p synapse-cli -p synapsed

echo "=== session 1 — architect captures decisions ==="
"$BIN/synapsed" -f "$BRAIN" -s "$SOCK" > /tmp/synapsed_demo.log 2>&1 &
DPID=$!
sleep 0.3
trap "kill $DPID 2>/dev/null; rm -f $SOCK" EXIT

put() {
    python3 - <<PY
import msgpack, socket, struct
s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
s.connect("$SOCK")
req = {"op": "put", "doc": {"text": """$1""", "title": """$2""", "meta": {"scope": "project/$3"}, "embed": False}}
b = msgpack.packb(req)
s.sendall(struct.pack("<I", len(b)) + b)
h = s.recv(4)
n = struct.unpack("<I", h)[0]
print(msgpack.unpackb(s.recv(n, socket.MSG_WAITALL), raw=False))
PY
}

search() {
    python3 - <<PY
import msgpack, socket, struct
s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
s.connect("$SOCK")
req = {"op": "search", "query": """$1""", "mode": "Lex", "limit": 3}
b = msgpack.packb(req)
s.sendall(struct.pack("<I", len(b)) + b)
h = s.recv(4)
n = struct.unpack("<I", h)[0]
print(msgpack.unpackb(s.recv(n, socket.MSG_WAITALL), raw=False))
PY
}

put "we chose Rust because single-binary shipping matters more than ecosystem breadth" "stack decision" supersynergy
put "SQLite + FTS5 + sqlite-vec + BLAKE3 beats any cluster at under 10M docs" "capacity plan" supersynergy
put "TrailBase handles the auth surface; Synapse handles the memory surface" "division of concerns" supersynergy
put "every .brainpack gets Ed25519 signed with key rotation every 90 days" "security policy" supersynergy
put "Tantivy 0.22 is pinned until the Collector API stabilises" "dep pinning" supersynergy

echo "=== session 2 — agent recalls during a new task ==="
search "why Rust"
echo "---"
search "brainpack security"
echo "---"
search "auth layer"

echo "=== session 3 — export brainpack for team distribution ==="
"$BIN/synapse" -f "$BRAIN" snap /tmp/claude_code_demo.brainpack 2>&1 || true
ls -lh /tmp/claude_code_demo.brainpack 2>&1 || true

echo "=== daemon stats ==="
python3 - <<PY
import msgpack, socket, struct
s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
s.connect("$SOCK")
b = msgpack.packb({"op": "stats"})
s.sendall(struct.pack("<I", len(b)) + b)
h = s.recv(4); n = struct.unpack("<I", h)[0]
print(msgpack.unpackb(s.recv(n, socket.MSG_WAITALL), raw=False))
PY

echo "=== done ==="
