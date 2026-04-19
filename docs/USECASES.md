# Top 20 Use-Cases for Synapse

Each entry: **what it solves**, **why Synapse**, **how to apply in a Supersynergy project**, and a minimal example.

---

## 1. Per-Project Claude Code Memory

Git-committable `.claude/brain.brainpack`. Agent recalls past decisions across sessions and teammates.

**Apply (EventsHub, SupersynergyCRM, synergine-app):**
```bash
./target/release/synapsed -f .claude/brain.db &
# hook PostToolUse/Stop → synapse put "context snapshot"
git add .claude/brain.brainpack
```

## 2. Competitor / Docs Crawl → Offline Searchable KB

Crawl `stripe.com/docs` once, `synapse put --batch`, commit the `.brainpack`. Query offline forever.

**Apply (super-research pipeline):**
```bash
super-research "stripe subscription api" --count 50 | synapse put-batch --embed
synapse snap stripe-docs.brainpack
```

## 3. Lead Database Hybrid Search (2.39M German companies)

Current: `master_enriched` SQLite (878 MB). Add vector-column → find "similar companies to X" by description.

**Apply (leads-dashboard):**
- Import existing SQLite into Synapse schema.
- Run hybrid RRF on "company industry + tech stack" fields.
- Result: `rank(lex) + rank(vec)` = intent-aware lead search.

## 4. Session-Level LLM Chat History

One `.brainpack` per user session. `session-start` loads, `stop` writes. No session re-explaining context.

**Apply (any Claude-integrated tool):**
```ts
const brain = new Synapse(`/tmp/synapse.sock`);
await brain.put({ text: `user:${msg}\nassistant:${resp}`, embed: true });
// on next turn:
const ctx = await brain.search(currentMsg, { mode: "Hybrid", embedQuery: true });
```

## 5. Agent Tool Output Memory

Every tool call's output → Synapse. Agent queries own history before re-running expensive tools.

**Apply (Claude Code hooks):**
- `PostToolUse` hook → `synapse put` stdout/stderr digest.
- Before tool call, `synapse search` for prior identical invocation → cache hit saves minutes.

## 6. Error / Log Deduplication Store

Dump stack traces into Synapse. BLAKE3-dedup collapses duplicates. FTS5 finds "similar to this crash."

**Apply (SupersynergyCRM observability):**
```bash
tail -f app.log | synapse put --stdin --title "err-$(date +%s)"
# dashboard: synapse search "DB connection" --mode Hybrid
```

## 7. Cold-Email Personalization Memory

Per-prospect brain: past emails, company facts, enrichment data. Next outbound loop pulls top-k context.

**Apply (outreach-engine):**
```bash
synapse put -f leads.brainpack --title "acme-corp" --text "$(cat acme-research.md)" --embed
```

## 8. Research Report Digest Archive

Every `super-research` run writes a `.brainpack`. Search all past research before spawning a new one.

**Apply:**
```bash
super-research "topic" | synapse put-batch --embed
synapse snap ~/research.brainpack
# next time: synapse search "topic" first → skip if hit
```

## 9. Knowledge Base for Sales / Onboarding

Internal FAQ → one file. New hire cheats: `synapse search "how do we deploy to staging?"`.

**Apply (new hires, KMU customers):**
- Markdown export of Notion/Obsidian into Synapse.
- Share `.brainpack` via Slack.
- Claude Code plugin surfaces answers inline.

## 10. Domain-Specific RAG (BFSG / DSGVO Compliance)

Ingest BFSG/DSGVO text + commentary. Every web project loads the same `bfsg.brainpack` → Claude cites actual clauses.

**Apply (premium-web projects):**
```bash
cat BFSG-2025.md DSGVO-commentary.md | synapse put-batch --embed
synapse snap ~/compliance/bfsg.brainpack
# in project: ln -s ~/compliance/bfsg.brainpack .claude/bfsg.brainpack
```

## 11. Design System Memory

Extract design tokens + components once (via `design-memory`), store in Synapse. Query: "show me buttons like X."

