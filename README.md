# EventRegister

A cross-platform personal daily tracker. Log activities in six categories — **chore,
work, emotion, thinking, writing, inventing** — and get live totals, last-logged
timestamps, and UTC-day streaks.

![stack](https://img.shields.io/badge/backend-Rust%20%2B%20Axum-orange)
![stack](https://img.shields.io/badge/web-Leptos%20WASM-blue)
![stack](https://img.shields.io/badge/database-PostgreSQL%2015-336791)
![stack](https://img.shields.io/badge/deploy-rootless%20Podman-purple)

## Current status

Milestone 1 is implemented: event logging, six-category summaries, streaks, a buildable
web client, a complete WeChat Mini Program, and a rootless container deployment.

**Authentication is not implemented yet.** The stack seeds a development demo user. Do not
expose the API to the public internet until Milestone 2 adds OAuth/OIDC and per-user
authorization. The full gap analysis is in [`docs/SPEC-REVIEW.md`](docs/SPEC-REVIEW.md).

## Architecture

```
Browser (Leptos/WASM) ──┐
                        ├── HTTPS ── Nginx ── /api ── Axum API ── SQLx ── PostgreSQL 15
WeChat Mini Program ────┘              └── static files (frontend/dist)
```

- **Backend:** Rust, Axum 0.7, Tokio, SQLx 0.7 (runtime queries), Serde, tracing.
- **Web:** Rust/WASM with Leptos 0.6 CSR, built with Trunk.
- **Mobile:** native WeChat Mini Program (WXML/WXSS/JS).
- **Database:** PostgreSQL 15 with an enum category type and JSONB metadata.
- **Edge:** Nginx TLS termination, static hosting, API reverse proxy.
- **Deploy:** rootless Podman Compose, systemd user service.

## Project layout

```
backend/            Axum API (src/main.rs, config, models, handlers, error)
frontend/           Leptos WASM app (index.html, style.css, Trunk.toml)
wechat/             WeChat Mini Program (app.json, pages/index/*)
nginx/conf.d/       TLS + static + reverse proxy configuration
systemd/            systemd user unit template (filled in by deploy.sh)
seeds/dev.sql       Development demo user
scripts/            smoke-test.sh
docs/SPEC-REVIEW.md Spec compliance and improvement plan
.vscode/            Tasks, launch configs, settings, extension recommendations
schema.sql          Idempotent schema bootstrap
deploy.sh           One-command rootless deployment
```

## Quick start (Podman)

```bash
cp .env.example .env
# edit .env: at minimum change POSTGRES_PASSWORD
./deploy.sh
```

Open <https://localhost> (accept the self-signed certificate) and try the form.

```bash
curl -fsS http://127.0.0.1:8080/health          # OK
curl -fsS http://127.0.0.1:8080/health/ready    # READY
./scripts/smoke-test.sh                         # end-to-end checks
```

`deploy.sh` builds the images, waits for PostgreSQL, applies the idempotent schema, seeds
the demo account, and installs/enables the systemd user service. It builds the web client
when `trunk` is installed; otherwise it installs a placeholder page with instructions.

## Local development

Start only the database:

```bash
podman compose up -d postgres
```

On a fresh volume the schema is applied automatically by the container entrypoint. Seed the
demo user once (the `deploy.sh` task does this for you):

```bash
podman exec -i event_register_db psql -U postgres -d event_register < seeds/dev.sql
```

Backend (API on `:8080`):

```bash
cd backend
cargo run
```

Web client (Trunk on `:3000`, proxying `/api` to `:8080`):

```bash
cd frontend
rustup target add wasm32-unknown-unknown
cargo install trunk --locked   # once
trunk serve
```

Open <http://127.0.0.1:3000>. The client defaults to the seeded demo user. To use another
UUID from the browser console:

```js
localStorage.setItem("event_register.user_id", "<uuid>");
```

To point the production build at a different API:

```bash
EVENT_REGISTER_API_BASE=https://api.example.com/api/v1 trunk build --release
```

## VS Code

Open this folder directly. Recommended extensions are listed in
`.vscode/extensions.json` (Rust Analyzer, CodeLLDB, TOML/YAML/Docker, EditorConfig,
Prettier).

- **Build default:** `Ctrl+Shift+B` → `backend: build`.
- **Tasks:** `Terminal → Run Task` for backend build/test/clippy/fmt, `frontend: serve`,
  `frontend: build`, `stack: up/down/logs/deploy`, schema/seed commands, smoke test.
- **Debug backend:** `F5` → `Backend: debug` (CodeLLDB), with the compose database running.
- **Debug web:** start `frontend: serve` first, then launch `Web: Chrome`.

## API

| Method | Path | Purpose |
| --- | --- | --- |
| `GET` | `/health` | Liveness; returns `OK`. |
| `GET` | `/health/ready` | Readiness; verifies PostgreSQL. |
| `POST` | `/api/v1/events` | Create an event (returns `201`). |
| `GET` | `/api/v1/summaries/{user_id}` | Six category summaries. |

Example:

```bash
curl -fsS -X POST http://127.0.0.1:8080/api/v1/events \
  -H 'content-type: application/json' \
  -d '{
    "user_id": "a0eebc99-9c0b-4ef8-bb6d-6bb9bd380a11",
    "category": "writing",
    "title": "Finished module draft",
    "content": "Optional notes",
    "metadata": {"mood": "focused"}
  }'
```

Errors are always `{"error": "..."}`. See [`SPEC.md`](SPEC.md) for the full contract,
limits, and acceptance criteria.

## Configuration

| Variable | Default | Used by |
| --- | --- | --- |
| `DATABASE_URL` | local compose URL | API |
| `BIND_ADDR` | `0.0.0.0:8080` | API |
| `DATABASE_MAX_CONNECTIONS` | `20` | API |
| `CORS_ALLOWED_ORIGINS` | localhost origins | API |
| `RUST_LOG` | `event_register=info,tower_http=info` | API |
| `POSTGRES_USER/PASSWORD/DB/PORT` | see `.env.example` | Compose |
| `API_PORT` | `8080` | Compose (loopback binding) |
| `SEED_DEMO` | `1` | `deploy.sh` (skip with `0`) |
| `EVENT_REGISTER_API_BASE` | `/api/v1` | Web build |

## WeChat

See [`wechat/README.md`](wechat/README.md). Import the `wechat/` folder in DevTools,
set the AppID and `config.js` `baseUrl`, and register the request domain. The client falls
back to the demo account until `/auth/wechat` exists.

## Production checklist

- [ ] Implement FR-1 (OAuth/OIDC + WeChat auth) and authorize every query by session user.
- [ ] Set `SEED_DEMO=0` and do not apply `seeds/dev.sql` in production.
- [ ] Replace the self-signed certificates with real ones and enable HSTS in
      `nginx/conf.d/event_register.conf`.
- [ ] Set a strong `POSTGRES_PASSWORD`; do not commit `.env`.
- [ ] Restrict `CORS_ALLOWED_ORIGINS` to your real origins.
- [ ] Keep the committed `Cargo.lock` files and add CI (`cargo test`, `clippy`, `trunk build`).
- [ ] Add rate limiting and a request body limit.
- [ ] Back up the `pgdata` volume.

## Troubleshooting

- **`first event returns 400 unknown user_id`** — the demo seed was skipped. Run
  `SEED_DEMO=1 ./deploy.sh` or apply `seeds/dev.sql`.
- **`https://localhost` shows a placeholder** — `trunk` was not installed when
  `deploy.sh` ran. Run `cd frontend && trunk build --release` and reload.
- **API container restarts** — check `podman compose logs api`; usually the database
  password in `.env` no longer matches the existing `pgdata` volume.
- **`podman exec: input device is not a TTY`** — do not use `-it` in scripts; the updated
  `deploy.sh` uses `-i`.

## Legacy scaffold generator

The root-level `g` script was the original one-shot generator. It is **not** the source of
truth anymore and would overwrite the improved files. Do not run it; it is kept only for
reference.
