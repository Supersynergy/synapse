#!/usr/bin/env python3
"""Bounded write-back buffer for high-frequency Synapse Memory producers.

Normal events stay in RAM for a short, explicit durability window and are
committed by ``synx put-batch`` in one SQLite transaction. Explicit operator
writes should continue to use ``synx put`` or ``synx remember`` directly.
"""

from __future__ import annotations

import argparse
import collections
import fcntl
import hashlib
import json
import os
import pathlib
import signal
import socket
import socketserver
import subprocess
import sys
import tempfile
import threading
import time
from typing import Any


def env_int(name: str, default: int, minimum: int) -> int:
    return max(minimum, int(os.environ.get(name, str(default))))


def default_state_dir() -> pathlib.Path:
    base = os.environ.get("XDG_STATE_HOME")
    if base:
        return pathlib.Path(os.path.expanduser(base)) / "synapse-memory" / "writeback"
    return pathlib.Path.home() / ".local" / "state" / "synapse-memory" / "writeback"


STATE_DIR = pathlib.Path(
    os.path.expanduser(
        os.environ.get("SYNAPSE_WRITEBACK_STATE_DIR", str(default_state_dir()))
    )
)
BUFFER_SOCKET = pathlib.Path(
    os.path.expanduser(
        os.environ.get(
            "SYNAPSE_WRITEBACK_SOCKET",
            str(
                pathlib.Path(tempfile.gettempdir())
                / f"synapse-writeback-{os.getuid()}.sock"
            ),
        )
    )
)
BRAIN_DB = pathlib.Path(
    os.path.expanduser(
        os.environ.get("SYNAPSE_DB", "~/.synapse/brain.db")
    )
)
SYNX_BIN = os.environ.get("SYNAPSE_WRITEBACK_SYNX", "synx")
SPILL_PATH = STATE_DIR / "spill.jsonl"
SPILL_LOCK = STATE_DIR / "spill.lock"
FLUSH_SECONDS = env_int("SYNAPSE_WRITEBACK_FLUSH_SECONDS", 120, 1)
RETRY_SECONDS = env_int("SYNAPSE_WRITEBACK_RETRY_SECONDS", 30, 1)
MAX_ITEMS = env_int("SYNAPSE_WRITEBACK_MAX_ITEMS", 64, 1)
MAX_BYTES = env_int("SYNAPSE_WRITEBACK_MAX_BYTES", 4 * 1024 * 1024, 64 * 1024)
MAX_ITEM_BYTES = min(
    MAX_BYTES,
    env_int("SYNAPSE_WRITEBACK_MAX_ITEM_BYTES", 256 * 1024, 1024),
)
MAX_CLIENT_FRAME = MAX_BYTES + 512 * 1024
DOWNSTREAM_TIMEOUT = float(
    os.environ.get("SYNAPSE_WRITEBACK_DOWNSTREAM_TIMEOUT", "60")
)


def ensure_state_dir() -> None:
    STATE_DIR.mkdir(parents=True, mode=0o700, exist_ok=True)
    os.chmod(STATE_DIR, 0o700)
    for path in (SPILL_PATH, SPILL_LOCK):
        if path.exists():
            os.chmod(path, 0o600)


def fsync_dir(path: pathlib.Path) -> None:
    try:
        fd = os.open(path, os.O_RDONLY)
    except OSError:
        return
    try:
        os.fsync(fd)
    finally:
        os.close(fd)


def canonical_item(raw: dict[str, Any]) -> dict[str, Any]:
    title = raw.get("title")
    if title is not None:
        title = str(title)[:512]
    uri = raw.get("uri")
    if uri is not None:
        uri = str(uri)[:4096]
    text = str(raw.get("text") or "").strip()
    meta = raw.get("meta")
    if meta is not None and not isinstance(meta, dict):
        raise ValueError("meta must be a JSON object or null")
    if raw.get("embedding") not in (None, []):
        raise ValueError("write-back accepts text only; embed after durable ingest")
    if not text:
        raise ValueError("empty text")

    request = {
        "title": title,
        "uri": uri,
        "text": text,
        "meta": meta,
        "embedding": None,
    }
    canonical = json.dumps(
        request, ensure_ascii=False, sort_keys=True, separators=(",", ":")
    ).encode("utf-8")
    if len(canonical) > MAX_ITEM_BYTES:
        raise ValueError(f"item too large: {len(canonical)} > {MAX_ITEM_BYTES}")
    key = str(raw.get("key") or hashlib.sha256(canonical).hexdigest())
    return {"key": key, **request}


