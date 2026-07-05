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

## Security

```bash
./target/release/nsfyd \
  --listen 0.0.0.0:8080 \
  --auth-token "$(openssl rand -hex 32)" \
  --rate-limit-per-min 300 \
  --max-topics 10000
```

| Flag / env | Default | What it does |
|---|---|---|
| `--auth-token` / `NSFY_AUTH_TOKEN` | none | When set, every route — including `/` — requires it. Pass it as `Authorization: Bearer <token>` (preferred, doesn't leak into access logs) or `?auth=<token>`. Compared in constant time so a wrong guess can't be timed. |
| `--rate-limit-per-min` / `NSFY_RATE_LIMIT_PER_MIN` | 300 | Per-IP token bucket covering HTTP requests and WS-originated publishes alike. Over budget → `429`. |
| `--max-topics` / `NSFY_MAX_TOPICS` | 10000 | Caps how many distinct topics the server will track at once, so an unauthenticated caller can't grow the topic table without bound. Over the cap → `503` for new topic names. |
| `--max-msg-size` / `NSFY_MAX_MSG_SIZE` | 65536 | Enforced both on the raw HTTP body and on the `message` field (also bounds WS frame/message size). `title` is capped at 512 bytes, tags at 32 entries of 64 bytes each. |
| `--topic-rate-limit-per-min` / `NSFY_TOPIC_RATE_LIMIT_PER_MIN` | 1200 | Aggregate publish budget per topic, across all IPs — catches a distributed flood aimed at one topic that per-IP limiting alone won't. |
| `--max-conns-per-ip` / `NSFY_MAX_CONNS_PER_IP` | 20 | Concurrent WS/SSE connections allowed per IP. Each connection only costs one request against the rate limiter, so this caps the separate cost of holding sockets open. |
| `--max-conns-total` / `NSFY_MAX_CONNS_TOTAL` | 10000 | Concurrent WS/SSE connections allowed server-wide. |
| `--trust-proxy` / `NSFY_TRUST_PROXY` | false | Use `X-Forwarded-For` / `X-Real-IP` as the client IP for rate limiting instead of the TCP peer address. **Only enable this behind a reverse proxy you control that sets/overwrites these headers** — see the TLS note below for why this matters. |

Topic names are restricted to `[A-Za-z0-9._-]`, max 128 characters — long enough for anything reasonable, and it keeps control characters out of log lines. Invalid names get a `400`.

If you run without `--auth-token`, the topic name is your only secret — pick something unguessable (`curl .../$(openssl rand -hex 16)`), same as ntfy.sh's model. Once you set a token, `/` also requires it, so topic names can't be enumerated by an outsider.

A panic in one request's handler is caught and turned into a `500` — it can't take the whole process (and every other client's connection) down with it.

### TLS

`nsfyd` speaks plain HTTP/WS, no TLS built in — that's a deliberate size tradeoff. Put a reverse proxy (nginx, Caddy, [Tailscale Serve](https://tailscale.com/kb/1312/serve)) in front for anything that crosses a network you don't control, otherwise the token and every message go out in cleartext.

If you do put a reverse proxy in front, pass `--trust-proxy` so rate limiting keys on the real client IP instead of the proxy's — but only if `nsfyd` itself isn't *also* reachable directly. If it is, a client can set its own `X-Forwarded-For` and dodge its rate limit entirely.

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
