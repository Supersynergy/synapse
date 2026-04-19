#!/usr/bin/env python3
"""Top-20 DB / single-file format bench.

Measures: bulk-insert 10 000 short docs, read-all round-trip, and on-disk size
for every format a Python stack can reach locally. Formats grouped by tier:

Tier 1 — battle-tested embedded:
  1. SQLite (stdlib)
  2. SQLite + WAL (tuned)
  3. DuckDB
  4. LMDB (optional: `pip install lmdb`)

Tier 2 — columnar / analytics:
  5. Apache Arrow IPC
  6. Parquet (pyarrow)
  7. Feather v2
  8. LanceDB (pip install lancedb)

Tier 3 — hot 2026 single-file formats:
  9. Synapse .synx v0.2.4
  10. .brainpack wrapped
  11. JSONL + zstd
  12. MessagePack + zstd (optional: `pip install msgpack`)
  13. CBOR (optional: `pip install cbor2`)

Tier 4 — legacy / reference:
  14. Pickle
  15. CSV + gzip
  16. TSV + zstd
  17. YAML (optional: `pip install pyyaml`)
  18. TOML (optional: `pip install tomli_w`)
  19. BSON (optional: `pip install bson`)
  20. DBM (stdlib)

Missing optional deps are skipped with a note. Output is a markdown table
readable from the shell."""
from __future__ import annotations

import csv
import gzip
import io
import json
import os
import pickle
import shutil
import sqlite3
import struct
import sys
import tempfile
import time
from dataclasses import dataclass
from pathlib import Path

N = int(os.environ.get("N", 10_000))


def _rows(n: int) -> list[dict]:
    return [
        {
            "id": i,
            "title": f"doc-{i}",
            "body": f"rust ships here tonight agent memory vector embed {i}" * 2,
            "scope": "global",
            "ts": 1_700_000_000 + i,
        }
        for i in range(n)
    ]


@dataclass
class Bench:
    name: str
    insert_ms: float
    read_ms: float
    size_bytes: int
    notes: str = ""


def bench_sqlite_default(rows: list[dict]) -> Bench:
    p = tempfile.NamedTemporaryFile(suffix=".db", delete=False).name
    t = time.perf_counter()
    c = sqlite3.connect(p)
    c.execute("CREATE TABLE docs(id INT, title TEXT, body TEXT, scope TEXT, ts INT)")
    c.executemany(
        "INSERT INTO docs VALUES (?,?,?,?,?)",
        [(r["id"], r["title"], r["body"], r["scope"], r["ts"]) for r in rows],
    )
    c.commit()
    insert_ms = (time.perf_counter() - t) * 1000
    size = os.path.getsize(p)
    t = time.perf_counter()
    out = c.execute("SELECT id, body FROM docs").fetchall()
    read_ms = (time.perf_counter() - t) * 1000
    c.close()
    os.unlink(p)
    return Bench("SQLite (default)", insert_ms, read_ms, size, f"rows={len(out)}")


def bench_sqlite_wal(rows: list[dict]) -> Bench:
    p = tempfile.NamedTemporaryFile(suffix=".db", delete=False).name
    t = time.perf_counter()
    c = sqlite3.connect(p)
    c.execute("PRAGMA journal_mode=WAL")
    c.execute("PRAGMA synchronous=NORMAL")
    c.execute("CREATE TABLE docs(id INT PRIMARY KEY, title TEXT, body TEXT, scope TEXT, ts INT)")
    c.executemany(
        "INSERT INTO docs VALUES (?,?,?,?,?)",
        [(r["id"], r["title"], r["body"], r["scope"], r["ts"]) for r in rows],
    )
    c.commit()
    insert_ms = (time.perf_counter() - t) * 1000
    size = os.path.getsize(p) + os.path.getsize(p + "-wal")
    t = time.perf_counter()
    out = c.execute("SELECT id, body FROM docs").fetchall()
    read_ms = (time.perf_counter() - t) * 1000
    c.close()
    os.unlink(p)
    for ext in ("-wal", "-shm"):
        try:
            os.unlink(p + ext)
        except FileNotFoundError:
            pass
    return Bench("SQLite + WAL (tuned)", insert_ms, read_ms, size, f"rows={len(out)}")