def item_size(item: dict[str, Any]) -> int:
    return len(
        json.dumps(
            item, ensure_ascii=False, sort_keys=True, separators=(",", ":")
        ).encode("utf-8")
    )


def spill_items(items: list[dict[str, Any]]) -> None:
    if not items:
        return
    ensure_state_dir()
    with SPILL_LOCK.open("a+") as lock:
        os.fchmod(lock.fileno(), 0o600)
        fcntl.flock(lock.fileno(), fcntl.LOCK_EX)
        try:
            with SPILL_PATH.open("a", encoding="utf-8") as out:
                os.fchmod(out.fileno(), 0o600)
                for item in items:
                    out.write(json.dumps(item, ensure_ascii=False) + "\n")
                out.flush()
                os.fsync(out.fileno())
            fsync_dir(STATE_DIR)
        finally:
            fcntl.flock(lock.fileno(), fcntl.LOCK_UN)


def load_spill() -> list[dict[str, Any]]:
    if not SPILL_PATH.exists():
        return []
    ensure_state_dir()
    loaded: collections.OrderedDict[str, dict[str, Any]] = collections.OrderedDict()
    loaded_bytes = 0
    with SPILL_LOCK.open("a+") as lock:
        os.fchmod(lock.fileno(), 0o600)
        fcntl.flock(lock.fileno(), fcntl.LOCK_SH)
        try:
            with SPILL_PATH.open(encoding="utf-8") as src:
                for line in src:
                    try:
                        item = canonical_item(json.loads(line))
                    except (ValueError, json.JSONDecodeError, TypeError):
                        continue
                    size = item_size(item)
                    if len(loaded) >= MAX_ITEMS or loaded_bytes + size > MAX_BYTES:
                        break
                    if item["key"] not in loaded:
                        loaded[item["key"]] = item
                        loaded_bytes += size
        finally:
            fcntl.flock(lock.fileno(), fcntl.LOCK_UN)
    return list(loaded.values())


def remove_spilled(keys: set[str]) -> None:
    if not keys or not SPILL_PATH.exists():
        return
    ensure_state_dir()
    temp = STATE_DIR / f"spill.tmp.{os.getpid()}"
    with SPILL_LOCK.open("a+") as lock:
        os.fchmod(lock.fileno(), 0o600)
        fcntl.flock(lock.fileno(), fcntl.LOCK_EX)
        try:
            with SPILL_PATH.open(encoding="utf-8") as src, temp.open(
                "w", encoding="utf-8"
            ) as out:
                os.fchmod(out.fileno(), 0o600)
                for line in src:
                    try:
                        raw = json.loads(line)
                        key = canonical_item(raw)["key"]
                    except (ValueError, json.JSONDecodeError, TypeError):
                        out.write(line)
                        continue
                    if key not in keys:
                        out.write(json.dumps(raw, ensure_ascii=False) + "\n")
                out.flush()
                os.fsync(out.fileno())
            os.replace(temp, SPILL_PATH)
            fsync_dir(STATE_DIR)
        finally:
            temp.unlink(missing_ok=True)
            fcntl.flock(lock.fileno(), fcntl.LOCK_UN)


def downstream_put_batch(items: list[dict[str, Any]]) -> list[int]:
    requests = [
        {
            "title": item["title"],
            "uri": item["uri"],
            "text": item["text"],
            "meta": item["meta"],
            "embedding": None,
        }
        for item in items
    ]
    payload = "".join(
        json.dumps(request, ensure_ascii=False, separators=(",", ":")) + "\n"
        for request in requests
    ).encode("utf-8")
    command = [
        SYNX_BIN,
        "-f",
        str(BRAIN_DB),
        "put-batch",
        "--max-items",
        str(len(requests)),
        "--max-bytes",
        str(max(len(payload), 1)),
    ]
    result = subprocess.run(
        command,
        input=payload,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        timeout=DOWNSTREAM_TIMEOUT,
        check=False,
    )
    if result.returncode != 0:
        error = result.stderr.decode("utf-8", "replace").strip()
        raise RuntimeError(f"synx put-batch failed ({result.returncode}): {error[:300]}")
    try:
        response = json.loads(result.stdout.decode("utf-8", "replace").strip())
        ids = response["ids"]
    except (UnicodeDecodeError, json.JSONDecodeError, KeyError, TypeError) as exc:
        raise RuntimeError("invalid synx put-batch response") from exc
    if not isinstance(ids, list) or len(ids) != len(items):
        raise RuntimeError("synx put-batch response count mismatch")
    return [int(value) for value in ids]


