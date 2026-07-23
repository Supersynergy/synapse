//! synapse-ultra CLI — lean observe + ingest + lake surface.
//!
//! Usage:
//!   synapse-ultra init [--db PATH]
//!   synapse-ultra ingest --db PATH --jsonl FILE
//!   synapse-ultra ingest --db PATH --json '{"agent":"claude","kind":"decision",...}'
//!   synapse-ultra inspect [--db PATH]
//!   synapse-ultra why --db PATH --uri URI [--depth N]
//!   synapse-ultra graph --db PATH --uri URI [--depth N] [--dot]
//!   synapse-ultra replay --db PATH --session ID [--limit N]
//!   synapse-ultra cost --db PATH [--since -7d] [--by agent|model|day]
//!   synapse-ultra events --db PATH [--agent X] [--kind Y] [--session S] [--limit N]
//!   synapse-ultra search --db PATH "fts5 query" [--limit N]
//!   synapse-ultra lake init   --db PATH [--catalog PATH]
//!   synapse-ultra lake archive --db PATH --older-than DAYS [--catalog PATH]
//!   synapse-ultra lake analytics --db PATH [--catalog PATH]
//!   synapse-ultra doctor [--db PATH]

use anyhow::Result;
use clap::{Parser, Subcommand};
use synapse_ultra::{Ultra, UltraResult};

#[derive(Parser)]
#[command(
    name = "synapse-ultra",
    version,
    about = "Synapse Ultra — event log, graph-v2 CTE, observe CLI for synapse-memory"
)]
struct Cli {
    /// Path to the brain.db file. Defaults to ~/.synapse/brain.db.
    #[arg(long, global = true, env = "SYNAPSE_DB")]
    db: Option<std::path::PathBuf>,

    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Initialize the Ultra schema (idempotent).
    Init,
    /// Ingest events from JSON or JSONL.
    Ingest {
        /// JSON string of a single Event.
        #[arg(long)]
        json: Option<String>,
        /// Path to a JSONL file (one Event per line).
        #[arg(long)]
        jsonl: Option<std::path::PathBuf>,
    },
    /// Ingest a decision (creates graph nodes + edges via trigger).
    Decision {
        /// Decision URI (the thing being decided).
        #[arg(long)]
        uri: String,
        /// Agent making the decision.
        #[arg(long)]
        agent: String,
        /// Optional rationale text.
        #[arg(long)]
        rationale: Option<String>,
        /// Source URI (what caused this decision).
        #[arg(long)]
        source: Option<String>,
        /// Target URI (what this decision produces/derives).
        #[arg(long)]
        target: Option<String>,
        /// Optional session id.
        #[arg(long)]
        session: Option<String>,
    },
    /// Show brain statistics.
    Inspect,
    /// Decision-chain: what caused this URI?
    Why {
        #[arg(long)]
        uri: String,
        #[arg(long, default_value_t = 5)]
        depth: i64,
    },
    /// Forward graph traversal: what does this URI lead to?
    Graph {
        #[arg(long)]
        uri: String,
        #[arg(long, default_value_t = 3)]
        depth: i64,
        /// Output as Graphviz DOT (pipe to `dot -Tsvg`).
        #[arg(long)]
        dot: bool,
    },
    /// Replay a session chronologically.
    Replay {
        #[arg(long)]
        session: String,
        #[arg(long, default_value_t = 1000)]
        limit: i64,
    },
    /// Token cost report.
    Cost {
        /// Days back to include (e.g. 7 = last 7 days). Default 7.
        #[arg(long, default_value_t = 7)]
        days: i64,
    },
    /// List events matching filters.
    Events {
        #[arg(long)]
        agent: Option<String>,
        #[arg(long)]
        kind: Option<String>,
        #[arg(long)]
        session: Option<String>,
        #[arg(long)]
        uri: Option<String>,
        #[arg(long, default_value_t = 100)]
        limit: i64,
    },
    /// Trace one agent across all sessions within a time range.
    Trace {
        #[arg(long)]
        agent: String,
        /// Days back (default 1 = last 24h).
        #[arg(long, default_value_t = 1)]
        days: i64,
        #[arg(long, default_value_t = 1000)]
        limit: i64,
    },
    /// Daily summary: what happened on a given day?
    Daily {
        /// Days back (0 = today, 1 = yesterday, ...). Default 0.
        #[arg(long, default_value_t = 0)]
        days_back: i64,
    },
    /// Session timeline: chronological events + decisions for one session.
    Timeline {
        #[arg(long)]
        session: String,
        #[arg(long, default_value_t = 1000)]
        limit: i64,
    },
    /// List all sessions with event/decision counts and cost.
    Sessions {
        #[arg(long)]
        agent: Option<String>,
        #[arg(long, default_value_t = 50)]
        limit: i64,
    },
    /// Full-text search over event content via FTS5.
    Search {
        /// FTS5 query (e.g. 'refactor', 'error timeout', '"exact phrase"', 'refactor* OR test*').
        query: String,
        #[arg(long, default_value_t = 50)]
        limit: i64,
    },
    /// Optional DuckLake archive operations.
    #[command(subcommand)]
    Lake(LakeCmd),
    /// Health check.
    Doctor,
}

