<!-- ┌──────────────────────────────────────────────┐
     │  nsfy  —  minimal pub-sub notification system  │
     │  1 binary + desktop + android.  that's it.     │
     └──────────────────────────────────────────────┘ -->

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

**EN** — A push notification system that doesn't need a cloud account.  
**ZH** — 一个不需要云账号的推送通知系统。  
**JP** — クラウドアカウント不要のプッシュ通知システム。  
**FR** — Un système de notification push sans compte cloud.  
**DE** — Ein Push-Benachrichtigungssystem ohne Cloud-Konto.

A single Rust binary on a VPS, a desktop tray app, an Android app.  
Publish with curl. Subscribe over WebSocket. Done.

---

## How it works

```
curl -d '{"title":"backup","message":"done"}'  \
     nsfyd:8080/backups                           POST /<topic>
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

One topic, three subscribers. Same message hits all at once.

---

## Badges explained

| Badge | What it says |
|-------|-------------|
| MIT | Grab the code, do what you want |
| Rust | Server runs on a potato — 7 MB idle |
| Tauri | Desktop app uses system webview, not Chromium |
| Kotlin | Native Android, no cross-platform compromises |
| minSdk 21 | Works back to Android 5.0 (2014) |
| 1.2 MB / 1.6 MB | Binary sizes. The whole thing fits on a floppy |

---

## Quick start

### Server

```bash
git clone https://github.com/VocabVictor/nsfy.git
cd nsfy/server
cargo build --release

./target/release/nsfyd --listen 0.0.0.0:8080
# → nsfyd listening on 0.0.0.0:8080
```

### Publish (any language, any tool)

```bash
curl -X POST http://localhost:8080/alerts \
  -H "Content-Type: application/json" \
  -d '{"title":"disk","message":"90% full","priority":5,"tags":["server"]}'
# → {"id":"...","time":...,"title":"disk","message":"90% full",...}
```

### Subscribe

```bash
# WebSocket — real-time, bidirectional
websocat ws://localhost:8080/alerts/ws

# SSE — browser-friendly, one-way
curl http://localhost:8080/alerts/sse

# HTTP poll — no persistent connection needed
curl http://localhost:8080/alerts/json
```

### Stats

```bash
curl http://localhost:8080/
# → {"topics":3,"total_subscribers":12,"topic_names":["alerts","backups","chat"]}
```

---

## API

| Method | Path | Meaning |
|--------|------|---------|
| `POST /:topic` | body: `{title, message, priority?, tags?}` | Send a message |
| `GET /:topic/ws` | WebSocket upgrade | Real-time subscribe |
| `GET /:topic/sse` | text/event-stream | SSE stream |
| `GET /:topic/json?since=:id` | JSON array | Poll messages since id |
| `GET /` | JSON | Topic list + subscriber count |

Priority: 1 (low) → 5 (critical). Tags: any string array, used for filtering on the client side.

---

<!-- ═══════════════════════════════════════════ -->
<!-- 中文 · CHINESE                               -->
<!-- ═══════════════════════════════════════════ -->

## 中文说明

### 这是什么

nsfy 是一个自托管的推送通知系统。服务端是一个 1.2MB 的 Rust 二进制文件，跑在任意 VPS 上。桌面端和 Android 端通过 WebSocket 长连接实时接收消息。

跟 ntfy.sh 的区别：不需要公网服务，不需要注册账号，不依赖 Firebase。你自己跑服务端，自己控制数据。

### 三端

| 端 | 技术 | 大小 | 说明 |
|----|------|------|------|
| 服务端 | Rust + axum | 1.2 MB | 单二进制，systemd 一行配 |
| 桌面端 | Tauri + Svelte | 2.0 MB | 系统托盘常驻，原生通知 |
| Android | Kotlin + Compose | 1.6 MB | minSdk 21，支持 2014 年以后所有手机 |

### 服务端安装

```bash
git clone https://github.com/VocabVictor/nsfy.git
cd nsfy/server && cargo build --release
./target/release/nsfyd --listen 0.0.0.0:8080
```

systemd unit:

```ini
[Unit]
Description=nsfy notification server
After=network.target

[Service]
ExecStart=/opt/nsfy/nsfyd --listen 0.0.0.0:8080
Restart=always
RestartSec=5