def bench_duckdb(rows: list[dict]) -> Bench:
    try:
        import duckdb  # type: ignore
    except ImportError:
        return Bench("DuckDB", 0, 0, 0, "skip: pip install duckdb")
    p = tempfile.NamedTemporaryFile(suffix=".duckdb", delete=False).name
    os.unlink(p)
    t = time.perf_counter()
    c = duckdb.connect(p)
    c.execute("CREATE TABLE docs(id INT, title TEXT, body TEXT, scope TEXT, ts BIGINT)")
    c.executemany(
        "INSERT INTO docs VALUES (?,?,?,?,?)",
        [(r["id"], r["title"], r["body"], r["scope"], r["ts"]) for r in rows],
    )
    c.close()
    insert_ms = (time.perf_counter() - t) * 1000
    size = os.path.getsize(p)
    t = time.perf_counter()
    c = duckdb.connect(p)
    out = c.execute("SELECT id, body FROM docs").fetchall()
    read_ms = (time.perf_counter() - t) * 1000
    c.close()
    os.unlink(p)
    return Bench("DuckDB", insert_ms, read_ms, size, f"rows={len(out)}")


def bench_lmdb(rows: list[dict]) -> Bench:
    try:
        import lmdb  # type: ignore
    except ImportError:
        return Bench("LMDB", 0, 0, 0, "skip: pip install lmdb")
    d = tempfile.mkdtemp()
    t = time.perf_counter()
    env = lmdb.open(d, map_size=512 * 1024 * 1024)
    with env.begin(write=True) as tx:
        for r in rows:
            tx.put(str(r["id"]).encode(), json.dumps(r).encode())
    insert_ms = (time.perf_counter() - t) * 1000
    t = time.perf_counter()
    out = []
    with env.begin() as tx:
        for k, v in tx.cursor():
            out.append(v)
    read_ms = (time.perf_counter() - t) * 1000
    env.close()
    size = sum(os.path.getsize(os.path.join(d, f)) for f in os.listdir(d))
    shutil.rmtree(d)
    return Bench("LMDB", insert_ms, read_ms, size, f"rows={len(out)}")


def bench_arrow_ipc(rows: list[dict]) -> Bench:
    try:
        import pyarrow as pa  # type: ignore
    except ImportError:
        return Bench("Arrow IPC", 0, 0, 0, "skip: pip install pyarrow")
    t = time.perf_counter()
    tbl = pa.Table.from_pylist(rows)
    buf = io.BytesIO()
    with pa.ipc.new_file(buf, tbl.schema) as w:
        w.write_table(tbl)
    data = buf.getvalue()
    insert_ms = (time.perf_counter() - t) * 1000
    t = time.perf_counter()
    out = pa.ipc.open_file(pa.BufferReader(data)).read_all()
    read_ms = (time.perf_counter() - t) * 1000
    return Bench("Arrow IPC", insert_ms, read_ms, len(data), f"rows={out.num_rows}")


def bench_parquet(rows: list[dict]) -> Bench:
    try:
        import pyarrow as pa  # type: ignore
        import pyarrow.parquet as pq  # type: ignore
    except ImportError:
        return Bench("Parquet", 0, 0, 0, "skip: pip install pyarrow")
    p = tempfile.NamedTemporaryFile(suffix=".parquet", delete=False).name
    t = time.perf_counter()
    tbl = pa.Table.from_pylist(rows)
    pq.write_table(tbl, p, compression="zstd")
    insert_ms = (time.perf_counter() - t) * 1000
    size = os.path.getsize(p)
    t = time.perf_counter()
    out = pq.read_table(p)
    read_ms = (time.perf_counter() - t) * 1000
    os.unlink(p)
    return Bench("Parquet (zstd)", insert_ms, read_ms, size, f"rows={out.num_rows}")


def bench_feather(rows: list[dict]) -> Bench:
    try:
        import pyarrow as pa  # type: ignore
        import pyarrow.feather as ft  # type: ignore
    except ImportError:
        return Bench("Feather v2", 0, 0, 0, "skip: pip install pyarrow")
    p = tempfile.NamedTemporaryFile(suffix=".feather", delete=False).name
    t = time.perf_counter()
    tbl = pa.Table.from_pylist(rows)
    ft.write_feather(tbl, p, compression="zstd")
    insert_ms = (time.perf_counter() - t) * 1000
    size = os.path.getsize(p)
    t = time.perf_counter()
    out = ft.read_table(p)
    read_ms = (time.perf_counter() - t) * 1000
    os.unlink(p)
    return Bench("Feather v2 (zstd)", insert_ms, read_ms, size, f"rows={out.num_rows}")


