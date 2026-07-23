#!/usr/bin/env python3
"""Synapse Eval Harness — LoCoMo & LongMemEval benchmark runner.

Runs Synapse's hybrid search against two long-context memory benchmarks
and reports Recall@k, MRR, and latency percentiles.

Usage:
    python3 eval/harness.py download
    python3 eval/harness.py ingest --db /tmp/eval-brain.db
    python3 eval/harness.py run --db /tmp/eval-brain.db --k 5
    python3 eval/harness.py report
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import shutil
import statistics
import subprocess
import sys
import time
from dataclasses import dataclass, field, asdict
from pathlib import Path
from typing import Any
from urllib.request import urlretrieve

# ─── Configuration ─────────────────────────────────────────────────────────

EVAL_DIR = Path(__file__).resolve().parent
DATASETS_DIR = EVAL_DIR / "datasets"
RESULTS_DIR = EVAL_DIR / "results"
REPORTS_DIR = EVAL_DIR / "reports"

LOCOMO_URL = "https://raw.githubusercontent.com/snap-research/locomo/main/data/locomo10.json"
LONGMEMEVAL_URL = "https://raw.githubusercontent.com/xiaowu0162/LongMemEval/main/data/longmemeval_s_oracle.json"

SYNX_BIN = os.environ.get("SYNX_BIN", "synx")
ULTRA_BIN = os.environ.get("ULTRA_BIN", "synapse-ultra")


# ─── Data structures ───────────────────────────────────────────────────────

@dataclass
class Question:
    qid: str
    question: str
    gold_answer: str
    gold_passage_ids: list[str]
    category: str = ""


@dataclass
class Dataset:
    name: str
    questions: list[Question]
    passages: dict[str, str]  # pid -> text
    version: str = ""


@dataclass
class RetrievalResult:
    qid: str
    retrieved_ids: list[str]
    latency_ms: float
    hit: bool  # gold passage in top-k
    rank: int  # 0 if not found


@dataclass
class RunReport:
    dataset: str
    git_sha: str
    k: int
    total_questions: int
    recall_at_k: float
    mrr: float
    latency_p50_ms: float
    latency_p95_ms: float
    per_category: dict[str, dict[str, float]] = field(default_factory=dict)
    results: list[RetrievalResult] = field(default_factory=list)


# ─── Dataset loaders ───────────────────────────────────────────────────────

def _sha256_file(path: Path) -> str:
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(65536), b""):
            h.update(chunk)
    return h.hexdigest()


def download_datasets() -> None:
    """Download LoCoMo and LongMemEval datasets."""
    DATASETS_DIR.mkdir(parents=True, exist_ok=True)

    targets = [
        ("locomo", LOCOMO_URL, DATASETS_DIR / "locomo" / "locomo10.json"),
        ("longmemeval", LONGMEMEVAL_URL, DATASETS_DIR / "longmemeval" / "longmemeval_s_oracle.json"),
    ]

    for name, url, dest in targets:
        dest.parent.mkdir(parents=True, exist_ok=True)
        if dest.exists():
            print(f"[skip] {name} already at {dest}")
            continue
        print(f"[download] {name} ← {url}")
        urlretrieve(url, dest)
        sha = _sha256_file(dest)
        (dest.with_suffix(".sha256")).write_text(sha + "\n")
        print(f"  sha256: {sha}")


def load_locomo() -> Dataset:
    """Load LoCoMo dataset from local JSON."""
    path = DATASETS_DIR / "locomo" / "locomo10.json"
    if not path.exists():
        sys.exit(f"LoCoMo not downloaded. Run: python3 {__file__} download")

    raw = json.loads(path.read_text())
    questions: list[Question] = []
    passages: dict[str, str] = {}

    # LoCoMo format: list of conversations with qa_pairs
    for conv in raw if isinstance(raw, list) else [raw]:
        conv_id = str(conv.get("id", conv.get("conversation_id", "unknown")))
        # Ingest passages (each turn becomes a passage)
        turns = conv.get("conversation", conv.get("turns", []))
        for turn in turns:
            pid = f"{conv_id}#{turn.get('turn_id', turn.get('id', 0))}"
            text = turn.get("utterance", turn.get("text", ""))
            passages[pid] = text

        # Load QA pairs
        for qa in conv.get("qa_pairs", conv.get("questions", [])):
            qid = str(qa.get("id", qa.get("qid", len(questions))))
            gold_pids = []
            for ref in qa.get("references", qa.get("evidence", [])):
                if isinstance(ref, str):
                    gold_pids.append(ref)
                elif isinstance(ref, dict):
                    gold_pids.append(f"{conv_id}#{ref.get('turn_id', ref.get('id', 0))}")
            questions.append(Question(
                qid=f"{conv_id}:{qid}",
                question=qa.get("question", ""),
                gold_answer=qa.get("answer", ""),
                gold_passage_ids=gold_pids,
                category=conv.get("category", qa.get("category", "general")),
            ))

    return Dataset(name="locomo", questions=questions, passages=passages, version=_sha256_file(path)[:12])


def load_longmemeval() -> Dataset:
    """Load LongMemEval dataset from local JSON."""
    path = DATASETS_DIR / "longmemeval" / "longmemeval_s_oracle.json"
    if not path.exists():
        sys.exit(f"LongMemEval not downloaded. Run: python3 {__file__} download")

    raw = json.loads(path.read_text())
    questions: list[Question] = []
    passages: dict[str, str] = {}

    # LongMemEval format: list of {id, question, answer, evidence, conversation}
    for item in raw:
        item_id = str(item.get("id", len(questions)))
        conv = item.get("conversation", [])
        for turn in conv:
            pid = f"{item_id}#{turn.get('turn_id', turn.get('id', 0))}"
            passages[pid] = turn.get("content", turn.get("utterance", ""))

        gold_pids = []
        for ev in item.get("evidence", item.get("gold_passages", [])):
            if isinstance(ev, str):
                gold_pids.append(ev)
            elif isinstance(ev, dict):
                gold_pids.append(f"{item_id}#{ev.get('turn_id', ev.get('id', 0))}")

        questions.append(Question(
            qid=f"lme:{item_id}",
            question=item.get("question", ""),
            gold_answer=item.get("answer", ""),
            gold_passage_ids=gold_pids,
            category=item.get("category", item.get("task_type", "general")),
        ))

    return Dataset(name="longmemeval", questions=questions, passages=passages, version=_sha256_file(path)[:12])


# ─── Synapse ingestion ────────────────────────────────────────────────────

def _run(cmd: list[str], check: bool = True) -> subprocess.CompletedProcess:
    return subprocess.run(cmd, check=check, capture_output=True, text=True)


def ingest_dataset(ds: Dataset, db: Path) -> int:
    """Ingest passages into Synapse brain. Returns number of docs ingested."""
    # Reset brain
    if db.exists():
        db.unlink()

    _run([SYNX_BIN, "init", "-f", str(db)])
    _run([ULTRA_BIN, "init", "--db", str(db)])

    # Write passages as JSONL for bulk ingest
    jsonl_path = RESULTS_DIR / f"ingest-{ds.name}.jsonl"
    jsonl_path.parent.mkdir(parents=True, exist_ok=True)
    with open(jsonl_path, "w") as f:
        for pid, text in ds.passages.items():
            event = {
                "ts": int(time.time()),
                "agent": "eval",
                "kind": "passage",
                "uri": pid,
                "content": text,
                "session": "eval-ingest",
            }
            f.write(json.dumps(event) + "\n")

    # Ingest via synapse-ultra
    result = _run([ULTRA_BIN, "ingest", "--db", str(db), "--jsonl", str(jsonl_path)])
    print(result.stdout.strip())

    # Also put into synapse-core docs (for hybrid search)
    for pid, text in ds.passages.items():
        _run([SYNX_BIN, "put", "-f", str(db), "--uri", pid, "--text", text],
             check=False)

    return len(ds.passages)


# ─── Retrieval & evaluation ───────────────────────────────────────────────

def retrieve(db: Path, query: str, k: int) -> tuple[list[str], float]:
    """Run hybrid retrieval and return (uris, latency_ms)."""
    t0 = time.perf_counter()
    result = _run([SYNX_BIN, "hybrid", "-f", str(db), "--limit", str(k), query])
    latency_ms = (time.perf_counter() - t0) * 1000.0

    uris: list[str] = []
    for line in result.stdout.strip().splitlines():
        # Expected format: "rank\tscore\turi\tpreview"
        parts = line.split("\t")
        if len(parts) >= 3:
            uris.append(parts[2])
    return uris, latency_ms


def evaluate(ds: Dataset, db: Path, k: int) -> RunReport:
    """Run retrieval for every question and compute metrics."""
    results: list[RetrievalResult] = []
    per_category: dict[str, list[RetrievalResult]] = {}

    for i, q in enumerate(ds.questions):
        if i % 50 == 0:
            print(f"  [{ds.name}] {i}/{len(ds.questions)}")

        uris, lat = retrieve(db, q.question, k)
        hit = any(g in uris[:k] for g in q.gold_passage_ids)
        rank = 0
        for g in q.gold_passage_ids:
            if g in uris:
                rank = uris.index(g) + 1
                break

        r = RetrievalResult(
            qid=q.qid,
            retrieved_ids=uris[:k],
            latency_ms=lat,
            hit=hit,
            rank=rank,
        )
        results.append(r)
        per_category.setdefault(q.category, []).append(r)

    # Aggregate
    recall = sum(1 for r in results if r.hit) / max(len(results), 1)
    mrr = sum(1.0 / r.rank for r in results if r.rank > 0) / max(len(results), 1)
    lats = sorted(r.latency_ms for r in results)
    p50 = lats[len(lats) // 2] if lats else 0.0
    p95 = lats[int(len(lats) * 0.95)] if lats else 0.0

    per_cat_summary: dict[str, dict[str, float]] = {}
    for cat, rs in per_category.items():
        per_cat_summary[cat] = {
            "count": len(rs),
            "recall_at_k": sum(1 for r in rs if r.hit) / len(rs),
            "mrr": sum(1.0 / r.rank for r in rs if r.rank > 0) / len(rs),
            "latency_p50_ms": sorted(r.latency_ms for r in rs)[len(rs) // 2],
        }

    git_sha = _git_sha()

    return RunReport(
        dataset=ds.name,
        git_sha=git_sha,
        k=k,
        total_questions=len(results),
        recall_at_k=recall,
        mrr=mrr,
        latency_p50_ms=p50,
        latency_p95_ms=p95,
        per_category=per_cat_summary,
        results=results,
    )


def _git_sha() -> str:
    try:
        r = subprocess.run(["git", "rev-parse", "--short", "HEAD"],
                           cwd=EVAL_DIR.parent, capture_output=True, text=True)
        return r.stdout.strip() or "unknown"
    except Exception:
        return "unknown"


# ─── Reporting ────────────────────────────────────────────────────────────

def save_report(report: RunReport) -> Path:
    RESULTS_DIR.mkdir(parents=True, exist_ok=True)
    path = RESULTS_DIR / f"{report.dataset}-k{report.k}-{int(time.time())}.json"
    data = asdict(report)
    path.write_text(json.dumps(data, indent=2))
    return path


def render_latest_report() -> str:
    """Render a Markdown report from the latest results."""
    REPORTS_DIR.mkdir(parents=True, exist_ok=True)
    md = ["# Synapse Eval — Latest Results\n"]
    md.append(f"_Generated: {time.strftime('%Y-%m-%d %H:%M:%S')}_\n")

    for ds_name in ["locomo", "longmemeval"]:
        files = sorted(RESULTS_DIR.glob(f"{ds_name}-k*.json"), key=lambda p: p.stat().st_mtime)
        if not files:
            md.append(f"## {ds_name}\n\n_No results yet._\n")
            continue
        data = json.loads(files[-1].read_text())
        md.append(f"## {ds_name} (k={data['k']}, n={data['total_questions']})\n")
        md.append(f"- **Recall@{data['k']}**: {data['recall_at_k']:.4f}")
        md.append(f"- **MRR**: {data['mrr']:.4f}")
        md.append(f"- **Latency p50**: {data['latency_p50_ms']:.2f} ms")
        md.append(f"- **Latency p95**: {data['latency_p95_ms']:.2f} ms")
        md.append(f"- **Git SHA**: `{data['git_sha']}`\n")
        if data.get("per_category"):
            md.append("### Per-category\n")
            md.append("| Category | n | Recall@k | MRR | p50 ms |")
            md.append("|---|---|---|---|---|")
            for cat, m in sorted(data["per_category"].items()):
                md.append(f"| {cat} | {m['count']} | {m['recall_at_k']:.4f} | {m['mrr']:.4f} | {m['latency_p50_ms']:.2f} |")
            md.append("")
        md.append("")

    out = REPORTS_DIR / "latest.md"
    out.write_text("\n".join(md))
    return out.read_text()


# ─── CLI ──────────────────────────────────────────────────────────────────

def main() -> int:
    parser = argparse.ArgumentParser(description="Synapse eval harness")
    sub = parser.add_subparsers(dest="cmd", required=True)

    sub.add_parser("download", help="Download LoCoMo + LongMemEval datasets")

    p_ingest = sub.add_parser("ingest", help="Ingest dataset into a Synapse brain")
    p_ingest.add_argument("--db", type=Path, required=True)
    p_ingest.add_argument("--dataset", choices=["locomo", "longmemeval", "both"], default="both")

    p_run = sub.add_parser("run", help="Run benchmark")
    p_run.add_argument("--db", type=Path, required=True)
    p_run.add_argument("--dataset", choices=["locomo", "longmemeval", "both"], default="both")
    p_run.add_argument("--k", type=int, default=5)

    sub.add_parser("report", help="Render latest results as Markdown")

    args = parser.parse_args()

    if args.cmd == "download":
        download_datasets()
        return 0

    if args.cmd == "ingest":
        datasets = []
        if args.dataset in ("locomo", "both"):
            datasets.append(load_locomo())
        if args.dataset in ("longmemeval", "both"):
            datasets.append(load_longmemeval())
        for ds in datasets:
            n = ingest_dataset(ds, args.db)
            print(f"[ingest] {ds.name}: {n} passages → {args.db}")
        return 0

    if args.cmd == "run":
        datasets = []
        if args.dataset in ("locomo", "both"):
            datasets.append(load_locomo())
        if args.dataset in ("longmemeval", "both"):
            datasets.append(load_longmemeval())
        for ds in datasets:
            print(f"\n=== Running {ds.name} (k={args.k}) ===")
            report = evaluate(ds, args.db, args.k)
            path = save_report(report)
            print(f"\nResults: {path}")
            print(f"  Recall@{args.k}: {report.recall_at_k:.4f}")
            print(f"  MRR:          {report.mrr:.4f}")
            print(f"  Latency p50:  {report.latency_p50_ms:.2f} ms")
            print(f"  Latency p95:  {report.latency_p95_ms:.2f} ms")
        return 0

    if args.cmd == "report":
        print(render_latest_report())
        return 0

    return 1


if __name__ == "__main__":
    sys.exit(main())
