use synapse_ultra::{Ultra, UltraError};
use synapse_ultra::graph::{upsert_node, upsert_edge, why, graph_expand};

fn bench_chain(n: i64) -> Result<(), UltraError> {
    let u = Ultra::open_memory()?;
    u.migrate()?;
    let ts = 0;
    u.with_conn(|c| {
        for i in 0..n {
            upsert_node(c, &format!("n{i}"), "decision", None, ts + i)?;
            if i > 0 {
                upsert_edge(c, &format!("n{}", i-1), &format!("n{i}"), "caused", 1.0, ts+i, None, None)?;
            }
        }
        Ok::<(), UltraError>(())
    })?;
    let start = std::time::Instant::now();
    let steps = u.with_conn(|c| why(c, &format!("n{}", n-1), 20))?;
    let elapsed = start.elapsed();
    println!("why(chain n={n}, depth=20): {} steps in {:?}", steps.len(), elapsed);
    Ok(())
}

fn bench_cycle(n: i64) -> Result<(), UltraError> {
    let u = Ultra::open_memory()?;
    u.migrate()?;
    let ts = 0;
    u.with_conn(|c| {
        for i in 0..n {
            upsert_node(c, &format!("n{i}"), "d", None, ts+i)?;
        }
        for i in 0..n {
            upsert_edge(c, &format!("n{i}"), &format!("n{}", (i+1)%n), "caused", 1.0, ts+i, None, None)?;
        }
        Ok::<(), UltraError>(())
    })?;
    let start = std::time::Instant::now();
    let steps = u.with_conn(|c| why(c, "n0", 100))?;
    let elapsed = start.elapsed();
    println!("why(cycle n={n}, depth=100): {} steps in {:?}", steps.len(), elapsed);
    Ok(())
}

fn main() -> Result<(), UltraError> {
    bench_chain(10_000)?;
    bench_chain(50_000)?;
    bench_cycle(1_000)?;
    bench_cycle(10_000)?;
    Ok(())
}
