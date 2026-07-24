#!/usr/bin/env python3
from __future__ import annotations

import json
import os
import pathlib
import signal
import socket
import subprocess
import sys
import tempfile
import time
import unittest


HERE = pathlib.Path(__file__).resolve().parent
SCRIPT = HERE / "synapse_writeback.py"


class WritebackIntegrationTest(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory()
        self.root = pathlib.Path(self.temp.name)
        self.socket = self.root / "writeback.sock"
        self.state = self.root / "state"
        self.log = self.root / "successful-batches.jsonl"
        self.mode = self.root / "mode"
        self.mode.write_text("ok", encoding="utf-8")
        self.fake_synx = self.root / "fake-synx"
        self.fake_synx.write_text(
            """#!/usr/bin/env python3
import json
import os
import pathlib
import sys

items = [json.loads(line) for line in sys.stdin if line.strip()]
if pathlib.Path(os.environ["FAKE_SYNX_MODE"]).read_text().strip() != "ok":
    print("injected downstream failure", file=sys.stderr)
    raise SystemExit(23)
with pathlib.Path(os.environ["FAKE_SYNX_LOG"]).open("a", encoding="utf-8") as out:
    out.write(json.dumps({"count": len(items), "items": items}) + "\\n")
print(json.dumps({"count": len(items), "ids": list(range(1, len(items) + 1))}))
""",
            encoding="utf-8",
        )
        self.fake_synx.chmod(0o700)
        self.env = {
            **os.environ,
            "SYNAPSE_WRITEBACK_SOCKET": str(self.socket),
            "SYNAPSE_WRITEBACK_STATE_DIR": str(self.state),
            "SYNAPSE_WRITEBACK_SYNX": str(self.fake_synx),
            "SYNAPSE_DB": str(self.root / "brain.db"),
            "SYNAPSE_WRITEBACK_FLUSH_SECONDS": "3600",
            "SYNAPSE_WRITEBACK_RETRY_SECONDS": "1",
            "SYNAPSE_WRITEBACK_MAX_ITEMS": "64",
            "SYNAPSE_WRITEBACK_MAX_BYTES": str(4 * 1024 * 1024),
            "FAKE_SYNX_MODE": str(self.mode),
            "FAKE_SYNX_LOG": str(self.log),
        }
        self.server: subprocess.Popen[str] | None = None

    def tearDown(self) -> None:
        self.stop_server()
        self.temp.cleanup()

    def run_cli(
        self, *args: str, stdin: str = "", check: bool = True
    ) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            [sys.executable, str(SCRIPT), *args],
            input=stdin,
            text=True,
            capture_output=True,
            env=self.env,
            check=check,
            timeout=10,
        )

    def start_server(self) -> None:
        self.server = subprocess.Popen(
            [sys.executable, str(SCRIPT), "serve"],
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            env=self.env,
        )
        deadline = time.monotonic() + 5
        while time.monotonic() < deadline:
            if self.socket.exists():
                try:
                    response = self.request({"op": "status"})
                    if response.get("status") == "running":
                        return
                except OSError:
                    pass
            if self.server.poll() is not None:
                break
            time.sleep(0.02)
        stdout, stderr = self.server.communicate(timeout=1)
        self.fail(f"server did not start\nstdout={stdout}\nstderr={stderr}")

    def stop_server(self, hard: bool = False) -> None:
        if self.server is None or self.server.poll() is not None:
            return
        self.server.send_signal(signal.SIGKILL if hard else signal.SIGTERM)
        self.server.wait(timeout=5)
        if self.server.stdout is not None:
            self.server.stdout.close()
        if self.server.stderr is not None:
            self.server.stderr.close()

    def request(self, payload: dict[str, object]) -> dict[str, object]:
        with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as client:
            client.settimeout(2)
            client.connect(str(self.socket))
            client.sendall(json.dumps(payload).encode("utf-8") + b"\n")
            raw = bytearray()
            while b"\n" not in raw:
                chunk = client.recv(65536)
                if not chunk:
                    break
                raw.extend(chunk)
        return json.loads(bytes(raw).split(b"\n", 1)[0])

    def successful_batches(self) -> list[dict[str, object]]:
        if not self.log.exists():
            return []
        return [
            json.loads(line)
            for line in self.log.read_text(encoding="utf-8").splitlines()
            if line
        ]

    def test_twenty_events_become_one_transaction(self) -> None:
        self.start_server()
        for index in range(20):
            response = self.request(
                {
                    "op": "enqueue",
                    "item": {
                        "title": f"event-{index}",
                        "text": f"payload-{index}",
                        "meta": {"source": "test"},
                    },
                }
            )
            self.assertEqual(response["queued"], 1)

        flushed = self.request({"op": "flush"})
        self.assertTrue(flushed["flushed"])
        self.assertEqual(flushed["pending_items"], 0)
        batches = self.successful_batches()
        self.assertEqual(len(batches), 1)
        self.assertEqual(batches[0]["count"], 20)

    def test_duplicate_is_coalesced_in_ram(self) -> None:
        self.start_server()
        item = {"title": "same", "text": "same payload"}
        first = self.request({"op": "enqueue", "item": item})
        second = self.request({"op": "enqueue", "item": item})
        self.assertEqual(first["queued"], 1)
        self.assertEqual(second["queued"], 0)
        self.assertEqual(second["pending_items"], 1)
        self.assertTrue(self.request({"op": "flush"})["flushed"])
        self.assertEqual(self.successful_batches()[0]["count"], 1)

    def test_periodic_deadline_flushes_without_operator_request(self) -> None:
        self.env["SYNAPSE_WRITEBACK_FLUSH_SECONDS"] = "1"
        self.start_server()
        self.request(
            {
                "op": "enqueue",
                "item": {"title": "timer", "text": "periodic payload"},
            }
        )
        deadline = time.monotonic() + 4
        while time.monotonic() < deadline and not self.successful_batches():
            time.sleep(0.05)
        batches = self.successful_batches()
        self.assertEqual(len(batches), 1)
        self.assertEqual(batches[0]["count"], 1)

    def test_cli_rejects_item_and_batch_overflow(self) -> None:
        self.env["SYNAPSE_WRITEBACK_MAX_ITEM_BYTES"] = "1024"
        oversized = self.run_cli("enqueue", stdin="x" * 1025, check=False)
        self.assertNotEqual(oversized.returncode, 0)
        self.assertIn("input exceeds 1024 bytes", oversized.stderr)

        lines = "".join(
            json.dumps({"title": str(index), "text": f"item-{index}"}) + "\n"
            for index in range(65)
        )
        too_many = self.run_cli("enqueue-jsonl", stdin=lines, check=False)
        self.assertNotEqual(too_many.returncode, 0)
        self.assertIn("input exceeds 64 items", too_many.stderr)

    def test_offline_enqueue_spills_and_recovers(self) -> None:
        result = self.run_cli(
            "enqueue",
            "--title",
            "offline",
            stdin="recover me",
        )
        self.assertEqual(json.loads(result.stdout)["spilled"], 1)
        self.start_server()
        status = self.request({"op": "status"})
        self.assertEqual(status["pending_items"], 1)
        self.assertTrue(self.request({"op": "flush"})["flushed"])
        self.assertEqual(self.successful_batches()[0]["count"], 1)
        self.assertEqual((self.state / "spill.jsonl").stat().st_size, 0)

    def test_downstream_failure_is_spilled_before_hard_crash(self) -> None:
        self.mode.write_text("fail", encoding="utf-8")
        self.start_server()
        self.request(
            {
                "op": "enqueue",
                "item": {"title": "failure", "text": "survive hard crash"},
            }
        )
        failed = self.request({"op": "flush"})
        self.assertFalse(failed["flushed"])
        self.assertGreater((self.state / "spill.jsonl").stat().st_size, 0)
        self.stop_server(hard=True)

        self.mode.write_text("ok", encoding="utf-8")
        self.server = None
        self.start_server()
        self.assertTrue(self.request({"op": "flush"})["flushed"])
        batches = self.successful_batches()
        self.assertEqual(len(batches), 1)
        self.assertEqual(batches[0]["items"][0]["text"], "survive hard crash")


if __name__ == "__main__":
    unittest.main()
