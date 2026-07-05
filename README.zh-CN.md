<p align="center">
  <a href="README.zh-CN.md">中文</a> | <a href="README.md">English</a> | <a href="README.ja.md">日本語</a> | <a href="README.fr.md">Français</a> | <a href="README.de.md">Deutsch</a>
</p>

<p align="center">
  <img src="https://img.shields.io/badge/license-MIT-blue" />
  <img src="https://img.shields.io/badge/server-Rust-f74c00?logo=rust" />
  <img src="https://img.shields.io/badge/desktop-Tauri-24c8a0?logo=tauri" />
  <img src="https://img.shields.io/badge/android-Kotlin-7f52ff?logo=kotlin" />
  <img src="https://img.shields.io/badge/minSdk-21-green?logo=android" />
  <img src="https://img.shields.io/badge/体积-3.1MB_服务端_|_1.6MB_APK-lightgrey" />
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
│  3.1 MB 二进制 · 空闲 ~7 MB · 10k 并发连接     │      │
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

## 安全

```bash
./target/release/nsfyd \
  --listen 0.0.0.0:8080 \
  --auth-token "$(openssl rand -hex 32)" \
  --rate-limit-per-min 300 \
  --max-topics 10000
```

| 参数 / 环境变量 | 默认值 | 作用 |
|---|---|---|
| `--auth-token` / `NSFY_AUTH_TOKEN` | 无 | 设置后，所有路由（包括 `/`）都需要鉴权。推荐用 `Authorization: Bearer <token>`（不会泄露到访问日志里），也支持 `?auth=<token>`。比较时使用常数时间算法，防止通过响应耗时猜出 token。 |
| `--rate-limit-per-min` / `NSFY_RATE_LIMIT_PER_MIN` | 300 | 按客户端 IP 做令牌桶限流，HTTP 请求和 WebSocket 内发布共享同一配额，超限返回 `429`。 |
| `--max-topics` / `NSFY_MAX_TOPICS` | 10000 | 服务端同时跟踪的 topic 总数上限，防止未鉴权调用者无限创建 topic 撑爆内存。超限时新 topic 名会返回 `503`。 |
| `--max-msg-size` / `NSFY_MAX_MSG_SIZE` | 65536 | 同时作用于原始 HTTP body 大小和 `message` 字段长度（也约束 WS 帧/消息大小）。`title` 上限 512 字节，tags 最多 32 个、每个 64 字节。 |
| `--topic-rate-limit-per-min` / `NSFY_TOPIC_RATE_LIMIT_PER_MIN` | 1200 | 单个 topic 跨所有 IP 的聚合发布配额，用来防住"每个 IP 都守规矩，但联合起来灌爆同一个 topic"的分布式攻击。 |
| `--max-conns-per-ip` / `NSFY_MAX_CONNS_PER_IP` | 20 | 单 IP 允许的并发 WS/SSE 连接数。每个连接在限流器里只算一次请求，这个参数单独限制"占着连接不放"的开销。 |
| `--max-conns-total` / `NSFY_MAX_CONNS_TOTAL` | 10000 | 服务端全局允许的并发 WS/SSE 连接数。 |
| `--trust-proxy` / `NSFY_TRUST_PROXY` | false | 限流时用 `X-Forwarded-For` / `X-Real-IP` 作为客户端 IP，而不是 TCP 对端地址。**只有在你自己控制、会覆写这些头的反向代理后面才能开**，原因见下面 TLS 部分。 |
| `--bandwidth-limit-per-min` / `NSFY_BANDWIDTH_LIMIT_PER_MIN` | 10000000（字节） | 按 IP 的发布带宽配额，单位是消息字节数而不是请求数——几条拉满大小的消息就能占满这个配额，不会被"只看请求次数"的限流漏掉。 |
| `--topic-creation-limit-per-min` / `NSFY_TOPIC_CREATION_LIMIT_PER_MIN` | 20 | 单 IP**创建全新 topic**的配额（发布到已存在的 topic 不消耗这个配额）。跟 `--max-topics` 是互补关系：那个是硬上限，这个管的是单个 IP 能以多快的速度去消耗剩余的额度。 |

topic 名只允许 `[A-Za-z0-9._-]`，最长 128 字符——足够覆盖正常用法，同时避免控制字符混进日志（日志注入）。不合法的名字会返回 `400`。

如果不设置 `--auth-token`，topic 名本身就是你唯一的"密码"——建议用不可猜测的随机字符串（比如 `curl .../$(openssl rand -hex 16)`），这和 ntfy.sh 的模型一致。一旦设置了 token，`/` 统计接口也会一并被保护，外部调用者就无法枚举出 topic 名单了。

单个请求处理过程中的 panic 会被捕获并转成 `500`，不会把整个进程（以及其他所有客户端的连接）一起拖垮。

## 持久化

nsfyd 默认是纯内存的：进程一重启，所有 topic 的消息缓存就没了。对大多数场景这没问题（订阅者通常只关心*新*消息），但如果你想要重启后历史消息还在，直接指定一个数据库文件就行：

```bash
./target/release/nsfyd --db-path /var/lib/nsfy/nsfy.db
```

SQLite 支持已经内置在每个二进制里（体积从 1.2MB 涨到 3.1MB 主要就是这个原因）——不设置 `--db-path` 就是纯内存,不需要单独编译一个版本。

| 参数 / 环境变量 | 默认值 | 作用 |
|---|---|---|
| `--db-path` / `NSFY_DB_PATH` | 无 | 持久化消息用的 SQLite 文件路径。不设置 = 纯内存（今天的行为）。 |
| `--db-keep-per-topic` / `NSFY_DB_KEEP_PER_TOPIC` | 等于 `--cache-size` | 数据库里**每个 topic** 保留的消息条数——跟内存缓存一样是环形缓冲区语义，只是落到磁盘上。这是硬上限，不是"永久保留全部历史"：每次插入都会在同一个事务里把该 topic 裁剪回这个数量。 |

**不会无限保留历史消息。** 磁盘上总行数的最坏情况上限是 `--max-topics × --db-keep-per-topic`（默认 10000 × 100 = 1,000,000 行）——如果你要落盘到小磁盘上，按你的实际流量去调这两个参数，而不是直接用默认值。每个 topic 是独立裁剪的，某一个 topic 很活跃也不会挤占其他 topic 的保留额度。

启动时，如果设置了 `--db-path`，nsfyd 会在开始接受连接之前，把每个 topic 最近的消息重新加载回内存——所以重启后重放出来的内容，跟一个全新订阅者通过 `/:topic/json`、`/:topic/ws`、`/:topic/sse` 能看到的一模一样，不多不少。

### TLS

`nsfyd` 本身只说明文 HTTP/WS，没有内置 TLS——这是为了控制体积做的取舍。只要流量会经过你不完全控制的网络，就在前面套一层反向代理（nginx、Caddy、[Tailscale Serve](https://tailscale.com/kb/1312/serve)），否则 token 和所有消息内容都是明文传输。

如果套了反向代理，记得加上 `--trust-proxy`，让限流认真实客户端 IP 而不是代理自己的 IP——但前提是 `nsfyd` 不能同时也被直接暴露访问。如果能被绕过代理直接访问，客户端可以自己伪造 `X-Forwarded-For`，直接绕开限流。

## 三端

| 端 | 技术 | 体积 | 说明 |
|----|------|------|------|
| 服务端 | Rust + axum | 3.1 MB | 单二进制，systemd 一行配，内置 SQLite 持久化 |
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
