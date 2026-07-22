//! Verify pool concurrency: N threads doing why() in parallel.
use synapse_ultra::{Ultra, UltraError};
use synapse_ultra::graph::{upsert_node, upsert_edge, why};
use std::sync::Arc;
use std::thread;

fn main() -> Result<(), UltraError> {
    let u = Arc::new(Ultra::open_memory()?);
    u.migrate()?;
    // Build a chain n0 -> n1 -> ... -> n999
    u.with_conn(|c| {
        for i in 0..1000 {
            upsert_node(c, &format!("n{i}"), "d", None, i)?;
            if i > 0 {
                upsert_edge(c, &format!("n{}", i-1), &format!("n{i}"), "caused", 1.0, i, None, None)?;
            }
        }
        Ok::<(), UltraError>(())
    })?;
    println!("Pool size after setup: {}", u.pool_size());

    let mut handles = vec![];
    let start = std::time::Instant::now();
    for t in 0..8 {
        let u2 = Arc::clone(&u);
        handles.push(thread::spawn(move || {
            let r = u2.with_conn(|c| why(c, "n999", 20)).unwrap();
            (t, r.len())
        }));
    }
    for h in handles {
        let (t, n) = h.join().unwrap();
        println!("thread {t}: {n} steps");
    }
    println!("8 concurrent why() in {:?}", start.elapsed());
    println!("Pool size after: {}", u.pool_size());
    Ok(())
}
