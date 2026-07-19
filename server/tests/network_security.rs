mod common;

use common::{TestServer, TOKEN};
use reqwest::blocking::Client;
use std::net::TcpListener;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};
use tungstenite::client::IntoClientRequest;
use tungstenite::http::{header::ORIGIN, HeaderValue};

#[test]
fn non_loopback_cleartext_is_always_rejected() {
    let temp = tempfile::tempdir().unwrap();
    let db = temp.path().join("nsfy.db");
    let status = Command::new(env!("CARGO_BIN_EXE_nsfyd"))
        .args(["--listen", "0.0.0.0:0", "--db-path"])
        .arg(db)
        .args(["--auth-token", TOKEN])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .unwrap();
    assert_eq!(status.code(), Some(2));
}

#[test]
fn cleartext_bypass_flag_does_not_exist() {
    let temp = tempfile::tempdir().unwrap();
    let status = Command::new(env!("CARGO_BIN_EXE_nsfyd"))
        .args(["--listen", "127.0.0.1:0", "--db-path"])
        .arg(temp.path().join("nsfy.db"))
        .arg("--allow-insecure-http")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .unwrap();
    assert_eq!(status.code(), Some(2));
}

#[test]
fn database_failure_is_fatal_instead_of_falling_back_to_memory() {
    let temp = tempfile::tempdir().unwrap();
    let status = Command::new(env!("CARGO_BIN_EXE_nsfyd"))
        .args(["--listen", "127.0.0.1:0", "--db-path"])
        .arg(temp.path())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .unwrap();
    assert_eq!(status.code(), Some(2));
}

#[test]
fn websocket_rejects_untrusted_browser_origins() {
    let server = TestServer::fresh();
    let mut request = format!("{}/alerts/ws", server.ws_url)
        .into_client_request()
        .unwrap();
    request.headers_mut().insert(
        ORIGIN,
        HeaderValue::from_static("https://malicious.example"),
    );
    assert!(tungstenite::connect(request).is_err());
}

#[test]
fn built_in_tls_encrypts_http_and_validates_the_server_certificate() {
    let certified = rcgen::generate_simple_self_signed(vec!["localhost".into()]).unwrap();
    let cert_pem = certified.cert.pem();
    let key_pem = certified.key_pair.serialize_pem();
    let temp = tempfile::tempdir().unwrap();
    let cert_path = temp.path().join("cert.pem");
    let key_path = temp.path().join("key.pem");
    let db_path = temp.path().join("nsfy.db");
    std::fs::write(&cert_path, &cert_pem).unwrap();
    std::fs::write(&key_path, key_pem).unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    let mut child = Command::new(env!("CARGO_BIN_EXE_nsfyd"))
        .args(["--listen", &format!("127.0.0.1:{port}"), "--db-path"])
        .arg(&db_path)
        .args(["--auth-token", TOKEN, "--tls-cert"])
        .arg(&cert_path)
        .arg("--tls-key")
        .arg(&key_path)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let url = format!("https://localhost:{port}/");
    let trusted = Client::builder()
        .add_root_certificate(reqwest::Certificate::from_pem(cert_pem.as_bytes()).unwrap())
        .timeout(Duration::from_millis(500))
        .build()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut ready = false;
    while Instant::now() < deadline {
        if trusted
            .get(&url)
            .bearer_auth(TOKEN)
            .send()
            .is_ok_and(|response| response.status().is_success())
        {
            ready = true;
            break;
        }
        thread::sleep(Duration::from_millis(30));
    }
    assert!(ready);
    assert!(Client::new().get(&url).send().is_err());
    let _ = child.kill();
    let _ = child.wait();
}