[Install]
WantedBy=multi-user.target
```

### Android 端安装

```bash
cd nsfy/android
./gradlew assembleRelease
adb install app/build/outputs/apk/release/app-release.apk
```

首次启动会自动连接 `localhost:8419`（配合 `adb reverse tcp:8419 tcp:8419` 使用）。正式使用时在 Settings 里填你的服务器地址。

### 桌面端安装

```bash
cd nsfy/desktop
npm install
cargo tauri build
# → src-tauri/target/release/bundle/
```

macOS 上出 `.dmg`，Windows 出 `.msi`，Linux 出 `.AppImage`。

---

<!-- ═══════════════════════════════════════════ -->
<!-- 日本語 · JAPANESE                            -->
<!-- ═══════════════════════════════════════════ -->

## 日本語

### 概要

nsfyはセルフホスト型のプッシュ通知システムです。サーバーは1.2MBのRustバイナリ一つ。デスクトップとAndroidはWebSocketで常時接続し、メッセージをリアルタイム受信します。

ntfy.shとの違い：パブリックサービス不要、アカウント登録不要、Firebase非依存。自分のVPSで動かし、データは自分で管理します。

### コンポーネント

| コンポーネント | 技術 | サイズ |
|---------------|------|--------|
| サーバー | Rust + axum | 1.2 MB |
| デスクトップ | Tauri + Svelte | 2.0 MB |
| Android | Kotlin + Compose | 1.6 MB |

### サーバー起動

```bash
git clone https://github.com/VocabVictor/nsfy.git
cd nsfy/server && cargo build --release
./target/release/nsfyd --listen 0.0.0.0:8080
```

### API

| メソッド | パス | 説明 |
|---------|------|------|
| `POST /:topic` | JSON本文 | メッセージ送信 |
| `GET /:topic/ws` | WebSocket | リアルタイム購読 |
| `GET /:topic/sse` | SSE | ブラウザ向けストリーム |
| `GET /:topic/json?since=:id` | JSON | HTTPポーリング |
| `GET /` | JSON | 統計情報 |

---

<!-- ═══════════════════════════════════════════ -->
<!-- FRANÇAIS · FRENCH                            -->
<!-- ═══════════════════════════════════════════ -->

## Français

### Aperçu

nsfy est un système de notification push auto-hébergé. Le serveur tient dans un binaire Rust de 1,2 Mo. Les clients desktop et Android maintiennent une connexion WebSocket permanente pour recevoir les messages en temps réel.

Différence avec ntfy.sh : pas de service public requis, pas de compte, pas de Firebase. Vous hébergez, vous contrôlez.

### Composants

| Composant | Technologie | Taille |
|-----------|-------------|--------|
| Serveur | Rust + axum | 1,2 Mo |
| Desktop | Tauri + Svelte | 2,0 Mo |
| Android | Kotlin + Compose | 1,6 Mo |

### Démarrage rapide

```bash
git clone https://github.com/VocabVictor/nsfy.git
cd nsfy/server && cargo build --release
./target/release/nsfyd --listen 0.0.0.0:8080
```

```bash
# Publier
curl -X POST http://localhost:8080/backups \
  -H "Content-Type: application/json" \
  -d '{"title":"sauvegarde","message":"terminée","priority":4}'

# S'abonner (WebSocket)
websocat ws://localhost:8080/backups/ws

# Statistiques
curl http://localhost:8080/
```

---

<!-- ═══════════════════════════════════════════ -->
<!-- DEUTSCH · GERMAN                            -->
<!-- ═══════════════════════════════════════════ -->

## Deutsch

### Übersicht

nsfy ist ein selbst gehostetes Push-Benachrichtigungssystem. Der Server besteht aus einer einzigen 1,2 MB Rust-Binärdatei. Desktop und Android halten dauerhafte WebSocket-Verbindungen für Echtzeit-Empfang.

Unterschied zu ntfy.sh: kein öffentlicher Dienst nötig, keine Kontoregistrierung, keine Firebase-Abhängigkeit. Sie hosten selbst, Sie behalten die Kontrolle.

### Komponenten

| Komponente | Technologie | Größe |
|------------|-------------|-------|
| Server | Rust + axum | 1,2 MB |
| Desktop | Tauri + Svelte | 2,0 MB |
| Android | Kotlin + Compose | 1,6 MB |

### Schnellstart

```bash
git clone https://github.com/VocabVictor/nsfy.git
cd nsfy/server && cargo build --release
./target/release/nsfyd --listen 0.0.0.0:8080
```

```bash
# Nachricht senden
curl -X POST http://localhost:8080/alerts \
  -H "Content-Type: application/json" \
  -d '{"title":"CPU","message":"95% Auslastung","priority":5}'

# Abonnieren per WebSocket
websocat ws://localhost:8080/alerts/ws
```

---

## Project structure

```
nsfy/
├── server/          Rust backend — axum on tokio
│   └── src/
│       ├── main.rs      entry point
│       ├── config.rs    CLI args + env vars
│       ├── pubsub.rs    dashmap + broadcast channels
│       ├── handlers.rs  HTTP / WS / SSE routes
│       └── message.rs   data types
├── desktop/         Tauri app — Svelte 5 frontend
│   ├── src/             Svelte components
│   └── src-tauri/       Rust backend (ws manager, notifications)
├── android/         Kotlin + Compose, minSdk 21
│   └── app/src/main/java/com/nsfy/app/
│       ├── service/     foreground WebSocket service
│       ├── data/        Room DB + repository
│       └── ui/screens/  Compose screens
└── README.md
```

---

## License

MIT — take it, fork it, ship it, sell it. Just keep the license notice.
