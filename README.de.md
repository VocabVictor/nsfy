<p align="center">
  <a href="README.zh-CN.md">中文</a> | <a href="README.md">English</a> | <a href="README.ja.md">日本語</a> | <a href="README.fr.md">Français</a> | <a href="README.de.md">Deutsch</a>
</p>

<p align="center">
  <img src="https://img.shields.io/badge/license-MIT-blue" />
  <img src="https://img.shields.io/badge/server-Rust-f74c00?logo=rust" />
  <img src="https://img.shields.io/badge/desktop-Tauri-24c8a0?logo=tauri" />
  <img src="https://img.shields.io/badge/android-Kotlin-7f52ff?logo=kotlin" />
  <img src="https://img.shields.io/badge/minSdk-21-green?logo=android" />
  <img src="https://img.shields.io/badge/Größe-1.2MB_Server_|_1.6MB_APK-lightgrey" />
</p>

---

# nsfy

Ein selbst gehostetes Push-Benachrichtigungssystem ohne Cloud-Konto. Der Server besteht aus einer einzigen Rust-Binärdatei. Desktop und Android halten dauerhafte WebSocket-Verbindungen.

Unterschied zu ntfy.sh: kein öffentlicher Dienst, keine Registrierung, kein Firebase. Sie hosten, Sie kontrollieren.

## So funktioniert's

```
curl -d '{"title":"backup","message":"done"}'  \
     nsfyd:8080/backups                          POST /<topic>
                                                       │
┌──────────────────── nsfyd ────────────────────┐      │
│  axum + dashmap + broadcast channels           │      │
│  10k Verbindungen geprüft; siehe PERFORMANCE.md │      │
└──────────────────────┬─────────────────────────┘      │
                       │ wss://<topic>/ws               │
         ┌─────────────┼─────────────┐                  │
         ▼             ▼             ▼                  │
   ┌──────────┐ ┌──────────┐ ┌──────────┐              │
   │ Desktop  │ │ Android  │ │  curl    │              │
   │ 2.0 MB   │ │ 1.6 MB   │ │  0 MB    │              │
   └──────────┘ └──────────┘ └──────────┘              │
```

## Schnellstart

```bash
git clone https://github.com/VocabVictor/nsfy.git
cd nsfy/server && cargo build --release
./target/release/nsfyd --listen 127.0.0.1:8080 --db-path ./nsfy.db
```

```bash
# Nachricht senden
curl -X POST http://localhost:8080/alerts \
  -H "Content-Type: application/json" \
  -d '{"title":"CPU","message":"95% Auslastung","priority":5}'

# Per WebSocket abonnieren
websocat ws://localhost:8080/alerts/ws

# Statistiken
curl http://localhost:8080/
```

## API

| Methode | Pfad | Beschreibung |
|---------|------|-------------|
| `POST /:topic` | JSON-Body | Nachricht senden |
| `GET /:topic/ws` | WebSocket | Echtzeit-Abonnement |
| `GET /:topic/sse` | SSE | Browser-Stream |
| `GET /:topic/json?since=:id` | JSON-Array | HTTP-Polling |
| `GET /` | JSON | Statistik |

## Komponenten

| Komponente | Technologie | Größe |
|------------|-------------|-------|
| Server | Rust + axum | 1,2 MB |
| Desktop | Tauri + Svelte | 2,0 MB |
| Android | Kotlin + Compose | 1,6 MB |

## License

MIT
