# nsfy — Claude Code instructions

## Git rules — read first

- Author: `nsfy <VocabVictor@users.noreply.github.com>` on every commit
- No `Co-Authored-By` lines. No `Claude`, no `noreply@anthropic.com`. Ever.
- Commit messages in English, imperative mood, lowercased first word
- Push only when the user says so, or after CI config changes that need triggering

## Project structure

```
server/          Rust — axum, dashmap, broadcast
desktop/         Tauri 2 — Svelte 5 frontend + Rust backend
android/         Kotlin — Compose, Room, OkHttp WS
```

## Build

```bash
# Server
cd server && cargo build --release         # → target/release/nsfyd

# Desktop (macOS)
cd desktop && npm ci && cargo tauri build  # → src-tauri/target/release/bundle/

# Android
cd android && ./gradlew assembleRelease    # → app/build/outputs/apk/release/
```

## Code rules

### All platforms
- No emoji in source code, README except language flags where sensible
- No AI filler words: unlock, unleash, supercharge, harness, leverage, delve, tapestry
- One sentence per log line. No exclamation marks
- Concrete names. `connectWs` not `establishWebSocketConnection`

### Rust server
- Stay on stable Rust. No nightly features
- `cargo clippy` before commit. Zero warnings
- `cargo fmt` — default rustfmt config

### Desktop (Tauri + Svelte)
- Svelte 5 runes only. No Svelte 4 stores
- Store files use `writable`/`readable` from `svelte/store`
- `$` prefix for store auto-subscription in components
- `npm run build` must pass before Tauri build

### Android
- minSdk 21. Test on API 21 emulator before tagging
- No Material Icons Extended. Unicode text symbols instead
- Keep APK under 2 MB release. Check with `ls -lh` before commit
- Room over raw SQLite. DAO methods are suspend functions
- OkHttp WebSocket in foreground service. Reconnect with 5s delay

## Before tagging a release

1. `cargo clippy` in server/ — zero warnings
2. `npm run build` in desktop/ — zero errors
3. `./gradlew assembleRelease` in android/ — BUILD SUCCESSFUL
4. Test Android APK on device via adb
5. `git tag -a vX.Y.Z -m "nsfy vX.Y.Z"`
6. `git push origin main --tags`