class MemoryBuffer:
    def __init__(self) -> None:
        self.lock = threading.Lock()
        self.condition = threading.Condition(self.lock)
        self.flush_lock = threading.Lock()
        self.pending: collections.OrderedDict[str, dict[str, Any]] = (
            collections.OrderedDict()
        )
        self.pending_bytes = 0
        self.stop_event = threading.Event()
        self.total_received = 0
        self.total_flushed = 0
        self.flush_count = 0
        self.failure_count = 0
        self.last_flush_at: float | None = None
        self.last_error: str | None = None
        self.started_at = time.time()
        recovered = load_spill()
        self.durable_keys = {item["key"] for item in recovered}
        self.enqueue(recovered, count_received=False, spill_overflow=False)

    def enqueue(
        self,
        items: list[dict[str, Any]],
        *,
        count_received: bool = True,
        spill_overflow: bool = True,
    ) -> tuple[int, int]:
        accepted = 0
        overflow: list[dict[str, Any]] = []
        with self.condition:
            for raw in items:
                item = canonical_item(raw)
                key = item["key"]
                if key in self.pending:
                    continue
                size = item_size(item)
                if (
                    len(self.pending) >= MAX_ITEMS
                    or self.pending_bytes + size > MAX_BYTES
                ):
                    overflow.append(item)
                    continue
                self.pending[key] = item
                self.pending_bytes += size
                accepted += 1
            if count_received:
                self.total_received += accepted + len(overflow)
            if len(self.pending) >= MAX_ITEMS or self.pending_bytes >= MAX_BYTES:
                self.condition.notify_all()
        if spill_overflow and overflow:
            spill_items(overflow)
            with self.lock:
                self.durable_keys.update(item["key"] for item in overflow)
        return accepted, len(overflow)

    def status(self) -> dict[str, Any]:
        with self.lock:
            return {
                "status": "running",
                "pending_items": len(self.pending),
                "pending_bytes": self.pending_bytes,
                "spill_bytes": SPILL_PATH.stat().st_size if SPILL_PATH.exists() else 0,
                "flush_seconds": FLUSH_SECONDS,
                "retry_seconds": RETRY_SECONDS,
                "max_items": MAX_ITEMS,
                "max_bytes": MAX_BYTES,
                "durability_window_seconds": FLUSH_SECONDS,
                "total_received": self.total_received,
                "total_flushed": self.total_flushed,
                "flush_count": self.flush_count,
                "failure_count": self.failure_count,
                "last_flush_at": self.last_flush_at,
                "last_error": self.last_error,
                "uptime_seconds": round(time.time() - self.started_at, 1),
            }

    def flush(self) -> bool:
        if not self.flush_lock.acquire(blocking=False):
            return False
        snapshot: list[dict[str, Any]] = []
        try:
            with self.condition:
                if not self.pending:
                    return True
                snapshot = list(self.pending.values())
                self.pending.clear()
                self.pending_bytes = 0

            downstream_put_batch(snapshot)
            keys = {item["key"] for item in snapshot}
            remove_spilled(keys)
            with self.condition:
                for key in keys:
                    duplicate = self.pending.pop(key, None)
                    if duplicate is not None:
                        self.pending_bytes -= item_size(duplicate)
                self.durable_keys.difference_update(keys)
                self.total_flushed += len(snapshot)
                self.flush_count += 1
                self.last_flush_at = time.time()
                self.last_error = None

            recovered = load_spill()
            with self.lock:
                self.durable_keys.update(item["key"] for item in recovered)
            self.enqueue(recovered, count_received=False, spill_overflow=False)
            return True
        except Exception as exc:
            with self.lock:
                new_spill = [
                    item
                    for item in snapshot
                    if item["key"] not in self.durable_keys
                ]
            if new_spill:
                spill_items(new_spill)
            with self.condition:
                self.durable_keys.update(item["key"] for item in new_spill)
                restored = collections.OrderedDict(
                    (item["key"], item) for item in snapshot
                )
                restored.update(self.pending)
                self.pending = restored
                self.pending_bytes = sum(
                    item_size(item) for item in self.pending.values()
                )
                self.failure_count += 1
                self.last_error = str(exc)[:300]
            print(f"synapse-writeback: flush failed: {exc}", file=sys.stderr)
            return False
        finally:
            self.flush_lock.release()

    def flusher(self) -> None:
        deadline = time.monotonic() + FLUSH_SECONDS
        while not self.stop_event.is_set():
            with self.condition:
                wait_for = max(0.0, deadline - time.monotonic())
                self.condition.wait(timeout=min(wait_for, 1.0))
                high_water = (
                    len(self.pending) >= MAX_ITEMS
                    or self.pending_bytes >= MAX_BYTES
                )
            if high_water or time.monotonic() >= deadline:
                success = self.flush()
                delay = FLUSH_SECONDS if success else RETRY_SECONDS
                deadline = time.monotonic() + delay


