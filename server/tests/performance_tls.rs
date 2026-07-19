mod common;
#[path = "perf/common.rs"]
mod perf;

use common::TOKEN;
use perf::{report, scale};
use reqwest::blocking::Client;
use std::net::TcpListener;
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

struct TlsServer {
    child: Child,
    url: String,
    client: Client,
}

impl Drop for TlsServer {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn spawn_tls(temp: &tempfile::TempDir) -> TlsServer {
    let certified = rcgen::generate_simple_self_signed(vec!["localhost".into()]).unwrap();
    let cert_pem = certified.cert.pem();
    let cert_path = temp.path().join("cert.pem");
    let key_path = temp.path().join("key.pem");
    std::fs::write(&cert_path, &cert_pem).unwrap();
    std::fs::write(&key_path, certified.key_pair.serialize_pem()).unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    let mut child = Command::new(env!("CARGO_BIN_EXE_nsfyd"))
        .args(["--listen", &format!("127.0.0.1:{port}"), "--db-path"])
        .arg(temp.path().join("tls.db"))
        .args(["--auth-token", TOKEN, "--tls-cert"])
        .arg(cert_path)
        .arg("--tls-key")
        .arg(key_path)
        .args([
            "--rate-limit-per-min",
            "10000000",
            "--topic-rate-limit-per-min",
            "10000000",
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let client = Client::builder()
        .add_root_certificate(reqwest::Certificate::from_pem(cert_pem.as_bytes()).unwrap())
        .timeout(Duration::from_secs(30))
        .build()
        .unwrap();
    let url = format!("https://localhost:{port}");
    let deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < deadline {
        if client
            .get(format!("{url}/"))
            .bearer_auth(TOKEN)
            .send()
            .is_ok_and(|response| response.status().is_success())
        {
            return TlsServer { child, url, client };
        }
        thread::sleep(Duration::from_millis(30));
    }
    let _ = child.kill();
    let _ = child.wait();
    panic!("TLS server did not become ready");
}

#[test]
#[ignore]
fn https_durable_publish_latency() {
    let temp = tempfile::tempdir().unwrap();
    let server = spawn_tls(&temp);
    let count = scale(100, 1000);
    let mut samples = Vec::with_capacity(count);
    let started = Instant::now();
    for sequence in 0..count {
        let request_started = Instant::now();
        server
            .client
            .post(format!("{}/tls", server.url))
            .bearer_auth(TOKEN)
            .json(&serde_json::json!({ "message": format!("tls-{sequence}") }))
            .send()
            .unwrap()
            .error_for_status()
            .unwrap();
        samples.push(request_started.elapsed());
    }
    report("https-durable-publish", count, started.elapsed(), &samples);
}
