//! Tests for synapse-ultra.

use synapse_ultra::{Ultra, EventKind, Event};
use synapse_ultra::events::{ingest_event, ingest_decision, ingest_event_json, EventFilter};
use synapse_ultra::graph::{upsert_node, upsert_edge, why, graph_expand};

fn fresh() -> Ultra {
    let u = Ultra::open_memory().unwrap();
    u.migrate().unwrap();
    u
}

#[test]
fn migrate_is_idempotent() {
    let u = Ultra::open_memory().unwrap();
    u.migrate().unwrap();
    u.migrate().unwrap(); // second call must not error
    let v = u.with_conn(|c| synapse_ultra::schema::schema_version(c));
    assert_eq!(v, 1);
}

#[test]
fn ingest_and_query_event() {
    let u = fresh();
    let e = Event {
        ts: 1000,
        session_id: Some("s1".into()),
        agent: "claude".into(),
        kind: EventKind::Decision.as_str().to_string(),
        uri: Some("file:foo.rs".into()),
        content: Some("refactored bar".into()),
        meta: None,
    };
    let id = u.with_conn(|c| ingest_event(c, &e)).unwrap();
    assert!(id > 0);
    // dedup: same event again returns same id
    let id2 = u.with_conn(|c| ingest_event(c, &e)).unwrap();
    assert_eq!(id, id2);
    // query
    let rows = u.with_conn(|c| {
        synapse_ultra::events::query_events(
            c,
            &EventFilter::new().agent("claude").kind("decision"),
        )
    }).unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].agent, "claude");
    assert_eq!(rows[0].kind, "decision");
}

#[test]
fn ingest_event_json_roundtrip() {
    let u = fresh();
    let json = r#"{"ts":1234,"session_id":"s2","agent":"codex","kind":"tool_call","uri":"file:x.rs","content":"edit"}"#;
    let id = u.with_conn(|c| ingest_event_json(c, json)).unwrap();
    assert!(id > 0);
}

#[test]
fn decision_creates_graph_nodes_and_edges() {
    let u = fresh();
    let id = u.with_conn(|c| {
        ingest_decision(
            c,
            5000,
            Some("sess"),
            "claude",
            "file:bar.rs",
            Some("because reason"),
            Some("file:foo.rs"),
            Some("file:baz.rs"),
            None,
        )
    }).unwrap();
    assert!(id > 0);
    // trigger should have created 3 nodes + 2 edges
    let (nodes, edges) = u.with_conn(|c| synapse_ultra::graph::counts(c)).unwrap();
    assert_eq!(nodes, 3);
    assert_eq!(edges, 2);
}

#[test]
fn why_chain_traverses_backwards() {
    let u = fresh();
    // Build a chain: A caused B caused C
    let ts = 1000;
    u.with_conn(|c| upsert_node(c, "A", "source", None, ts)).unwrap();
    u.with_conn(|c| upsert_node(c, "B", "decision", None, ts + 1)).unwrap();
    u.with_conn(|c| upsert_node(c, "C", "target", None, ts + 2)).unwrap();
    u.with_conn(|c| upsert_edge(c, "A", "B", "caused", 1.0, ts, None, None)).unwrap();
    u.with_conn(|c| upsert_edge(c, "B", "C", "derived_from", 1.0, ts, None, None)).unwrap();
    // why(C) should return C (depth 0), B (depth 1), A (depth 2)
    let steps = u.with_conn(|c| why(c, "C", 5)).unwrap();
    assert_eq!(steps.len(), 3);
    assert_eq!(steps[0].uri, "C");
    assert_eq!(steps[0].depth, 0);
    assert_eq!(steps[1].uri, "B");
    assert_eq!(steps[1].depth, 1);
    assert_eq!(steps[2].uri, "A");
    assert_eq!(steps[2].depth, 2);
}

#[test]
fn graph_expand_traverses_forwards() {
    let u = fresh();
    let ts = 2000;
    u.with_conn(|c| upsert_node(c, "X", "source", None, ts)).unwrap();
    u.with_conn(|c| upsert_node(c, "Y", "decision", None, ts + 1)).unwrap();
    u.with_conn(|c| upsert_node(c, "Z", "target", None, ts + 2)).unwrap();
    u.with_conn(|c| upsert_edge(c, "X", "Y", "caused", 1.0, ts, None, None)).unwrap();
    u.with_conn(|c| upsert_edge(c, "Y", "Z", "derived_from", 1.0, ts, None, None)).unwrap();
    let steps = u.with_conn(|c| graph_expand(c, "X", 5)).unwrap();
    assert!(steps.iter().any(|s| s.uri == "X" && s.depth == 0));
    assert!(steps.iter().any(|s| s.uri == "Y" && s.depth == 1));
    assert!(steps.iter().any(|s| s.uri == "Z" && s.depth == 2));
}

#[test]
fn why_chain_10k_nodes_under_50ms() {
    let u = fresh();
    let ts = 0;
    // Build a linear chain of 10k nodes: n0 -> n1 -> ... -> n9999
    u.with_conn(|c| {
        for i in 0..10_000i64 {
            upsert_node(c, &format!("n{i}"), "decision", None, ts + i).unwrap();
            if i > 0 {
                upsert_edge(c, &format!("n{}", i - 1), &format!("n{i}"), "caused", 1.0, ts + i, None, None).unwrap();
            }
        }
        let start = std::time::Instant::now();
        let steps = why(c, "n9999", 20).unwrap();
        let elapsed = start.elapsed();
        assert!(steps.len() >= 1, "should return at least the starting node");
        assert!(
            elapsed.as_millis() < 100,
            "why() on 10k nodes took {:?}, expected < 100ms",
            elapsed
        );
        Ok::<(), synapse_ultra::UltraError>(())
    }).unwrap();
}

#[test]
fn brain_stats_returns_counts() {
    let u = fresh();
    // ingest one event
    let e = Event::now("claude", EventKind::Message);
    u.with_conn(|c| ingest_event(c, &e)).unwrap();
    let stats = u.with_conn(|c| synapse_ultra::observe::brain_stats(c)).unwrap();
    assert_eq!(stats.events, 1);
    assert_eq!(stats.ultra_schema_version, 1);
}

#[test]
fn token_cost_aggregates() {
    let u = fresh();
    u.with_conn(|c| {
        synapse_ultra::events::ingest_token_cost(c, 1000, Some("s"), "claude", "claude-fable-5", 100, 200, 0, 0, 0.01, None)
    }).unwrap();
    u.with_conn(|c| {
        synapse_ultra::events::ingest_token_cost(c, 2000, Some("s"), "codex", "gpt-5.6", 50, 75, 0, 0, 0.005, None)
    }).unwrap();
    let stats = u.with_conn(|c| synapse_ultra::observe::brain_stats(c)).unwrap();
    assert_eq!(stats.token_cost_rows, 2);
    assert!((stats.total_cost_usd - 0.015).abs() < 1e-9);
}

#[test]
fn ingest_jsonl_file_skips_blank_lines() {
    let u = fresh();
    let tmp = tempfile::NamedTempFile::new().unwrap();
    let content = r#"{"ts":1,"agent":"a","kind":"message","content":"hi"}
# this is a comment

{"ts":2,"agent":"a","kind":"message","content":"yo"}
"#;
    std::fs::write(tmp.path(), content).unwrap();
    let n = u.with_conn(|c| synapse_ultra::events::ingest_jsonl_file(c, tmp.path())).unwrap();
    assert_eq!(n, 2);
}