class RequestHandler(socketserver.StreamRequestHandler):
    def handle(self) -> None:
        raw = self.rfile.readline(MAX_CLIENT_FRAME + 1)
        if not raw or len(raw) > MAX_CLIENT_FRAME:
            return
        try:
            request = json.loads(raw)
            operation = request.get("op")
            state: MemoryBuffer = self.server.memory_state  # type: ignore[attr-defined]
            if operation == "enqueue":
                queued, spilled = state.enqueue([request["item"]])
                response = {"queued": queued, "spilled": spilled, **state.status()}
            elif operation == "enqueue_batch":
                queued, spilled = state.enqueue(list(request.get("items") or []))
                response = {"queued": queued, "spilled": spilled, **state.status()}
            elif operation == "flush":
                success = state.flush()
                response = {"flushed": success, **state.status()}
            elif operation == "status":
                response = state.status()
            else:
                response = {"error": f"unknown operation: {operation}"}
        except Exception as exc:
            response = {"error": str(exc)}
        self.wfile.write(
            json.dumps(response, ensure_ascii=False).encode("utf-8") + b"\n"
        )


class BufferServer(socketserver.ThreadingMixIn, socketserver.UnixStreamServer):
    daemon_threads = True
    allow_reuse_address = True


def buffer_request(request: dict[str, Any], timeout: float = 2.0) -> dict[str, Any]:
    body = json.dumps(request, ensure_ascii=False).encode("utf-8") + b"\n"
    if len(body) > MAX_CLIENT_FRAME:
        raise ValueError("buffer request too large")
    with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as client:
        client.settimeout(timeout)
        client.connect(str(BUFFER_SOCKET))
        client.sendall(body)
        response = bytearray()
        while len(response) <= MAX_CLIENT_FRAME:
            chunk = client.recv(65536)
            if not chunk:
                break
            response.extend(chunk)
            if b"\n" in chunk:
                break
    if not response:
        raise ConnectionError("empty buffer response")
    parsed = json.loads(bytes(response).split(b"\n", 1)[0])
    if "error" in parsed:
        raise RuntimeError(parsed["error"])
    return parsed


def serve() -> int:
    ensure_state_dir()
    BUFFER_SOCKET.parent.mkdir(parents=True, exist_ok=True)
    if BUFFER_SOCKET.exists():
        try:
            buffer_request({"op": "status"}, timeout=0.3)
        except Exception:
            BUFFER_SOCKET.unlink(missing_ok=True)
        else:
            print("synapse-writeback: already running", file=sys.stderr)
            return 0

    state = MemoryBuffer()
    server = BufferServer(str(BUFFER_SOCKET), RequestHandler)
    server.timeout = 1.0
    server.memory_state = state  # type: ignore[attr-defined]
    os.chmod(BUFFER_SOCKET, 0o600)

    def request_stop(_signum: int, _frame: Any) -> None:
        state.stop_event.set()

    signal.signal(signal.SIGTERM, request_stop)
    signal.signal(signal.SIGINT, request_stop)
    flusher = threading.Thread(
        target=state.flusher, name="synapse-writeback-flush"
    )
    flusher.start()
    try:
        while not state.stop_event.is_set():
            server.handle_request()
    finally:
        state.stop_event.set()
        with state.condition:
            state.condition.notify_all()
        flusher.join(timeout=3)
        if not state.flush():
            with state.lock:
                missing = [
                    item
                    for item in state.pending.values()
                    if item["key"] not in state.durable_keys
                ]
            spill_items(missing)
        server.server_close()
        BUFFER_SOCKET.unlink(missing_ok=True)
    return 0