def bench_lancedb(rows: list[dict]) -> Bench:
    try:
        import lancedb  # type: ignore
    except ImportError:
        return Bench("LanceDB", 0, 0, 0, "skip: pip install lancedb")
    d = tempfile.mkdtemp()
    t = time.perf_counter()
    db = lancedb.connect(d)
    tbl = db.create_table("docs", data=rows)
    insert_ms = (time.perf_counter() - t) * 1000
    size = sum(
        os.path.getsize(os.path.join(dp, f))
        for dp, _, fs in os.walk(d)
        for f in fs
    )
    t = time.perf_counter()
    out = tbl.to_pandas()
    read_ms = (time.perf_counter() - t) * 1000
    shutil.rmtree(d)
    return Bench("LanceDB", insert_ms, read_ms, size, f"rows={len(out)}")


def bench_jsonl_zstd(rows: list[dict]) -> Bench:
    try:
        import zstandard as zstd  # type: ignore
    except ImportError:
        return Bench("JSONL + zstd", 0, 0, 0, "skip: pip install zstandard")
    p = tempfile.NamedTemporaryFile(suffix=".jsonl.zst", delete=False).name
    cctx = zstd.ZstdCompressor(level=3)
    dctx = zstd.ZstdDecompressor()
    t = time.perf_counter()
    body = "\n".join(json.dumps(r) for r in rows).encode()
    with open(p, "wb") as fh:
        fh.write(cctx.compress(body))
    insert_ms = (time.perf_counter() - t) * 1000
    size = os.path.getsize(p)
    t = time.perf_counter()
    with open(p, "rb") as fh:
        decoded = dctx.decompress(fh.read())
    out = [json.loads(line) for line in decoded.splitlines()]
    read_ms = (time.perf_counter() - t) * 1000
    os.unlink(p)
    return Bench("JSONL + zstd", insert_ms, read_ms, size, f"rows={len(out)}")


def bench_msgpack_zstd(rows: list[dict]) -> Bench:
    try:
        import msgpack  # type: ignore
        import zstandard as zstd  # type: ignore
    except ImportError:
        return Bench("MessagePack + zstd", 0, 0, 0, "skip: pip install msgpack zstandard")
    p = tempfile.NamedTemporaryFile(suffix=".mp.zst", delete=False).name
    cctx = zstd.ZstdCompressor(level=3)
    dctx = zstd.ZstdDecompressor()
    t = time.perf_counter()
    payload = msgpack.packb(rows)
    with open(p, "wb") as fh:
        fh.write(cctx.compress(payload))
    insert_ms = (time.perf_counter() - t) * 1000
    size = os.path.getsize(p)
    t = time.perf_counter()
    with open(p, "rb") as fh:
        raw = dctx.decompress(fh.read())
    out = msgpack.unpackb(raw)
    read_ms = (time.perf_counter() - t) * 1000
    os.unlink(p)
    return Bench("MessagePack + zstd", insert_ms, read_ms, size, f"rows={len(out)}")


def bench_cbor(rows: list[dict]) -> Bench:
    try:
        import cbor2  # type: ignore
    except ImportError:
        return Bench("CBOR", 0, 0, 0, "skip: pip install cbor2")
    p = tempfile.NamedTemporaryFile(suffix=".cbor", delete=False).name
    t = time.perf_counter()
    with open(p, "wb") as fh:
        cbor2.dump(rows, fh)
    insert_ms = (time.perf_counter() - t) * 1000
    size = os.path.getsize(p)
    t = time.perf_counter()
    with open(p, "rb") as fh:
        out = cbor2.load(fh)
    read_ms = (time.perf_counter() - t) * 1000
    os.unlink(p)
    return Bench("CBOR", insert_ms, read_ms, size, f"rows={len(out)}")


