use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use synapse_core::{embed::Embedder, snap, PutRequest, SearchMode, Store};

#[derive(Parser)]
#[command(name = "synapse", version, about = "Single-file memory for AI agents")]
struct Cli {
    #[arg(short = 'f', long, default_value = ".synapse/brain.db", global = true)]
    file: PathBuf,
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Initialize a new memory file
    Init,
    /// Append a doc from stdin (or --text)
    Put {
        #[arg(long)] title: Option<String>,
        #[arg(long)] uri: Option<String>,
        #[arg(long)] text: Option<String>,
        #[arg(long, default_value_t = false)] no_embed: bool,
    },
    /// Lexical FTS5 search
    Find { query: String, #[arg(long, default_value_t = 10)] limit: usize },
    /// Vector kNN search
    Vec { query: String, #[arg(long, default_value_t = 10)] limit: usize },
    /// Hybrid (RRF fusion) search
    Hybrid { query: String, #[arg(long, default_value_t = 10)] limit: usize },
    /// Stats
    Stats,
    /// Export to .brainpack
    Snap { out: PathBuf, #[arg(long, default_value_t = 3)] level: i32 },
    /// Import a .brainpack into this file
    Restore { pack: PathBuf },
}

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();
    let cli = Cli::parse();
    if let Some(p) = cli.file.parent() { std::fs::create_dir_all(p).ok(); }
    match cli.cmd {
        Cmd::Init => {
            Store::open(&cli.file)?;
            println!("ok init {}", cli.file.display());
        }
        Cmd::Put { title, uri, text, no_embed } => {
            let body = match text {
                Some(t) => t,
                None => {
                    let mut s = String::new();
                    std::io::Read::read_to_string(&mut std::io::stdin(), &mut s)?;
                    s.trim().to_string()
                }
            };
            anyhow::ensure!(!body.is_empty(), "empty text");
            let mut store = Store::open(&cli.file)?;
            let embedding = if no_embed { None } else {
                let mut e = Embedder::new_with_cache::<std::path::PathBuf>(cli.file.parent().map(|p| p.join(".emb-cache"))).context("embedder init")?;
                Some(e.embed_one(&body)?)
            };
            let id = store.put(&PutRequest { title, uri, text: body, embedding, ..Default::default() })?;
            println!("{}", id);
        }
        Cmd::Find { query, limit } => {
            let store = Store::open(&cli.file)?;
            let hits = store.search(&query, SearchMode::Lex, None, limit)?;
            print_hits(&hits);
        }
        Cmd::Vec { query, limit } => {
            let store = Store::open(&cli.file)?;
            let mut e = Embedder::new_with_cache::<std::path::PathBuf>(cli.file.parent().map(|p| p.join(".emb-cache")))?;
            let q = e.embed_one(&query)?;
            let hits = store.search("", SearchMode::Vec, Some(&q), limit)?;
            print_hits(&hits);
        }
        Cmd::Hybrid { query, limit } => {
            let store = Store::open(&cli.file)?;
            let mut e = Embedder::new_with_cache::<std::path::PathBuf>(cli.file.parent().map(|p| p.join(".emb-cache")))?;
            let q = e.embed_one(&query)?;
            let hits = store.search(&query, SearchMode::Hybrid, Some(&q), limit)?;
            print_hits(&hits);
        }
        Cmd::Stats => {
            let store = Store::open(&cli.file)?;
            let s = store.stats()?;
            println!("{}", serde_json::to_string_pretty(&s)?);
        }
        Cmd::Snap { out, level } => {
            snap::export(&cli.file, &out, level)?;
            println!("ok snap {}", out.display());
        }
        Cmd::Restore { pack } => {
            snap::import(&pack, &cli.file)?;
            println!("ok restore {}", cli.file.display());
        }
    }
    Ok(())
}

fn print_hits(hits: &[synapse_core::Hit]) {
    for h in hits {
        println!("{}\t{:.4}\t{}", h.id, h.score, h.text.chars().take(120).collect::<String>());
    }
}