**Apply (10-theme template-library):**
- Each theme = one `.brainpack`.
- Cross-theme search: "find dashboards with dark mode + radix".

## 12. Code Search / Semantic Grep

Index repo into Synapse. `synapse search "handle auth refresh token"` returns semantically-near functions beyond `grep`.

**Apply (ZeroClawUltimate, SupersynergyCRM):**
```bash
find . -name "*.ts" -exec synapse put --title {} --text "$(cat {})" --embed \;
```

## 13. CRM Contact & Interaction Store

Per-contact `.brainpack`: every call, email, note, dealing. FTS5 BM25 + vec for "who told us about X?"

**Apply (openclaw-crm, SupersynergyCRM):**
- Write adapter: CRM row → Synapse put.
- Reports: `synapse search "enterprise deal" --mode Hybrid --limit 20`.

## 14. Scraped Product Catalog Memory

Scrape 10k products → Synapse. BLAKE3 dedup collapses duplicates across sources. FTS5 + vec = "alternatives to product X."

**Apply (omni-scraper, stealth_scraper_matrix):**
```bash
scrape-cli amazon.de | synapse put-batch --embed
```

## 15. Model-Evaluation Trace Store

Every LLM response → Synapse. Rerun same prompt → query for prior runs → A/B regression detection.

**Apply (memvidbench-style ML evals):**
```bash
for model in claude-4-7 claude-4-6 haiku-4-5; do
  respond "$PROMPT" --model "$model" | synapse put --title "$model-$PROMPT_ID" --embed
done
synapse search "$PROMPT_ID" → compare
```

## 16. MCP Memory Endpoint for Any Agent

`synapse-mcp` is an MCP server. Claude / any MCP client → tool-calls `put` / `search`.

**Apply (CC globally):**
```json
// ~/.claude/settings.json mcpServers section
"synapse": { "command": "synapse-mcp", "args": ["--sock", "/tmp/synapse.sock"] }
```

## 17. Offline Wiki / Reference Bundle

Dump MDN / Rust docs / PostgreSQL manual once → ship `.brainpack`. Dev boxes have zero-bandwidth offline docs.

**Apply (portable-dev laptops, air-gapped):**
```bash
monolith https://doc.rust-lang.org/std/index.html | synapse put-batch
synapse snap rust-std.brainpack
```

## 18. Support-Ticket Similarity / Triage

New ticket → vec-search past tickets → show top-5 + resolution. Humans close 60 % faster.

**Apply (customer support internal):**
```bash
synapse put --text "$new_ticket" --embed
synapse search "$new_ticket" --mode Vec --limit 5
```

## 19. Screenshot Memory (text extracted via OCR)

Per-screenshot `.brainpack`. OCR → Synapse. "Find that error from last Tuesday" works.

**Apply (M4 Max, ~/Desktop clutter):**
- Hook on screenshot → OCR → synapse put.
- Query: `synapse search "auth error login" --mode Hybrid`.

## 20. Domain Data Packs as Products

Sell premade `.brainpack` files: DSGVO commentary pack, BFSG pack, German-tax-code pack, E-Commerce SKU pack, DACH-lead pack.

**Apply (Supersynergy revenue):**
- "Done-for-you compliance pack" → one file → drop into any Claude Code project.
- Price point: 99 €/pack/yr refresh.

---

## Integration Pattern: Synapse Daemon per User

Run one daemon per user on login. All projects talk to it. Each project owns `.claude/brain.brainpack` for durable memory; ephemeral session memory = in-daemon only. `launchd` unit in repo.

```xml
<!-- ~/Library/LaunchAgents/com.supersynergy.synapsed.plist -->
<plist version="1.0"><dict>
  <key>Label</key><string>com.supersynergy.synapsed</string>
  <key>ProgramArguments</key>
    <array>
      <string>/usr/local/bin/synapsed</string>
      <string>-f</string><string>/Users/master/.synapse/brain.db</string>
      <string>-s</string><string>/tmp/synapse.sock</string>
      <string>--lazy-embed</string>
    </array>
  <key>RunAtLoad</key><true/>
  <key>KeepAlive</key><true/>
</dict></plist>
```
