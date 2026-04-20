use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use synapse_core::{embed::Embedder, sign, snap, PutRequest, SearchMode, Store};

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
        /// Path to Ed25519 signing key (32-byte raw file)
        #[arg(long)] sign: Option<PathBuf>,
    },
    /// Verify Ed25519 signature of a doc by id
    Verify {
        id: i64,
        /// Path to verifying key (32-byte raw file)
        #[arg(long)] vk: PathBuf,
    },
    /// Generate an Ed25519 keypair
    Keygen {
        /// Output secret key path
        #[arg(long, default_value = "synapse.sk")] sk: PathBuf,
        /// Output public key path
        #[arg(long, default_value = "synapse.vk")] vk: PathBuf,
    },
    /// Export signed .brainpack
    SnapSigned {
        out: PathBuf,
        #[arg(long, default_value_t = 3)] level: i32,
        #[arg(long)] sk: PathBuf,
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
    /// (extension doesn't matter, content does — .syn/.synapse/.brainpack/.bp all accepted)
    Restore { pack: PathBuf },
    /// Merge two brainpacks by URI-matching docs, CRDT-merging meta_crdt per doc
    /// (extension doesn't matter, content does — .syn/.synapse/.brainpack/.bp all accepted)
    Merge {
        file_a: PathBuf,
        file_b: PathBuf,
        #[arg(short = 'o', long)] out: PathBuf,
        #[arg(long, default_value_t = 3)] level: i32,
    },
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
        Cmd::Put { title, uri, text, no_embed, sign: sign_path } => {
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
            let req = PutRequest { title, uri, text: body, embedding, ..Default::default() };
            let id = if let Some(sk_path) = sign_path {
                let sk = sign::load_signing_key(&sk_path).context("load signing key")?;
                store.put_signed(&req, Some(&sk))?
            } else {
                store.put(&req)?
            };
            println!("{}", id);
        }
        Cmd::Verify { id, vk } => {
            let store = Store::open(&cli.file)?;
            let vk = sign::load_verifying_key(&vk).context("load verifying key")?;
            store.verify(id, &vk)?;
            println!("ok verified id={}", id);
        }
        Cmd::Keygen { sk, vk } => {
            sign::keygen(&sk, &vk).context("keygen")?;
            println!("ok sk={} vk={}", sk.display(), vk.display());
        }
        Cmd::SnapSigned { out, level, sk } => {
            let signing_key = sign::load_signing_key(&sk).context("load signing key")?;
            snap::export_signed(&cli.file, &out, level, &signing_key)?;
            println!("ok snap-signed {}", out.display());
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
        Cmd::Merge { file_a, file_b, out, level } => {
            snap::merge_packs(&file_a, &file_b, &out, level)?;
            println!("ok merge {}", out.display());
        }
    }
    Ok(())
}

fn print_hits(hits: &[synapse_core::Hit]) {
    for h in hits {
        println!("{}\t{:.4}\t{}", h.id, h.score, h.text.chars().take(120).collect::<String>());
    }
}