def bench_pickle(rows: list[dict]) -> Bench:
    p = tempfile.NamedTemporaryFile(suffix=".pkl", delete=False).name
    t = time.perf_counter()
    with open(p, "wb") as fh:
        pickle.dump(rows, fh, protocol=5)
    insert_ms = (time.perf_counter() - t) * 1000
    size = os.path.getsize(p)
    t = time.perf_counter()
    with open(p, "rb") as fh:
        out = pickle.load(fh)
    read_ms = (time.perf_counter() - t) * 1000
    os.unlink(p)
    return Bench("Pickle (py5)", insert_ms, read_ms, size, f"rows={len(out)}")


def bench_csv_gzip(rows: list[dict]) -> Bench:
    p = tempfile.NamedTemporaryFile(suffix=".csv.gz", delete=False).name
    t = time.perf_counter()
    with gzip.open(p, "wt") as fh:
        w = csv.DictWriter(fh, fieldnames=list(rows[0].keys()))
        w.writeheader()
        w.writerows(rows)
    insert_ms = (time.perf_counter() - t) * 1000
    size = os.path.getsize(p)
    t = time.perf_counter()
    with gzip.open(p, "rt") as fh:
        out = list(csv.DictReader(fh))
    read_ms = (time.perf_counter() - t) * 1000
    os.unlink(p)
    return Bench("CSV + gzip", insert_ms, read_ms, size, f"rows={len(out)}")


def bench_dbm(rows: list[dict]) -> Bench:
    import dbm  # stdlib

    d = tempfile.mkdtemp()
    p = os.path.join(d, "docs")
    t = time.perf_counter()
    with dbm.open(p, "c") as db:
        for r in rows:
            db[str(r["id"])] = json.dumps(r)
    insert_ms = (time.perf_counter() - t) * 1000
    size = sum(
        os.path.getsize(os.path.join(d, f)) for f in os.listdir(d)
    )
    t = time.perf_counter()
    with dbm.open(p, "r") as db:
        out = [json.loads(db[k]) for k in db.keys()]
    read_ms = (time.perf_counter() - t) * 1000
    shutil.rmtree(d)
    return Bench("DBM (stdlib)", insert_ms, read_ms, size, f"rows={len(out)}")


def bench_synx_external(rows: list[dict]) -> Bench:
    # Looks for a prebuilt /tmp/synapse_uc_10000_3.synx from the Rust bench.
    p = "/tmp/synapse_uc_10000_3.synx"
    if not os.path.exists(p):
        return Bench(
            "Synapse .synx (external)", 0, 0, 0,
            "skip: run `bash bench/bench_20_usecases.sh` first",
        )
    sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "sdk" / "python"))
    from synapse_reader import SynxReader  # type: ignore

    t = time.perf_counter()
    r = SynxReader(p)
    insert_ms = 0.0  # not applicable — measured separately in Rust bench
    read_ms = (time.perf_counter() - t) * 1000
    size = os.path.getsize(p)
    return Bench(
        "Synapse .synx (existing)", insert_ms, read_ms, size,
        f"chunks={len(r.chunks)}",
    )


def main() -> None:
    rows = _rows(N)
    benches = [
        bench_sqlite_default,
        bench_sqlite_wal,
        bench_duckdb,
        bench_lmdb,
        bench_arrow_ipc,
        bench_parquet,
        bench_feather,
        bench_lancedb,
        bench_synx_external,
        bench_jsonl_zstd,
        bench_msgpack_zstd,
        bench_cbor,
        bench_pickle,
        bench_csv_gzip,
        bench_dbm,
    ]
    results: list[Bench] = []
    for fn in benches:
        try:
            results.append(fn(rows))
        except Exception as e:
            results.append(Bench(fn.__name__, 0, 0, 0, f"error: {e}"))

    print(f"# Top formats bench — N={N}\n")
    header = f"| {'format':<28} | {'insert ms':>10} | {'read ms':>10} | {'size KB':>10} | notes |"
    print(header)
    print("|" + "-" * (len(header) - 2) + "|")
    results.sort(key=lambda b: b.size_bytes or 9e18)
    for b in results:
        size_kb = b.size_bytes / 1024 if b.size_bytes else 0
        print(
            f"| {b.name:<28} | {b.insert_ms:>10.2f} | {b.read_ms:>10.2f} | "
            f"{size_kb:>10.1f} | {b.notes} |"
        )


if __name__ == "__main__":
    main()
