<p align="center">
  <a href="README.zh-CN.md">中文</a> | <a href="README.md">English</a> | <a href="README.ja.md">日本語</a> | <a href="README.fr.md">Français</a> | <a href="README.de.md">Deutsch</a>
</p>

<p align="center">
  <img src="https://img.shields.io/badge/license-MIT-blue" />
  <img src="https://img.shields.io/badge/server-Rust-f74c00?logo=rust" />
  <img src="https://img.shields.io/badge/desktop-Tauri-24c8a0?logo=tauri" />
  <img src="https://img.shields.io/badge/android-Kotlin-7f52ff?logo=kotlin" />
  <img src="https://img.shields.io/badge/minSdk-21-green?logo=android" />
  <img src="https://img.shields.io/badge/体积-1.2MB_服务端_|_1.6MB_APK-lightgrey" />
</p>

---

# nsfy

一个不需要云账号的推送通知系统。服务端是一个 Rust 二进制文件，跑在任意 VPS 上。桌面端和 Android 端通过 WebSocket 长连接实时接收消息。

跟 ntfy.sh 的区别：不需要公网服务，不需要注册账号，不依赖 Firebase。你自己跑服务端，自己控制数据。

## 工作原理

```
curl -d '{"title":"备份","message":"完成"}'  \
     nsfyd:8080/backups                          POST /<topic>
                                                       │
┌──────────────────── nsfyd ────────────────────┐      │
│  axum + dashmap + broadcast 通道               │      │
│  1.2 MB 二进制 · 空闲 ~7 MB · 10k 并发连接     │      │
└──────────────────────┬─────────────────────────┘      │
                       │ ws://<topic>/ws                │
         ┌─────────────┼─────────────┐                  │
         ▼             ▼             ▼                  │
   ┌──────────┐ ┌──────────┐ ┌──────────┐              │
   │ 桌面端   │ │ Android  │ │  curl    │              │
   │ Tauri    │ │ Kotlin   │ │  SSE     │              │
   │ 2.0 MB   │ │ 1.6 MB   │ │  0 MB    │              │
   └──────────┘ └──────────┘ └──────────┘              │
```

一个 topic，三个订阅者，消息同时到达。

## 快速开始

### 服务端

```bash
git clone https://github.com/VocabVictor/nsfy.git
cd nsfy/server
cargo build --release

./target/release/nsfyd --listen 0.0.0.0:8080
```

systemd 配置：

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

### 发送消息

```bash
curl -X POST http://localhost:8080/alerts \
  -H "Content-Type: application/json" \
  -d '{"title":"磁盘","message":"用了90%","priority":5,"tags":["server"]}'
```

### 订阅消息

```bash
# WebSocket — 实时双向
websocat ws://localhost:8080/alerts/ws

# SSE — 浏览器友好
curl http://localhost:8080/alerts/sse

# HTTP 轮询 — 无需长连接
curl http://localhost:8080/alerts/json
```

### 查看统计

```bash
curl http://localhost:8080/
# → {"topics":3,"total_subscribers":12,"topic_names":["alerts","backups","chat"]}
```

## API

| 方法 | 路径 | 说明 |
|--------|------|------|
| `POST /:topic` | JSON body | 发送消息 |
| `GET /:topic/ws` | WebSocket 升级 | 实时订阅 |
| `GET /:topic/sse` | text/event-stream | SSE 流 |
| `GET /:topic/json?since=:id` | JSON 数组 | 从指定 ID 之后拉取 |
| `GET /` | JSON | 全局统计 |

消息格式：

```json
{
  "id": "01J...",
  "time": 1718832000,
  "title": "备份完成",
  "message": "数据库备份完成，2.3GB",
  "priority": 3,
  "tags": ["backup", "db"]
}
```

优先级 1（最低）到 5（最高）。tags 是字符串数组，客户端自行过滤。

## 三端

| 端 | 技术 | 体积 | 说明 |
|----|------|------|------|
| 服务端 | Rust + axum | 1.2 MB | 单二进制，systemd 一行配 |
| 桌面端 | Tauri + Svelte | 2.0 MB | 系统托盘常驻，原生通知 |
| Android | Kotlin + Compose | 1.6 MB | minSdk 21，从 Android 5.0 开始全兼容 |

## Android 安装

```bash
cd nsfy/android
./gradlew assembleRelease
adb install app/build/outputs/apk/release/app-release.apk
```

首次启动自动连接 `localhost:8419`（配合 `adb reverse tcp:8419 tcp:8419`）。正式使用在 Settings 里填写服务器地址。

## 桌面端安装

```bash
cd nsfy/desktop
npm install
cargo tauri build
# → src-tauri/target/release/bundle/
```

macOS 出 `.dmg`，Windows 出 `.msi`，Linux 出 `.AppImage`。

## 目录结构

```
nsfy/
├── server/          Rust 服务端
│   └── src/         main / config / pubsub / handlers / message
├── desktop/         Tauri 桌面端
│   ├── src/         Svelte 前端组件
│   └── src-tauri/   Rust 后端（WS 管理、通知）
├── android/         Kotlin + Compose，minSdk 21
│   └── app/         service / data / ui
└── README*.md
```

## License

MIT — 随便用，随便改，随便卖。保留 license 声明即可。