def read_jsonl_items() -> list[dict[str, Any]]:
    items: list[dict[str, Any]] = []
    total_bytes = 0
    line_number = 0
    line_limit = min(MAX_BYTES, MAX_ITEM_BYTES + 64 * 1024)
    while True:
        raw_line = sys.stdin.buffer.readline(line_limit + 1)
        if not raw_line:
            break
        line_number += 1
        if len(raw_line) > line_limit:
            raise ValueError(f"JSONL line {line_number} exceeds {line_limit} bytes")
        total_bytes += len(raw_line)
        if total_bytes > MAX_BYTES:
            raise ValueError(f"input exceeds {MAX_BYTES} bytes")
        line = raw_line.decode("utf-8")
        if not line.strip():
            continue
        if len(items) >= MAX_ITEMS:
            raise ValueError(f"input exceeds {MAX_ITEMS} items")
        try:
            raw = json.loads(line)
        except json.JSONDecodeError as exc:
            raise ValueError(f"invalid JSONL at line {line_number}") from exc
        items.append(canonical_item(raw))
    return items


def enqueue_or_spill(items: list[dict[str, Any]]) -> dict[str, Any]:
    if not items:
        return {"queued": 0, "spilled": 0}
    try:
        if len(items) == 1:
            request = {"op": "enqueue", "item": items[0]}
        else:
            request = {"op": "enqueue_batch", "items": items}
        return buffer_request(request)
    except Exception:
        spill_items(items)
        return {
            "queued": 0,
            "spilled": len(items),
            "status": "buffer_offline",
        }


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Bounded RAM write-back for high-frequency Synapse writes"
    )
    commands = parser.add_subparsers(dest="command", required=True)
    commands.add_parser("serve")
    enqueue = commands.add_parser("enqueue")
    enqueue.add_argument("--title")
    enqueue.add_argument("--uri")
    enqueue.add_argument("--meta")
    commands.add_parser("enqueue-jsonl")
    commands.add_parser("status")
    commands.add_parser("flush")
    args = parser.parse_args()

    if args.command == "serve":
        return serve()
    if args.command == "status":
        try:
            print(json.dumps(buffer_request({"op": "status"})))
            return 0
        except Exception:
            print(
                json.dumps(
                    {
                        "status": "offline",
                        "spill_bytes": (
                            SPILL_PATH.stat().st_size if SPILL_PATH.exists() else 0
                        ),
                    }
                )
            )
            return 1
    if args.command == "flush":
        response = buffer_request({"op": "flush"})
        print(json.dumps(response))
        return 0 if response.get("flushed") else 1
    if args.command == "enqueue":
        meta = None
        if args.meta:
            meta = json.loads(args.meta)
            if not isinstance(meta, dict):
                raise ValueError("--meta must be a JSON object")
        raw_text = sys.stdin.buffer.read(MAX_ITEM_BYTES + 1)
        if len(raw_text) > MAX_ITEM_BYTES:
            raise ValueError(f"input exceeds {MAX_ITEM_BYTES} bytes")
        item = canonical_item(
            {
                "title": args.title,
                "uri": args.uri,
                "text": raw_text.decode("utf-8"),
                "meta": meta,
            }
        )
        print(json.dumps(enqueue_or_spill([item])))
        return 0
    if args.command == "enqueue-jsonl":
        print(json.dumps(enqueue_or_spill(read_jsonl_items())))
        return 0
    return 2


if __name__ == "__main__":
    try:
        exit_code = main()
    except (ConnectionError, json.JSONDecodeError, RuntimeError, ValueError) as exc:
        print(f"synapse-writeback: {exc}", file=sys.stderr)
        exit_code = 2
    raise SystemExit(exit_code)
