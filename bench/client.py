"""Minimal Python client for synapsed. Length-prefixed msgpack over unix socket."""
import msgpack
import socket
import struct
import time
import json
import random
import sys

class Client:
    def __init__(self, sock_path="/tmp/synapse.sock"):
        self.s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        self.s.connect(sock_path)

    def _call(self, req):
        body = msgpack.packb(req)
        self.s.sendall(struct.pack("<I", len(body)) + body)
        head = self._recv(4)
        n = struct.unpack("<I", head)[0]
        return msgpack.unpackb(self._recv(n), raw=False)

    def _recv(self, n):
        buf = b""
        while len(buf) < n:
            chunk = self.s.recv(n - len(buf))
            if not chunk:
                raise IOError("eof")
            buf += chunk
        return buf

    def ping(self):
        return self._call({"op": "Ping"})

    def put(self, text, title=None, embed=False):
        return self._call({"op": "Put", "args": {
            "title": title, "uri": None, "text": text, "meta": None, "embed": embed
        }})

    def put_batch(self, items):
        args = [{"title": i.get("title"), "uri": None, "text": i["text"],
                 "meta": None, "embed": i.get("embed", False)} for i in items]
        return self._call({"op": "PutBatch", "args": args})

    def search(self, q, mode="Lex", limit=10, embed_query=False):
        return self._call({"op": "Search", "args": {
            "mode": mode, "q": q, "limit": limit, "embed_query": embed_query
        }})

    def stats(self):
        return self._call({"op": "Stats"})

    def snap(self, out, level=3):
        return self._call({"op": "Snap", "args": {"out": out, "level": level}})


if __name__ == "__main__":
    cmd = sys.argv[1] if len(sys.argv) > 1 else "bench"
    c = Client()

    if cmd == "ping":
        print(c.ping())
    elif cmd == "bench":
        N = int(sys.argv[2]) if len(sys.argv) > 2 else 1000
        random.seed(42)
        words = "auth token jwt session refresh user admin api cache queue worker shard index vector embedding fts tantivy hnsw sqlite rust python node typescript react nextjs docker deploy bug fix refactor migration schema table column latency bench test".split()
        docs = [{"title": f"doc{i}", "text": " ".join(random.choices(words, k=30)), "embed": False}
                for i in range(N)]
        t0 = time.time()
        ids = c.put_batch(docs)
        t1 = time.time()
        print(f"put_batch {N}: {(t1-t0)*1000:.1f}ms ({N/(t1-t0):.0f} docs/s)")

        queries = ["auth", "token", "bug", "fix", "cache", "shard", "admin", "react", "docker", "python"]
        t0 = time.time()
        for q in queries:
            c.search(q, mode="Lex", limit=10)
        t1 = time.time()
        print(f"lex search avg: {(t1-t0)*1000/len(queries):.2f}ms/q")

        t0 = time.time()
        for _ in range(1000):
            c.ping()
        t1 = time.time()
        print(f"ping RTT: {(t1-t0)*1000/1000:.3f}ms/call (1000 pings)")

        print(json.dumps(c.stats(), indent=2))
    elif cmd == "bench-embed":
        N = int(sys.argv[2]) if len(sys.argv) > 2 else 100
        random.seed(42)
        words = "auth token jwt session refresh user admin api cache queue worker".split()
        docs = [{"title": f"doc{i}", "text": " ".join(random.choices(words, k=30)), "embed": True}
                for i in range(N)]
        t0 = time.time()
        c.put_batch(docs)
        t1 = time.time()
        print(f"put_batch +embed {N}: {(t1-t0)*1000:.1f}ms ({N/(t1-t0):.1f} docs/s)")

        t0 = time.time()
        for _ in range(10):
            c.search("auth cache", mode="Hybrid", limit=10, embed_query=True)
        t1 = time.time()
        print(f"hybrid RRF avg: {(t1-t0)*1000/10:.2f}ms/q")
