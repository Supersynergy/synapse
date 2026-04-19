//! synapsed — persistent daemon. Unix socket + length-prefixed msgpack.

mod proto;

use anyhow::{Context, Result};
use clap::Parser;
use proto::{PutReq, Request, Response};
use std::path::PathBuf;
use std::sync::Arc;
use synapse_core::{embed::Embedder, snap, PutRequest, SearchMode, Store};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::Mutex;
use tracing::{error, info, warn};

#[derive(Parser)]
#[command(name = "synapsed", version, about = "Synapse daemon")]
struct Cli {
    #[arg(short = 'f', long, default_value = ".synapse/brain.db")]
    file: PathBuf,
    #[arg(short = 's', long, default_value = "/tmp/synapse.sock")]
    sock: PathBuf,
    /// Skip loading the embedding model at startup (lazy init on first use)
    #[arg(long, default_value_t = false)]
    lazy_embed: bool,
}

struct State {
    store: Mutex<Store>,
    embedder: Mutex<Option<Embedder>>,
    db_path: PathBuf,
}

impl State {
    async fn ensure_embedder(&self) -> Result<()> {
        let mut g = self.embedder.lock().await;
        if g.is_none() {
            info!("loading embedder (BGE-small-en-v1.5)…");
            let t0 = std::time::Instant::now();
            *g = Some(Embedder::new().context("embedder init")?);
            info!("embedder ready in {:?}", t0.elapsed());
        }
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "synapsed=info".into()))
        .init();
    let cli = Cli::parse();
    if let Some(p) = cli.file.parent() { std::fs::create_dir_all(p).ok(); }
    let store = Store::open(&cli.file).context("open store")?;
    let embedder = if cli.lazy_embed {
        None
    } else {
        info!("warming embedder…");
        Some(Embedder::new().context("embedder init")?)
    };
    let state = Arc::new(State {
        store: Mutex::new(store),
        embedder: Mutex::new(embedder),
        db_path: cli.file.clone(),
    });

    let _ = std::fs::remove_file(&cli.sock);
    let listener = UnixListener::bind(&cli.sock).with_context(|| format!("bind {}", cli.sock.display()))?;
    info!("listening on {} (db={})", cli.sock.display(), cli.file.display());

    loop {
        let (stream, _) = match listener.accept().await {
            Ok(p) => p,
            Err(e) => { error!("accept: {e}"); continue; }
        };
        let s = state.clone();
        tokio::spawn(async move {
            if let Err(e) = handle_conn(stream, s).await {
                warn!("conn: {e}");
            }
        });
    }
}

async fn handle_conn(mut stream: UnixStream, state: Arc<State>) -> Result<()> {
    loop {
        let mut lenbuf = [0u8; 4];
        if stream.read_exact(&mut lenbuf).await.is_err() { return Ok(()); }
        let len = u32::from_le_bytes(lenbuf) as usize;
        if len == 0 || len > 256 * 1024 * 1024 {
            return Err(anyhow::anyhow!("bad frame len {len}"));
        }
        let mut buf = vec![0u8; len];
        stream.read_exact(&mut buf).await?;
        let req: Request = rmp_serde::from_slice(&buf).context("decode request")?;
        let resp = dispatch(&state, req).await;
        let encoded = rmp_serde::to_vec_named(&resp)?;
        stream.write_all(&(encoded.len() as u32).to_le_bytes()).await?;
        stream.write_all(&encoded).await?;
        stream.flush().await?;
    }
}

async fn dispatch(state: &State, req: Request) -> Response {
    match req {
        Request::Ping => Response::Pong,
        Request::Put(p) => match put_one(state, p).await {
            Ok(id) => Response::Id(id),
            Err(e) => Response::Err(e.to_string()),
        },
        Request::PutBatch(batch) => match put_batch(state, batch).await {
            Ok(ids) => Response::Ids(ids),
            Err(e) => Response::Err(e.to_string()),
        },
        Request::Search { mode, q, limit, embed_query } => match search(state, mode, &q, limit, embed_query).await {
            Ok(hits) => Response::Hits(hits),
            Err(e) => Response::Err(e.to_string()),
        },
        Request::Stats => match state.store.lock().await.stats() {
            Ok(s) => Response::Stats { docs: s.docs, vecs: s.vecs },
            Err(e) => Response::Err(e.to_string()),
        },
        Request::Snap { out, level } => match snap::export(&state.db_path, &out, level) {
            Ok(()) => Response::Ok,
            Err(e) => Response::Err(e.to_string()),
        },
        Request::Shutdown => {
            info!("shutdown requested");
            std::process::exit(0);
        }
    }
}

async fn put_one(state: &State, p: PutReq) -> Result<i64> {
    let embedding = if p.embed {
        state.ensure_embedder().await?;
        let g = state.embedder.lock().await;
        let e = g.as_ref().expect("embedder present");
        Some(e.embed_one(&p.text)?)
    } else { None };
    let mut req: PutRequest = p.into();
    req.embedding = embedding;
    let mut store = state.store.lock().await;
    Ok(store.put(&req)?)
}

async fn put_batch(state: &State, batch: Vec<PutReq>) -> Result<Vec<i64>> {
    let any_embed = batch.iter().any(|r| r.embed);
    let embeddings = if any_embed {
        state.ensure_embedder().await?;
        let texts: Vec<String> = batch.iter().map(|r| r.text.clone()).collect();
        let g = state.embedder.lock().await;
        let e = g.as_ref().expect("embedder present");
        Some(e.embed_batch(&texts)?)
    } else { None };
    let reqs: Vec<PutRequest> = batch.into_iter().enumerate().map(|(i, p)| {
        let emb = if p.embed { embeddings.as_ref().map(|v| v[i].clone()) } else { None };
        let mut r: PutRequest = p.into();
        r.embedding = emb;
        r
    }).collect();
    let mut store = state.store.lock().await;
    Ok(store.put_batch(&reqs)?)
}

async fn search(state: &State, mode: SearchMode, q: &str, limit: usize, embed_query: bool) -> Result<Vec<synapse_core::Hit>> {
    let emb = if embed_query {
        state.ensure_embedder().await?;
        let g = state.embedder.lock().await;
        let e = g.as_ref().expect("embedder present");
        Some(e.embed_one(q)?)
    } else { None };
    let store = state.store.lock().await;
    Ok(store.search(q, mode, emb.as_deref(), limit)?)
}
