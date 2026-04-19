#!/bin/bash
set -e
N=${1:-1000}
DIR=/tmp/mv2bench
rm -rf $DIR && mkdir -p $DIR
cd $DIR

python3 -c "
import random, json
random.seed(42)
words = ['auth','token','jwt','session','refresh','user','admin','api','cache','queue','worker','shard','index','vector','embedding','fts','tantivy','hnsw','sqlite','rust','python','node','typescript','react','nextjs','docker','deploy','bug','fix','refactor','migration','schema','table','column','latency','bench','test']
docs = [{'title': f'doc{i}', 'text': ' '.join(random.choices(words, k=30))} for i in range($N)]
with open('docs.json','w') as f: json.dump(docs, f)
with open('docs.jsonl','w') as f:
    for d in docs: f.write(json.dumps(d)+'\n')
print(f'gen {len(docs)} docs')
"

echo ""
echo "=== MV2 batch insert ==="
memvid create mem.mv2 >/dev/null
t0=$(python3 -c "import time;print(time.time())")
memvid put-many mem.mv2 --input docs.json > /tmp/mv2_insert.log 2>&1
t1=$(python3 -c "import time;print(time.time())")
mv2_ins=$(python3 -c "print(round($t1-$t0,2))")
echo "MV2 put-many: ${mv2_ins}s"
tail -3 /tmp/mv2_insert.log

echo ""
echo "=== MV2 search (lex) ==="
t0=$(python3 -c "import time;print(time.time())")
for q in auth token bug fix cache shard admin react docker python; do
  memvid find mem.mv2 --query "$q" --limit 10 >/dev/null 2>&1
done
t1=$(python3 -c "import time;print(time.time())")
mv2_s=$(python3 -c "print(round(($t1-$t0)*100,2))")
echo "MV2 find avg: ${mv2_s}ms/query"

echo ""
echo "=== MV2 vec-search ==="
t0=$(python3 -c "import time;print(time.time())")
for q in "authentication bug" "cache layer" "rust deploy"; do
  memvid vec-search mem.mv2 --query "$q" --limit 10 >/dev/null 2>&1 || true
done
t1=$(python3 -c "import time;print(time.time())")
mv2_vs=$(python3 -c "print(round(($t1-$t0)*1000/3,2))")
echo "MV2 vec-search avg: ${mv2_vs}ms/query"

mv2_sz=$(stat -f%z mem.mv2)

echo ""
echo "=== SQLite FTS5 insert ==="
sqlite3 sq.db "CREATE VIRTUAL TABLE docs USING fts5(title, text);"
t0=$(python3 -c "import time;print(time.time())")
python3 -c "
import sqlite3, json
c = sqlite3.connect('sq.db')
with open('docs.jsonl') as f:
    rows = [(json.loads(l)['title'], json.loads(l)['text']) for l in f]
c.executemany('INSERT INTO docs VALUES (?,?)', rows)
c.commit()
"
t1=$(python3 -c "import time;print(time.time())")
sq_ins=$(python3 -c "print(round($t1-$t0,3))")
echo "SQLite FTS5 insert: ${sq_ins}s"

echo ""
echo "=== SQLite FTS5 search ==="
t0=$(python3 -c "import time;print(time.time())")
for q in auth token bug fix cache shard admin react docker python; do
  sqlite3 sq.db "SELECT title FROM docs WHERE docs MATCH '$q' LIMIT 10;" >/dev/null
done
t1=$(python3 -c "import time;print(time.time())")
sq_s=$(python3 -c "print(round(($t1-$t0)*100,2))")
echo "SQLite FTS5 avg: ${sq_s}ms/query"

sq_sz=$(stat -f%z sq.db)

echo ""
echo "╔═══════════════ BENCHMARK ($N docs) ═══════════════╗"
printf "║ %-18s │ %-12s │ %-12s ║\n" "Op" "MV2" "SQLite FTS5"
printf "║ %-18s │ %-12s │ %-12s ║\n" "insert (total)" "${mv2_ins}s" "${sq_ins}s"
printf "║ %-18s │ %-12s │ %-12s ║\n" "lex search avg" "${mv2_s}ms" "${sq_s}ms"
printf "║ %-18s │ %-12s │ %-12s ║\n" "vec search avg" "${mv2_vs}ms" "N/A"
printf "║ %-18s │ %-12s │ %-12s ║\n" "file size" "${mv2_sz}B" "${sq_sz}B"
echo "╚════════════════════════════════════════════════════╝"
