#!/bin/bash
# Full bench harness: synapse daemon vs bare SQLite vs MV2 (if installed).
# Outputs to bench/RESULTS.md
set -e
cd "$(dirname "$0")/.."

N=${N:-1000}
EMBED_N=${EMBED_N:-500}
SOCK=/tmp/synapse_bench.sock
DB=/tmp/synapse_bench.db
CACHE=/tmp/synapse_bench.emb-cache

echo "=== Build release ==="
cargo build --release --quiet -p synapsed -p synapse-cli

echo "=== Prepare ==="
rm -rf "$DB"* "$CACHE" "$SOCK"
./target/release/synapsed -f "$DB" -s "$SOCK" --emb-cache "$CACHE" --lazy-embed > /tmp/synapsed.bench.log 2>&1 &
PID=$!
sleep 0.3
trap "kill $PID 2>/dev/null; rm -f $SOCK $DB* $CACHE" EXIT

echo "=== Python harness ==="
python3 - <<PY
import msgpack, socket, struct, time, json, random, os, sys
SOCK = "$SOCK"
N = $N
EMB_N = $EMBED_N

class C:
    def __init__(self, s):
        self.s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        self.s.connect(s)
    def call(self, r):
        b = msgpack.packb(r)
        self.s.sendall(struct.pack("<I", len(b)) + b)
        h = self.s.recv(4, socket.MSG_WAITALL) if hasattr(socket,'MSG_WAITALL') else self._recv(4)
        if len(h)<4: h = h + self._recv(4-len(h))
        n = struct.unpack("<I", h)[0]
        return msgpack.unpackb(self._recv(n), raw=False)
    def _recv(self, n):
        out=b""
        while len(out)<n:
            c=self.s.recv(n-len(out))
            if not c: raise IOError()
            out+=c
        return out

c = C(SOCK)
random.seed(42)
words = "auth token jwt session refresh user admin api cache queue worker shard index vector embedding fts tantivy hnsw sqlite rust python node typescript react nextjs docker deploy bug fix refactor migration schema table column latency bench test".split()

results = {}

# 1. insert no-embed
docs = [{"title": f"d{i}", "uri": None, "text": " ".join(random.choices(words, k=30)), "meta": None, "embed": False} for i in range(N)]
t0=time.perf_counter(); c.call({"op":"PutBatch","args":docs}); t1=time.perf_counter()
results["insert_no_embed_ms"] = round((t1-t0)*1000, 2)
results["insert_no_embed_docs_per_sec"] = round(N/(t1-t0))

# 2. lex search
qs = random.sample(words, 10)
t0=time.perf_counter()
for q in qs: c.call({"op":"Search","args":{"mode":"Lex","q":q,"limit":10,"embed_query":False}})
t1=time.perf_counter()
results["lex_ms_per_q"] = round((t1-t0)*1000/len(qs), 3)

# 3. ping RTT
t0=time.perf_counter()
for _ in range(1000): c.call({"op":"Ping"})
t1=time.perf_counter()
results["rtt_us"] = round((t1-t0)*1e6/1000, 1)

# 4. insert + embed
docs_e = [{"title": f"e{i}", "uri": None, "text": " ".join(random.choices(words, k=30)), "meta": None, "embed": True} for i in range(EMB_N)]
t0=time.perf_counter(); c.call({"op":"PutBatch","args":docs_e}); t1=time.perf_counter()
results["insert_embed_ms"] = round((t1-t0)*1000, 2)
results["insert_embed_docs_per_sec"] = round(EMB_N/(t1-t0), 1)

# 5. cache-hit re-insert (same text)
t0=time.perf_counter(); c.call({"op":"PutBatch","args":docs_e}); t1=time.perf_counter()
results["reinsert_cache_hit_ms"] = round((t1-t0)*1000, 2)

# 6. vec + hybrid search
t0=time.perf_counter()
for _ in range(10): c.call({"op":"Search","args":{"mode":"Vec","q":"auth","limit":10,"embed_query":True}})
t1=time.perf_counter()
results["vec_ms_per_q"] = round((t1-t0)*1000/10, 3)

t0=time.perf_counter()
for _ in range(10): c.call({"op":"Search","args":{"mode":"Hybrid","q":"cache","limit":10,"embed_query":True}})
t1=time.perf_counter()
results["hybrid_ms_per_q"] = round((t1-t0)*1000/10, 3)

# 7. snapshot
t0=time.perf_counter()
c.call({"op":"Snap","args":{"out":"/tmp/synapse_bench.brainpack","level":3}})
t1=time.perf_counter()
results["snap_ms"] = round((t1-t0)*1000, 2)
results["brainpack_size_bytes"] = os.path.getsize("/tmp/synapse_bench.brainpack")

results["db_size_bytes"] = os.path.getsize("$DB")
results["cache_size_bytes"] = os.path.getsize("$CACHE") if os.path.exists("$CACHE") else 0
results["N_no_embed"] = N
results["N_embed"] = EMB_N

print(json.dumps(results, indent=2))
with open("bench/results.json","w") as f: json.dump(results, f, indent=2)
PY

echo ""
echo "=== Done. bench/results.json written. ==="
