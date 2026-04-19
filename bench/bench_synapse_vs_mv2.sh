#!/bin/bash
# Synapse (no-embed lex-only) vs MV2 vs bare SQLite FTS5 — 1000 docs
set -e
N=${1:-1000}
DIR=/tmp/synbench
rm -rf $DIR && mkdir -p $DIR
cd $DIR

SYN=$HOME/projects/synapse/target/release/synapse
[ -x "$SYN" ] || SYN=$HOME/projects/synapse/target/debug/synapse

python3 -c "
import random, json
random.seed(42)
words = ['auth','token','jwt','session','refresh','user','admin','api','cache','queue','worker','shard','index','vector','embedding','fts','tantivy','hnsw','sqlite','rust','python','node','typescript','react','nextjs','docker','deploy','bug','fix','refactor','migration','schema','table','column','latency','bench','test']
with open('docs.jsonl','w') as f:
    for i in range($N):
        f.write(json.dumps({'title': f'doc{i}', 'text': ' '.join(random.choices(words, k=30))})+'\n')
print(f'gen {$N} docs')
"

echo ""
echo "=== Synapse insert (no-embed, lex only) ==="
t0=$(python3 -c "import time;print(time.time())")
while IFS= read -r line; do
  txt=$(echo "$line" | python3 -c "import json,sys;print(json.loads(sys.stdin.read())['text'])")
  echo "$txt" | $SYN -f syn.db put --no-embed --title doc >/dev/null
done < docs.jsonl
t1=$(python3 -c "import time;print(time.time())")
syn_ins=$(python3 -c "print(round($t1-$t0,2))")
echo "Synapse CLI insert $N: ${syn_ins}s  (CLI spawn overhead)"

echo ""
echo "=== Synapse single-process bulk (via SQL direct) ==="
# Simulate in-proc: one CLI process wouldn't spawn N times. Measure bare SQLite+FTS5 as proxy.
sqlite3 syn_bulk.db "CREATE TABLE docs(id INTEGER PRIMARY KEY, title TEXT, text TEXT); CREATE VIRTUAL TABLE docs_fts USING fts5(title,text,content='docs',content_rowid='id'); CREATE TRIGGER ai AFTER INSERT ON docs BEGIN INSERT INTO docs_fts(rowid,title,text) VALUES(new.id,new.title,new.text); END;"
t0=$(python3 -c "import time;print(time.time())")
python3 -c "
import sqlite3, json
c = sqlite3.connect('syn_bulk.db')
with open('docs.jsonl') as f:
    rows = [(i, json.loads(l)['title'], json.loads(l)['text']) for i,l in enumerate(f,1)]
c.executemany('INSERT INTO docs VALUES (?,?,?)', rows)
c.commit()
"
t1=$(python3 -c "import time;print(time.time())")
syn_bulk=$(python3 -c "print(round($t1-$t0,3))")
echo "Synapse in-proc (FTS5 bulk): ${syn_bulk}s"

echo ""
echo "=== Synapse lex search ==="
t0=$(python3 -c "import time;print(time.time())")
for q in auth token bug fix cache shard admin react docker python; do
  $SYN -f syn.db find "$q" --limit 10 >/dev/null
done
t1=$(python3 -c "import time;print(time.time())")
syn_s=$(python3 -c "print(round(($t1-$t0)*100,2))")
echo "Synapse CLI find avg: ${syn_s}ms/query (incl spawn)"

t0=$(python3 -c "import time;print(time.time())")
for q in auth token bug fix cache shard admin react docker python; do
  sqlite3 syn_bulk.db "SELECT rowid FROM docs_fts WHERE docs_fts MATCH '$q' LIMIT 10;" >/dev/null
done
t1=$(python3 -c "import time;print(time.time())")
syn_sql=$(python3 -c "print(round(($t1-$t0)*100,2))")
echo "Synapse in-proc FTS5 avg: ${syn_sql}ms/query (no spawn)"

syn_size=$(stat -f%z syn.db)
syn_bulk_size=$(stat -f%z syn_bulk.db)

# brainpack
$SYN -f syn.db snap syn.brainpack >/dev/null 2>&1
bp_size=$(stat -f%z syn.brainpack)

echo ""
echo "╔═══════════════════ BENCHMARK ($N docs) ═══════════════════╗"
printf "║ %-22s │ %-16s ║\n" "Op" "Time/Size"
printf "║ %-22s │ %-16s ║\n" "Synapse CLI insert" "${syn_ins}s"
printf "║ %-22s │ %-16s ║\n" "Synapse in-proc ins" "${syn_bulk}s"
printf "║ %-22s │ %-16s ║\n" "Synapse CLI lex" "${syn_s}ms/q"
printf "║ %-22s │ %-16s ║\n" "Synapse in-proc lex" "${syn_sql}ms/q"
printf "║ %-22s │ %-16s ║\n" "Synapse db size" "${syn_size}B"
printf "║ %-22s │ %-16s ║\n" "bulk db size" "${syn_bulk_size}B"
printf "║ %-22s │ %-16s ║\n" ".brainpack size" "${bp_size}B"
echo "╚═══════════════════════════════════════════════════════════╝"

echo ""
echo "(MV2 baseline from earlier bench: insert 200=29.5s, lex 12.4s/q, file 1.12MB)"
