<p align="center">
  <a href="README.zh-CN.md">中文</a> | <a href="README.md">English</a> | <a href="README.ja.md">日本語</a> | <a href="README.fr.md">Français</a> | <a href="README.de.md">Deutsch</a>
</p>

<p align="center">
  <img src="https://img.shields.io/badge/license-MIT-blue" />
  <img src="https://img.shields.io/badge/server-Rust-f74c00?logo=rust" />
  <img src="https://img.shields.io/badge/desktop-Tauri-24c8a0?logo=tauri" />
  <img src="https://img.shields.io/badge/android-Kotlin-7f52ff?logo=kotlin" />
  <img src="https://img.shields.io/badge/minSdk-21-green?logo=android" />
  <img src="https://img.shields.io/badge/taille-1.2MB_serveur_|_1.6MB_apk-lightgrey" />
</p>

---

# nsfy

Un système de notification push sans compte cloud. Le serveur tient dans un seul binaire Rust. Desktop et Android maintiennent une connexion WebSocket permanente.

Différence avec ntfy.sh : pas de service public, pas de compte, pas de Firebase. Vous hébergez, vous contrôlez.

## Comment ça marche

```
curl -d '{"title":"backup","message":"done"}'  \
     nsfyd:8080/backups                          POST /<topic>
                                                       │
┌──────────────────── nsfyd ────────────────────┐      │
│  axum + dashmap + broadcast channels           │      │
│  1.2 MB binaire · ~7 MB idle · 10k connexions  │      │
└──────────────────────┬─────────────────────────┘      │
                       │ ws://<topic>/ws                │
         ┌─────────────┼─────────────┐                  │
         ▼             ▼             ▼                  │
   ┌──────────┐ ┌──────────┐ ┌──────────┐              │
   │ Desktop  │ │ Android  │ │  curl    │              │
   │ 2.0 MB   │ │ 1.6 MB   │ │  0 MB    │              │
   └──────────┘ └──────────┘ └──────────┘              │
```

## Démarrage rapide

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

## API

| Méthode | Chemin | Description |
|---------|--------|-------------|
| `POST /:topic` | Corps JSON | Envoyer un message |
| `GET /:topic/ws` | WebSocket | Abonnement temps réel |
| `GET /:topic/sse` | SSE | Flux navigateur |
| `GET /:topic/json?since=:id` | JSON | Polling HTTP |
| `GET /` | JSON | Statistiques globales |

## Composants

| Composant | Technologie | Taille |
|-----------|-------------|--------|
| Serveur | Rust + axum | 1,2 Mo |
| Desktop | Tauri + Svelte | 2,0 Mo |
| Android | Kotlin + Compose | 1,6 Mo |

## License

MIT
