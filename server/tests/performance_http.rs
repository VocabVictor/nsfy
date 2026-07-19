mod common;
#[path = "perf/common.rs"]
mod perf;

use perf::{report, scale, server};
use reqwest::blocking::Client;
use serde_json::json;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

fn publish_case(label: &str, workers: usize, total: usize, bytes: usize, topics: usize) {
    let temp = tempfile::tempdir().unwrap();
    let server = Arc::new(server(&temp.path().join("performance.db")));
    let samples = Arc::new(Mutex::new(Vec::with_capacity(total)));
    let started = Instant::now();
    let handles: Vec<_> = (0..workers)
        .map(|worker| {
            let server = Arc::clone(&server);
            let samples = Arc::clone(&samples);
            thread::spawn(move || {
                let client = Client::builder()
                    .timeout(Duration::from_secs(30))
                    .build()
                    .unwrap();
                for sequence in (worker..total).step_by(workers) {
                    let topic = sequence % topics;
                    let body = json!({
                        "title": "performance",
                        "message": "x".repeat(bytes),
                        "priority": 3,
                        "tags": ["benchmark"],
                        "category": ["performance", "http"]
                    });
                    let request_started = Instant::now();
                    server
                        .request(client.post(format!("{}/load-{topic}", server.base_url)))
                        .json(&body)
                        .send()
                        .unwrap()
                        .error_for_status()
                        .unwrap();
                    samples.lock().unwrap().push(request_started.elapsed());
                }
            })
        })
        .collect();
    for handle in handles {
        handle.join().unwrap();
    }
    report(label, total, started.elapsed(), &samples.lock().unwrap());
}

#[test]
#[ignore]
fn durable_publish_payload_matrix() {
    for (bytes, quick, full) in [(64, 100, 800), (1024, 100, 800), (32768, 20, 200)] {
        publish_case(
            &format!("durable-publish-{bytes}-bytes"),
            4,
            scale(quick, full),
            bytes,
            1,
        );
    }
}

#[test]
#[ignore]
fn durable_publish_concurrency_and_topic_topology() {
    let total = scale(200, 1200);
    for workers in [1, 4, 16] {
        publish_case(
            &format!("hot-topic-{workers}-workers"),
            workers,
            total,
            256,
            1,
        );
    }
    publish_case("distributed-64-topics", 16, total, 256, 64);
}

#[test]
#[ignore]
fn poll_replay_latency() {
    let temp = tempfile::tempdir().unwrap();
    let server = server(&temp.path().join("poll.db"));
    let client = Client::new();
    for sequence in 0..100 {
        server
            .request(client.post(format!("{}/poll", server.base_url)))
            .json(&json!({ "message": format!("message-{sequence}") }))
            .send()
            .unwrap()
            .error_for_status()
            .unwrap();
    }
    let count = scale(100, 1000);
    let mut samples = Vec::with_capacity(count);
    let started = Instant::now();
    for _ in 0..count {
        let request_started = Instant::now();
        let messages: Vec<serde_json::Value> = server
            .request(client.get(format!("{}/poll/json", server.base_url)))
            .send()
            .unwrap()
            .error_for_status()
            .unwrap()
            .json()
            .unwrap();
        assert_eq!(messages.len(), 100);
        samples.push(request_started.elapsed());
    }
    report(
        "poll-100-message-replay",
        count,
        started.elapsed(),
        &samples,
    );
}
