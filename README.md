# ConsumeView

Local **Tauri 2** desktop app that sits between Cursor and Gemini / Grok: an OpenAI-compatible proxy on `127.0.0.1:4000`, SQLite token metrics, and a monitoring panel (session, overview, settings, system tray).

## Why

Cursor can override the OpenAI base URL. Point it at ConsumeView so requests are routed to Google (Gemini) or xAI (Grok) by model prefix, while costs and latency stay on your machine.

## Architecture

| Layer | Location | Responsibility |
| --- | --- | --- |
| Domain | `src-tauri/src/domain/` | Prefix routing, pricing, aggregates (no I/O) |
| Infra | `proxy/`, `db/`, `secrets/` | Axum proxy, SQLite WAL, OS keyring, `token-logged` events |
| UI | `src/` | Presentation only — no pricing/routing logic |

## Quick start

```bash
pnpm install
pnpm tauri dev
```

Requirements: [Tauri Linux prerequisites](https://tauri.app/start/prerequisites/) (webkit2gtk, etc.).

Useful checks without a full desktop build:

```bash
pnpm build                 # frontend
cd src-tauri && cargo test # domain unit tests
cd src-tauri && cargo check
```

## Cursor setup

1. Start ConsumeView (proxy listens on **`http://127.0.0.1:4000`**).
2. In **Settings → Models** (or OpenAI-compatible provider settings):
   - **Override OpenAI Base URL:** `http://localhost:4000/v1`
   - **API Key:** any non-empty placeholder (e.g. `consumeview-local`) — real keys live only in ConsumeView
   - **Models:** use names with prefixes:
     - `gemini-2.0-flash`, `gemini-1.5-pro`, `gemini-1.5-flash`, …
     - `grok-2`, `grok-2-mini`, …
3. In ConsumeView **Settings**, paste your real **Gemini** and **xAI** keys (stored in the OS keyring).
4. Pick or create an **active session** before generating — traffic is attributed to that session.

### Routing

| Model prefix | Upstream |
| --- | --- |
| `grok-*` | `https://api.x.ai/v1/chat/completions` |
| `gemini-*` | `https://generativelanguage.googleapis.com/v1beta/openai/chat/completions` |

## Proxy API

- `GET /health` — liveness
- `POST /v1/chat/completions` — OpenAI-compatible chat (stream supported)

Bind address is **localhost only** (`127.0.0.1:4000`).

## Tray

Closing the window hides ConsumeView to the system tray; the proxy keeps running. Use the tray menu to show the window again or quit.

## Security

- Real API keys never go to Cursor — only into ConsumeView’s keyring (or a local `0600` fallback file if the keyring service is unavailable).
- Metrics database: app data dir `ConsumeView/metrics.db` (SQLite WAL).

## License

MIT
