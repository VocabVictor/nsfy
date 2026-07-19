mod common;
#[path = "perf/common.rs"]
mod perf;

use common::TOKEN;
use perf::{scale, server};
use reqwest::blocking::Client;
use std::io::{BufRead, BufReader};
use std::sync::{Arc, Barrier};
use std::thread;
use std::time::Instant;
use sysinfo::{Pid, System};
use tungstenite::client::IntoClientRequest;
use tungstenite::http::{header::AUTHORIZATION, HeaderValue};

fn websocket(
    url: &str,
) -> tungstenite::WebSocket<tungstenite::stream::MaybeTlsStream<std::net::TcpStream>> {
    let mut request = url.into_client_request().unwrap();
    request.headers_mut().insert(
        AUTHORIZATION,
        HeaderValue::from_str(&format!("Bearer {TOKEN}")).unwrap(),
    );
    tungstenite::connect(request).unwrap().0
}

fn memory_bytes(pid: u32) -> u64 {
    let mut system = System::new();
    system.refresh_processes(
        sysinfo::ProcessesToUpdate::Some(&[Pid::from_u32(pid)]),
        true,
    );
    system.process(Pid::from_u32(pid)).unwrap().memory()
}

#[test]
#[ignore]
fn websocket_same_topic_connection_capacity() {
    let temp = tempfile::tempdir().unwrap();
    let server = server(&temp.path().join("connections.db"));
    let baseline = memory_bytes(server.pid());
    let count = scale(500, 10_000);
    let started = Instant::now();
    let mut sockets: Vec<_> = (0..count)
        .map(|_| websocket(&format!("{}/connections/ws", server.ws_url)))
        .collect();
    let connected_memory = memory_bytes(server.pid());
    println!(
        "websocket-same-topic-connections: count={count} elapsed={:.3}s baseline={}MiB connected={}MiB delta={:.1}KiB/connection",
        started.elapsed().as_secs_f64(), baseline / 1024 / 1024, connected_memory / 1024 / 1024,
        (connected_memory.saturating_sub(baseline)) as f64 / 1024.0 / count as f64,
    );
    assert_eq!(sockets.len(), count);
    let fanout_started = Instant::now();
    server
        .request(Client::new().post(format!("{}/connections", server.base_url)))
        .json(&serde_json::json!({ "message": "capacity-fanout" }))
        .send()
        .unwrap()
        .error_for_status()
        .unwrap();
    for socket in &mut sockets {
        assert!(socket.read().unwrap().is_text());
    }
    println!(
        "websocket-capacity-fanout: deliveries={count} elapsed={:.3}s rate={:.0}/s",
        fanout_started.elapsed().as_secs_f64(),
        count as f64 / fanout_started.elapsed().as_secs_f64(),
    );
}

#[test]
#[ignore]
fn websocket_distinct_topic_memory() {
    let temp = tempfile::tempdir().unwrap();
    let server = server(&temp.path().join("distinct-topics.db"));
    let baseline = memory_bytes(server.pid());
    let count = scale(100, 1500);
    let sockets: Vec<_> = (0..count)
        .map(|index| websocket(&format!("{}/topic-{index}/ws", server.ws_url)))
        .collect();
    let used = memory_bytes(server.pid());
    println!(
        "websocket-distinct-topics: count={count} baseline={}MiB connected={}MiB delta={:.1}KiB/topic+connection",
        baseline / 1024 / 1024, used / 1024 / 1024,
        (used.saturating_sub(baseline)) as f64 / 1024.0 / count as f64,
    );
    assert_eq!(sockets.len(), count);
}

#[test]
#[ignore]
fn retained_message_memory() {
    let temp = tempfile::tempdir().unwrap();
    let server = server(&temp.path().join("retained.db"));
    let baseline = memory_bytes(server.pid());
    let count = scale(200, 5000);
    let client = Client::new();
    let payload = "x".repeat(4096);
    for sequence in 0..count {
        server
            .request(client.post(format!("{}/cache-{}", server.base_url, sequence % 50)))
            .json(&serde_json::json!({ "message": payload }))
            .send()
            .unwrap()
            .error_for_status()
            .unwrap();
    }
    let used = memory_bytes(server.pid());
    println!(
        "retained-message-memory: messages={count} payload=4096B baseline={}MiB loaded={}MiB delta={}MiB",
        baseline / 1024 / 1024, used / 1024 / 1024,
        used.saturating_sub(baseline) / 1024 / 1024,
    );
}

#[test]
#[ignore]
fn websocket_fanout_delivery() {
    let temp = tempfile::tempdir().unwrap();
    let server = server(&temp.path().join("fanout.db"));
    let subscriber_count = scale(25, 250);
    let message_count = scale(5, 25);
    let mut sockets: Vec<_> = (0..subscriber_count)
        .map(|_| websocket(&format!("{}/fanout/ws", server.ws_url)))
        .collect();
    let client = Client::new();
    let started = Instant::now();
    for sequence in 0..message_count {
        server
            .request(client.post(format!("{}/fanout", server.base_url)))
            .json(&serde_json::json!({ "message": format!("fanout-{sequence}") }))
            .send()
            .unwrap()
            .error_for_status()
            .unwrap();
    }
    for socket in &mut sockets {
        for _ in 0..message_count {
            assert!(socket.read().unwrap().is_text());
        }
    }
    let deliveries = subscriber_count * message_count;
    println!(
        "websocket-fanout: subscribers={subscriber_count} messages={message_count} deliveries={deliveries} elapsed={:.3}s rate={:.0}/s",
        started.elapsed().as_secs_f64(), deliveries as f64 / started.elapsed().as_secs_f64(),
    );
}

#[test]
#[ignore]
fn sse_fanout_first_message_latency() {
    let temp = tempfile::tempdir().unwrap();
    let server = Arc::new(server(&temp.path().join("sse.db")));
    let subscribers = scale(10, 100);
    let barrier = Arc::new(Barrier::new(subscribers + 1));
    let handles: Vec<_> = (0..subscribers)
        .map(|_| {
            let server = Arc::clone(&server);
            let barrier = Arc::clone(&barrier);
            thread::spawn(move || {
                let response = server
                    .request(Client::new().get(format!("{}/sse/sse", server.base_url)))
                    .send()
                    .unwrap()
                    .error_for_status()
                    .unwrap();
                barrier.wait();
                let mut reader = BufReader::new(response);
                let mut line = String::new();
                loop {
                    reader.read_line(&mut line).unwrap();
                    if line.starts_with("data:") {
                        break;
                    }
                    line.clear();
                }
            })
        })
        .collect();
    barrier.wait();
    let started = Instant::now();
    server
        .request(Client::new().post(format!("{}/sse", server.base_url)))
        .json(&serde_json::json!({ "message": "fanout" }))
        .send()
        .unwrap()
        .error_for_status()
        .unwrap();
    for handle in handles {
        handle.join().unwrap();
    }
    println!(
        "sse-fanout: subscribers={subscribers} first-message-complete={:.2}ms",
        started.elapsed().as_secs_f64() * 1000.0,
    );
}
