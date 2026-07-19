#![allow(dead_code)]

use reqwest::blocking::{Client, RequestBuilder};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};
use tempfile::TempDir;

pub const TOKEN: &str = "test-token-0123456789abcdef";

pub struct TestServer {
    child: Child,
    pub base_url: String,
    pub ws_url: String,
    pub db_path: PathBuf,
    _temp: Option<TempDir>,
}

impl TestServer {
    pub fn fresh() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let db_path = temp.path().join("nsfy.db");
        let mut server = Self::spawn(&db_path, &[]);
        server._temp = Some(temp);
        server
    }

    pub fn spawn(db_path: &Path, extra: &[&str]) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        drop(listener);
        let address = format!("127.0.0.1:{port}");
        let mut command = Command::new(env!("CARGO_BIN_EXE_nsfyd"));
        command
            .args(["--listen", &address, "--db-path"])
            .arg(db_path)
            .args(["--auth-token", TOKEN])
            .args(extra)
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        let child = command.spawn().unwrap();
        let server = Self {
            child,
            base_url: format!("http://{address}"),
            ws_url: format!("ws://{address}"),
            db_path: db_path.to_path_buf(),
            _temp: None,
        };
        server.wait_until_ready();
        server
    }

    pub fn request(&self, builder: RequestBuilder) -> RequestBuilder {
        builder.bearer_auth(TOKEN)
    }

    pub fn pid(&self) -> u32 {
        self.child.id()
    }

    pub fn stop(mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }

    fn wait_until_ready(&self) {
        let client = Client::builder()
            .timeout(Duration::from_millis(300))
            .build()
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(10);
        while Instant::now() < deadline {
            if self
                .request(client.get(format!("{}/", self.base_url)))
                .send()
                .is_ok_and(|response| response.status().is_success())
            {
                return;
            }
            thread::sleep(Duration::from_millis(30));
        }
        panic!("nsfyd did not become ready at {}", self.base_url);
    }
}

impl Drop for TestServer {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

pub fn message(body: &str) -> serde_json::Value {
    serde_json::json!({
        "title": "protocol test",
        "message": body,
        "priority": 4,
        "tags": ["test"],
        "category": ["qa", "protocol"]
    })
}
