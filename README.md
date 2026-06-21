<p align="center">
  <a href="README.zh-CN.md">中文</a> | <a href="README.md">English</a> | <a href="README.ja.md">日本語</a> | <a href="README.fr.md">Français</a> | <a href="README.de.md">Deutsch</a>
</p>

<p align="center">
  <img src="https://img.shields.io/badge/license-MIT-blue" />
  <img src="https://img.shields.io/badge/server-Rust-f74c00?logo=rust" />
  <img src="https://img.shields.io/badge/desktop-Tauri-24c8a0?logo=tauri" />
  <img src="https://img.shields.io/badge/android-Kotlin-7f52ff?logo=kotlin" />
  <img src="https://img.shields.io/badge/minSdk-21-green?logo=android" />
  <img src="https://img.shields.io/badge/size-1.2MB_server_|_1.6MB_apk-lightgrey" />
</p>

---

# nsfy

A push notification system that doesn't need a cloud account.

One Rust binary on a VPS. A desktop tray app. An Android app. Publish with curl, subscribe over WebSocket. That's the whole thing.

No public service to depend on. No account to register. No Firebase.

## How it works

```
curl -d '{"title":"backup","message":"done"}'  \
     nsfyd:8080/backups                          POST /<topic>
                                                       │
┌──────────────────── nsfyd ────────────────────┐      │
│  axum + dashmap + broadcast channels           │      │
│  1.2 MB binary · ~7 MB idle · 10k connections  │      │
└──────────────────────┬─────────────────────────┘      │
                       │ ws://<topic>/ws                │
         ┌─────────────┼─────────────┐                  │
         ▼             ▼             ▼                  │
   ┌──────────┐ ┌──────────┐ ┌──────────┐              │
   │ Desktop  │ │ Android  │ │  curl    │              │
   │ Tauri    │ │ Kotlin   │ │  SSE     │              │
   │ 2.0 MB   │ │ 1.6 MB   │ │  0 MB    │              │
   └──────────┘ └──────────┘ └──────────┘              │
```

One topic, three subscribers. Same message lands everywhere at once.

## Quick start

### Server

```bash
git clone https://github.com/VocabVictor/nsfy.git
cd nsfy/server
cargo build --release

./target/release/nsfyd --listen 0.0.0.0:8080
```

systemd unit:

```ini
[Unit]
Description=nsfy
After=network.target

[Service]
ExecStart=/opt/nsfy/nsfyd --listen 0.0.0.0:8080
Restart=always
RestartSec=5

[Install]
WantedBy=multi-user.target
```

### Publish

```bash
curl -X POST http://localhost:8080/alerts \
  -H "Content-Type: application/json" \
  -d '{"title":"disk","message":"90% full","priority":5,"tags":["server"]}'
```

### Subscribe

```bash
# WebSocket — real-time, bidirectional
websocat ws://localhost:8080/alerts/ws

# SSE — browser-friendly
curl http://localhost:8080/alerts/sse

# HTTP poll — when you can't hold a connection
curl http://localhost:8080/alerts/json
```

### Stats

```bash
curl http://localhost:8080/
# → {"topics":3,"total_subscribers":12,"topic_names":["alerts","backups","chat"]}
```

## API

| Method | Path | What it does |
|--------|------|-------------|
| `POST /:topic` | body: `{title, message, priority?, tags?}` | Send a message |
| `GET /:topic/ws` | WebSocket upgrade | Real-time subscribe |
| `GET /:topic/sse` | text/event-stream | SSE stream |
| `GET /:topic/json?since=:id` | JSON array | Poll since a given message id |
| `GET /` | JSON | Topic list + subscriber count |

Message format:

```json
{
  "id": "01J...",
  "time": 1718832000,
  "title": "Backup done",
  "message": "Database backup finished, 2.3GB",
  "priority": 3,
  "tags": ["backup", "db"]
}
```

Priority runs 1 (low) through 5 (critical). Tags are free-form strings — filter on the client side however you want.

## Components

| Piece | Stack | Size | Notes |
|-------|-------|------|-------|
| Server | Rust + axum | 1.2 MB | Single binary, one systemd line |
| Desktop | Tauri + Svelte | 2.0 MB | System tray, native notifications |
| Android | Kotlin + Compose | 1.6 MB | minSdk 21 — works on phones from 2014 |

## Building

### Desktop

```bash
cd nsfy/desktop
npm install
cargo tauri build
# → src-tauri/target/release/bundle/
```

Produces `.dmg` on macOS, `.msi` on Windows, `.AppImage` on Linux.

### Android

```bash
cd nsfy/android
./gradlew assembleRelease
adb install app/build/outputs/apk/release/app-release.apk
```

On first launch the app connects to `localhost:8419` — handy with `adb reverse tcp:8419 tcp:8419` for dev. Set your actual server address in Settings for production use.

## Project layout

```
nsfy/
├── server/          Rust — axum, dashmap, broadcast
│   └── src/         main / config / pubsub / handlers / message
├── desktop/         Tauri — Svelte 5 + Rust WS manager
│   ├── src/         Components and stores
│   └── src-tauri/   Tauri Rust backend
├── android/         Kotlin — Compose, Room, OkHttp
│   └── app/         service / data / ui
├── README.md
├── README.zh-CN.md
├── README.ja.md
├── README.fr.md
└── README.de.md
```

## License

MIT — take it, fork it, ship it. Keep the license notice and you're good.
