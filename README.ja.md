<p align="center">
  <a href="README.zh-CN.md">中文</a> | <a href="README.md">English</a> | <a href="README.ja.md">日本語</a> | <a href="README.fr.md">Français</a> | <a href="README.de.md">Deutsch</a>
</p>

<p align="center">
  <img src="https://img.shields.io/badge/license-MIT-blue" />
  <img src="https://img.shields.io/badge/server-Rust-f74c00?logo=rust" />
  <img src="https://img.shields.io/badge/desktop-Tauri-24c8a0?logo=tauri" />
  <img src="https://img.shields.io/badge/android-Kotlin-7f52ff?logo=kotlin" />
  <img src="https://img.shields.io/badge/minSdk-21-green?logo=android" />
  <img src="https://img.shields.io/badge/サイズ-1.2MB_server_|_1.6MB_apk-lightgrey" />
</p>

---

# nsfy

クラウドアカウント不要のプッシュ通知システム。サーバーはRustのバイナリ一つで、任意のVPSで動作。デスクトップとAndroidはWebSocketで常時接続し、メッセージをリアルタイム受信します。

ntfy.shとの違い：パブリックサービス不要、アカウント登録不要、Firebase非依存。自分のサーバーで動かし、データは自分で管理。

## 仕組み

```
curl -d '{"title":"backup","message":"done"}'  \
     nsfyd:8080/backups                          POST /<topic>
                                                       │
┌──────────────────── nsfyd ────────────────────┐      │
│  axum + dashmap + broadcast チャンネル         │      │
│  1.2 MB バイナリ · アイドル ~7 MB · 10k 接続  │      │
└──────────────────────┬─────────────────────────┘      │
                       │ ws://<topic>/ws                │
         ┌─────────────┼─────────────┐                  │
         ▼             ▼             ▼                  │
   ┌──────────┐ ┌──────────┐ ┌──────────┐              │
   │ Desktop  │ │ Android  │ │  curl    │              │
   │ 2.0 MB   │ │ 1.6 MB   │ │  0 MB    │              │
   └──────────┘ └──────────┘ └──────────┘              │
```

## クイックスタート

```bash
git clone https://github.com/VocabVictor/nsfy.git
cd nsfy/server && cargo build --release
./target/release/nsfyd --listen 0.0.0.0:8080
```

```bash
# メッセージ送信
curl -X POST http://localhost:8080/alerts \
  -H "Content-Type: application/json" \
  -d '{"title":"CPU","message":"95%使用率","priority":5}'

# WebSocketで購読
websocat ws://localhost:8080/alerts/ws

# 統計
curl http://localhost:8080/
```

## API

| メソッド | パス | 説明 |
|---------|------|------|
| `POST /:topic` | JSON本文 | メッセージ送信 |
| `GET /:topic/ws` | WebSocket | リアルタイム購読 |
| `GET /:topic/sse` | SSE | ブラウザ向けストリーム |
| `GET /:topic/json?since=:id` | JSON配列 | HTTPポーリング |
| `GET /` | JSON | 統計情報 |

## コンポーネント

| コンポーネント | 技術 | サイズ |
|---------------|------|--------|
| サーバー | Rust + axum | 1.2 MB |
| デスクトップ | Tauri + Svelte | 2.0 MB |
| Android | Kotlin + Compose | 1.6 MB |

## License

MIT