#[derive(Subcommand)]
enum LakeCmd {
    /// Initialize the DuckLake catalog.
    Init {
        #[arg(long, default_value = "metadata.ducklake")]
        catalog: std::path::PathBuf,
    },
    /// Archive events older than N days to DuckLake.
    Archive {
        #[arg(long)]
        older_than: i64,
        #[arg(long, default_value = "metadata.ducklake")]
        catalog: std::path::PathBuf,
    },
    /// Start an interactive DuckDB analytics shell.
    Analytics {
        #[arg(long, default_value = "metadata.ducklake")]
        catalog: std::path::PathBuf,
    },
}

fn default_db() -> std::path::PathBuf {
    dirs_next::home_dir()
        .map(|h| h.join(".synapse").join("brain.db"))
        .unwrap_or_else(|| std::path::PathBuf::from("brain.db"))
}

fn open_ultra(db: &std::path::Path) -> UltraResult<Ultra> {
    if let Some(parent) = db.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).ok();
        }
    }
    Ultra::open(db)
}

fn run() -> Result<()> {
    let cli = Cli::parse();
    let db = cli.db.clone().unwrap_or_else(default_db);
    let ultra = open_ultra(&db)?;

    match cli.cmd {
        Cmd::Init => {
            ultra.migrate()?;
            println!("ultra: schema migrated at {}", db.display());
        }
        Cmd::Ingest { json, jsonl } => {
            ultra.migrate()?;
            let count = ultra.with_conn(|c| {
                let mut n = 0;
                if let Some(j) = json {
                    synapse_ultra::events::ingest_event_json(c, &j)?;
                    n += 1;
                }
                if let Some(p) = jsonl {
                    n += synapse_ultra::events::ingest_jsonl_file(c, &p)?;
                }
                UltraResult::Ok(n)
            })?;
            println!("ultra: ingested {count} events");
        }
        Cmd::Decision { uri, agent, rationale, source, target, session } => {
            ultra.migrate()?;
            let ts = chrono::Utc::now().timestamp();
            let id = ultra.with_conn(|c| {
                synapse_ultra::events::ingest_decision(
                    c,
                    ts,
                    session.as_deref(),
                    &agent,
                    &uri,
                    rationale.as_deref(),
                    source.as_deref(),
                    target.as_deref(),
                    None,
                )
            })?;
            println!("ultra: decision {id} ingested (uri={uri})");
            if source.is_some() || target.is_some() {
                println!("  graph nodes + edges auto-populated by trigger");
            }
        }
        Cmd::Inspect => {
            ultra.migrate()?;
            let stats = ultra.with_conn(synapse_ultra::observe::brain_stats)?;
            println!("Synapse Ultra — brain stats ({})", db.display());
            println!("  docs:              {}", stats.docs);
            println!("  events:            {}", stats.events);
            println!("  decisions:         {}", stats.decisions);
            println!("  graph_nodes:       {}", stats.graph_nodes);
            println!("  graph_edges:       {}", stats.graph_edges);
            println!("  sessions:          {}", stats.sessions);
            println!("  token_cost rows:   {}", stats.token_cost_rows);
            println!("  total cost USD:    ${:.4}", stats.total_cost_usd);
            println!(
                "  total tokens:      {} in / {} out",
                stats.total_input_tokens, stats.total_output_tokens
            );
            println!("  db size:           {} bytes", stats.db_size_bytes);
            println!("  ultra schema:      v{}", stats.ultra_schema_version);
            let agents = ultra.with_conn(|c| synapse_ultra::observe::top_agents(c, 5))?;
            if !agents.is_empty() {
                println!("  top agents:");
                for (a, c) in agents {
                    println!("    {a}: {c}");
                }
            }
            let kinds = ultra.with_conn(|c| synapse_ultra::observe::top_kinds(c, 5))?;
            if !kinds.is_empty() {
                println!("  top kinds:");
                for (k, c) in kinds {
                    println!("    {k}: {c}");
                }
            }
        }
        Cmd::Why { uri, depth } => {
            ultra.migrate()?;
            let steps = ultra.with_conn(|c| synapse_ultra::graph::why(c, &uri, depth))?;
            if steps.is_empty() {
                println!("ultra: no chain found for {uri}");
            } else {
                for s in steps {
                    println!(
                        "[d{}] {} ({})  path: {}",
                        s.depth, s.uri, s.kind, s.path
                    );
                }
            }
        }
        Cmd::Graph { uri, depth, dot } => {
            ultra.migrate()?;
            if dot {
                let dot_str = ultra.with_conn(|c| synapse_ultra::graph::to_dot(c, &uri, depth))?;
                print!("{dot_str}");
            } else {
                let steps = ultra.with_conn(|c| synapse_ultra::graph::graph_expand(c, &uri, depth))?;
                if steps.is_empty() {
                    println!("ultra: no forward chain found for {uri}");
                } else {
                    for s in steps {
                        println!("[d{}] {}  path: {}", s.depth, s.uri, s.path);
                    }
                }
            }
        }
        Cmd::Replay { session, limit } => {
            ultra.migrate()?;
            let entries = ultra.with_conn(|c| synapse_ultra::observe::replay(c, &session, limit))?;
            if entries.is_empty() {
                println!("ultra: no events for session {session}");
            } else {
                for e in entries {
                    let uri = e.uri.unwrap_or_default();
                    let content = e.content.unwrap_or_default();
                    let content_preview = if content.len() > 120 {
                        format!("{}…", &content[..120])
                    } else {
                        content
                    };
                    println!("[{}] {} uri={} {}", e.ts, e.kind, uri, content_preview);
                }
            }
        }
        Cmd::Cost { days } => {
            ultra.migrate()?;
            let since = chrono::Utc::now().timestamp() - days * 86400;
            let rows = ultra.with_conn(|c| synapse_ultra::observe::cost_by_day(c, since))?;
            if rows.is_empty() {
                println!("ultra: no token_cost rows in the last {days} days");
            } else {
                println!("{:<12} {:<14} {:<28} {:>6} {:>10} {:>10} {:>10}",
                    "day", "agent", "model", "calls", "in_tok", "out_tok", "usd");
                for r in rows {
                    println!("{:<12} {:<14} {:<28} {:>6} {:>10} {:>10} {:>10.4}",
                        r.bucket, r.agent, r.model, r.calls, r.input_tokens, r.output_tokens, r.cost_usd);
                }
            }
        }
        Cmd::Events { agent, kind, session, uri, limit } => {
            ultra.migrate()?;
            let mut filter = synapse_ultra::EventFilter::new().limit(limit);
            if let Some(a) = agent { filter = filter.agent(a); }
            if let Some(k) = kind { filter = filter.kind(k); }
            if let Some(s) = session { filter = filter.session(s); }
            if let Some(u) = uri { filter = filter.uri(u); }
            let rows = ultra.with_conn(|c| synapse_ultra::events::query_events(c, &filter))?;
            if rows.is_empty() {
                println!("ultra: no events match");
            } else {
                for r in rows {
                    let content = r.content.unwrap_or_default();
                    let preview = if content.len() > 100 {
                        format!("{}…", &content[..100])
                    } else {
                        content
                    };
                    println!("[{}] {} {} uri={} {}", r.ts, r.agent, r.kind, r.uri.unwrap_or_default(), preview);
                }
            }
        }
        Cmd::Lake(lc) => {
            match lc {
                LakeCmd::Init { catalog } => {
                    let cfg = synapse_ultra::lake::LakeConfig {
                        catalog_path: catalog.clone(),
                        data_dir: catalog.parent().unwrap_or(std::path::Path::new(".")).join("lake-data"),
                    };
                    synapse_ultra::lake::init(&cfg)?;
                    println!("ultra: ducklake catalog initialized at {}", catalog.display());
                }
                LakeCmd::Archive { older_than, catalog } => {
                    let cfg = synapse_ultra::lake::LakeConfig {
                        catalog_path: catalog.clone(),
                        data_dir: catalog.parent().unwrap_or(std::path::Path::new(".")).join("lake-data"),
                    };
                    let cutoff = chrono::Utc::now().timestamp() - older_than * 86400;
                    let n = synapse_ultra::lake::archive(&db, &cfg, cutoff)?;
                    println!("ultra: archived {n} events older than {older_than} days");
                }
                LakeCmd::Analytics { catalog } => {
                    let cfg = synapse_ultra::lake::LakeConfig {
                        catalog_path: catalog.clone(),
                        data_dir: catalog.parent().unwrap_or(std::path::Path::new(".")).join("lake-data"),
                    };
                    synapse_ultra::lake::analytics_shell(&db, &cfg)?;
                }
            }
        }
        Cmd::Doctor => {
            ultra.migrate()?;
            let stats = ultra.with_conn(synapse_ultra::observe::brain_stats)?;
            println!("ultra: doctor — OK");
            println!("  db: {} ({} bytes)", db.display(), stats.db_size_bytes);
            println!("  ultra schema: v{}", stats.ultra_schema_version);
            println!("  duckdb CLI: {}", if synapse_ultra::lake::duckdb_available() { "available" } else { "NOT found (optional)" });
        }
        Cmd::Trace { agent, days, limit } => {
            ultra.migrate()?;
            let now = chrono::Utc::now().timestamp();
            let since = now - days * 86400;
            let rows = ultra.with_conn(|c| {
                synapse_ultra::observe::agent_trace(c, &agent, since, now, limit)
            })?;
            if rows.is_empty() {
                println!("ultra: trace — no events for agent '{agent}' in last {days}d");
                return Ok(());
            }
            println!("# agent trace: {agent} (last {days}d, {} events)", rows.len());
            for r in rows {
                let sess = r.session_id.as_deref().unwrap_or("-");
                let uri = r.uri.as_deref().unwrap_or("-");
                let preview = r.content_preview.as_deref().unwrap_or("");
                let preview = if preview.len() > 80 { &preview[..80] } else { preview };
                println!("{}\t{}\t{}\t{}\t{}", r.ts, sess, r.kind, uri, preview);
            }
        }
        Cmd::Daily { days_back } => {
            ultra.migrate()?;
            let now = chrono::Utc::now().timestamp();
            let day_end = now - days_back * 86400;
            let day_start = day_end - 86400;
            let s = ultra.with_conn(|c| {
                synapse_ultra::observe::daily_summary(c, day_start, day_end)
            })?;
            println!("# daily summary — day -{}d  [{} .. {}]", days_back, day_start, day_end);
            println!("events:       {}", s.total_events);
            println!("decisions:    {}", s.total_decisions);
            println!("sessions:     {}", s.total_sessions);
            println!("cost_usd:     {:.4}", s.total_cost_usd);
            println!("tokens:       in={} out={}", s.total_input_tokens, s.total_output_tokens);
            println!("graph_growth: +{} nodes +{} edges", s.new_graph_nodes, s.new_graph_edges);
            println!();
            println!("## agents ({}):", s.agents.len());
            for a in &s.agents {
                println!("  {}:", a.agent);
                println!("    events={} decisions={} sessions={} cost=${:.4}", a.events, a.decisions, a.sessions, a.cost_usd);
                println!("    tokens: in={} out={}", a.input_tokens, a.output_tokens);
                println!("    first_ts={} last_ts={}", a.first_ts, a.last_ts);
                if !a.top_kinds.is_empty() {
                    let kinds = a.top_kinds.iter().map(|(k, c)| format!("{k}={c}")).collect::<Vec<_>>().join(" ");
                    println!("    top_kinds: {kinds}");
                }
                if !a.top_uris.is_empty() {
                    let uris = a.top_uris.iter().map(|(u, c)| format!("{u}={c}")).collect::<Vec<_>>().join(" ");
                    println!("    top_uris:  {uris}");
                }
            }
            if !s.top_decisions.is_empty() {
                println!();
                println!("## top decisions ({}):", s.top_decisions.len());
                for d in &s.top_decisions {
                    let rat = d.rationale.as_deref().unwrap_or("-");
                    let rat = if rat.len() > 80 { &rat[..80] } else { rat };
                    println!("  {}d#{}  agent={}  uri={}", d.ts, d.id, d.agent, d.uri);
                    if let Some(src) = &d.source_uri { println!("    source: {src}"); }
                    if let Some(tgt) = &d.target_uri { println!("    target: {tgt}"); }
                    println!("    rationale: {rat}");
                }
            }
        }
        Cmd::Timeline { session, limit } => {
            ultra.migrate()?;
            let rows = ultra.with_conn(|c| {
                synapse_ultra::observe::session_timeline(c, &session, limit)
            })?;
            if rows.is_empty() {
                println!("ultra: timeline — no events for session '{session}'");
                return Ok(());
            }
            println!("# session timeline: {session} ({} rows)", rows.len());
            for r in rows {
                let marker = if r.is_decision { "DECISION" } else { "event" };
                let uri = r.uri.as_deref().unwrap_or("-");
                let preview = r.content_preview.as_deref().unwrap_or("");
                let preview = if preview.len() > 80 { &preview[..80] } else { preview };
                println!("{}\t{}\t{}\t{}\t{}\t{}", r.ts, marker, r.agent, r.kind, uri, preview);
            }
        }
        Cmd::Sessions { agent, limit } => {
            ultra.migrate()?;
            let rows = ultra.with_conn(|c| {
                synapse_ultra::observe::list_sessions(c, agent.as_deref(), limit)
            })?;
            if rows.is_empty() {
                println!("ultra: sessions — no sessions found");
                return Ok(());
            }
            println!("# sessions ({}):", rows.len());
            println!("session_id\tagent\tevents\tdecisions\tfirst_ts\tlast_ts\tcost_usd");
            for r in rows {
                println!("{}\t{}\t{}\t{}\t{}\t{}\t{:.4}", r.session_id, r.agent, r.events, r.decisions, r.first_ts, r.last_ts, r.cost_usd);
            }
        }
        Cmd::Search { query, limit } => {
            ultra.migrate()?;
            let rows = ultra.with_conn(|c| {
                synapse_ultra::events::search_events(c, &query, Some(limit))
            })?;
            if rows.is_empty() {
                println!("ultra: search — no hits for '{query}'");
                return Ok(());
            }
            println!("# ultra search: '{query}' ({} hits)", rows.len());
            for r in rows {
                let content = r.content.unwrap_or_default();
                let preview = if content.len() > 100 {
                    format!("{}…", &content[..100])
                } else {
                    content
                };
                println!("[{}] {} {} uri={} {}", r.ts, r.agent, r.kind, r.uri.unwrap_or_default(), preview);
            }
        }
    }
    Ok(())
}

fn main() -> Result<()> {
    run()
}
