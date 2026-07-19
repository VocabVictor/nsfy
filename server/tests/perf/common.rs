#![allow(dead_code)]

use crate::common::TestServer;
use std::path::Path;
use std::time::Duration;

pub fn scale(quick: usize, full: usize) -> usize {
    match std::env::var("NSFY_PERF_PROFILE").as_deref() {
        Ok("quick") => quick,
        _ => full,
    }
}

pub fn server(db: &Path) -> TestServer {
    TestServer::spawn(
        db,
        &[
            "--rate-limit-per-min",
            "100000000",
            "--topic-rate-limit-per-min",
            "100000000",
            "--bandwidth-limit-per-min",
            "100000000000",
            "--topic-creation-limit-per-min",
            "1000000",
            "--max-conns-per-ip",
            "20000",
            "--max-conns-total",
            "20000",
            "--max-topics",
            "20000",
        ],
    )
}

pub fn percentile(samples: &[Duration], percentile: usize) -> Duration {
    let mut values = samples.to_vec();
    values.sort_unstable();
    let index = (values.len().saturating_sub(1) * percentile) / 100;
    values[index]
}

pub fn report(label: &str, count: usize, elapsed: Duration, samples: &[Duration]) {
    println!(
        "{label}: count={count} elapsed={:.3}s rate={:.0}/s p50={:.2}ms p95={:.2}ms p99={:.2}ms",
        elapsed.as_secs_f64(),
        count as f64 / elapsed.as_secs_f64(),
        percentile(samples, 50).as_secs_f64() * 1000.0,
        percentile(samples, 95).as_secs_f64() * 1000.0,
        percentile(samples, 99).as_secs_f64() * 1000.0,
    );
}
