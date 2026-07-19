# Server test suites

The server tests are split by responsibility so no source file becomes a
catch-all test dump.

- `http_protocol.rs`: publish, poll, replay, validation, and HTTP auth.
- `stream_protocol.rs`: WebSocket authentication/read-only behavior and SSE.
- `persistence.rs`: durable acknowledgement, restart recovery, migration, WAL.
- `network_security.rs`: TLS, certificate validation, origin policy, fail-closed startup.
- `topic_permissions.rs`: independent Topic read/write capabilities and admin override.
- `performance_http.rs`: durable publish payload/concurrency/topic matrix and replay latency.
- `performance_stream.rs`: 10k WebSocket capacity, memory, WS/SSE fan-out, and retained data.
- `performance_tls.rs`: certificate-validated HTTPS publish latency.
- `unit/`: private module tests, referenced from production modules only in test builds.
- `common/mod.rs`: isolated server process fixture used by protocol suites.

Run correctness tests with `cargo test`. Run the explicit performance baseline
with `server/scripts/run-performance.ps1`. It runs the full matrix serially and
writes a Git-ignored report under `.performance-results/`; pass `-Quick` only
for development smoke checks.
