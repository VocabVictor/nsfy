# NSFY performance report

This report records the full release-mode suite, not a small smoke test.

## Environment

- Date: 2026-07-19
- OS: Windows NT 10.0.26200.0
- Logical CPUs: 20
- Transport: loopback, except that the HTTPS case still performs a real TLS handshake and certificate validation
- Persistence: SQLite WAL with `synchronous=FULL`; publish acknowledgement follows transaction commit
- Execution: serial test cases (`--test-threads=1`) to avoid benchmark interference

## Results

| Workload | Scale | Result |
|---|---:|---:|
| Durable publish, 1 worker, hot Topic | 1,200 | 289 msg/s; p99 6.58 ms |
| Durable publish, 4 workers, hot Topic | 1,200 | 585 msg/s; p99 20.21 ms |
| Durable publish, 16 workers, hot Topic | 1,200 | 2,249 msg/s; p99 15.40 ms |
| Durable publish, 16 workers, 64 Topics | 1,200 | 2,101 msg/s; p99 18.77 ms |
| Durable publish, 64-byte payload | 800 | 629 msg/s; p99 14.23 ms |
| Durable publish, 1 KiB payload | 800 | 630 msg/s; p99 14.44 ms |
| Durable publish, 32 KiB payload | 200 | 465 msg/s; p99 27.47 ms |
| HTTPS durable publish with trusted certificate | 1,000 | 302 msg/s; p99 4.95 ms |
| Poll and decode a 100-message replay | 1,000 polls | 2,171 polls/s; p99 0.66 ms |
| WebSocket capacity, one Topic | 10,000 connections | 6.210 s to connect; 205 MiB server RSS |
| Fan-out across all capacity connections | 10,000 deliveries | 0.488 s; 20,487 deliveries/s |
| WebSocket fan-out stream | 250 × 25 | 44,401 deliveries/s |
| SSE first-message fan-out | 100 subscribers | 28.11 ms until all completed |
| Distinct-Topic WebSockets | 1,500 | 52 MiB RSS; 30.0 KiB per Topic+connection |
| Retained 4 KiB messages | 5,000 across 50 Topics | 24 MiB RSS increase |

The full run performed more than 12,000 durable SQLite writes in addition to
connection, replay, TLS, and fan-out workloads.

## Persistence and Topic-memory optimization

The SQLite writer now drains up to 64 already-queued concurrent publishes into
one transaction. Every caller waits for that transaction's real
`synchronous=FULL` commit, so successful-response durability did not change.
Compared with the 2026-07-18 baseline, 16-worker hot-Topic throughput rose from
319 to 2,249 msg/s. The 64-Topic case rose from 321 to 2,101 msg/s while its P99
fell from 119.80 to 18.77 ms. A serial caller still performs one durable commit
per acknowledgement and therefore remains near 300 msg/s; weakening SQLite
durability solely to improve that number was deliberately rejected.

The default live stream buffer is now 256 messages instead of 1,024 and can be
adjusted with `--stream-buffer-size`. The retained replay cache remains
independent and unchanged. Distinct-Topic WebSocket memory fell from 53.8 to
30.0 KiB per Topic+connection, a 44% reduction, while the 10,000-client
same-Topic capacity test still delivered to every connection.

## Memory correction

The first 10,000-connection measurement consumed about 1.4 GiB because the
WebSocket library allocated its default 128 KiB read buffer per connection.
NSFY now starts each read and write buffer at 4 KiB while retaining the same
message-size limit and allowing buffers to grow when required. The repeated
full-capacity run used 205 MiB, about 20.1 KiB per connection, and delivered a
message successfully to every connection.

## Reproduction

```powershell
.\server\scripts\run-performance.ps1
```

The command runs payload size, concurrency, Topic topology, polling, HTTPS,
WebSocket capacity, WebSocket/SSE fan-out, retained-memory, and process-memory
tests. Reports are written to the Git-ignored `.performance-results` directory.
`-Quick` exists only for development and must not be used for release claims.

These numbers are a reproducible local baseline, not a WAN latency promise.
Release validation should repeat the same full suite on every target server.
