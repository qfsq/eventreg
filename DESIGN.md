# System Design & Data Flow

## 1. Components

```
Browser (Leptos/WASM) ──┐
                        ├── HTTPS ── Nginx ── /api ── Axum API ── SQLx ── PostgreSQL
WeChat Mini Program ────┘              └── static files (frontend/dist)
```

- **API (`backend/`)** — Axum 0.7, Tokio, SQLx 0.7 (runtime queries, no compile-time DB
  needed), Serde, tracing.
- **Web (`frontend/`)** — Rust/WASM with Leptos 0.6 CSR, built by Trunk. Defaults to
  same-origin `/api/v1`; Trunk proxies `/api` during development.
- **Mobile (`wechat/`)** — native Mini Program calling the same REST API.
- **Database** — PostgreSQL 15; enum-typed categories and `JSONB` metadata.
- **Edge** — Nginx terminates TLS, serves static assets, proxies API/health routes.

## 2. Backend Module Layout

| File | Responsibility |
| --- | --- |
| `src/main.rs` | Bootstrap, tracing, pool, router, CORS, graceful shutdown. |
| `src/config.rs` | Environment parsing and defaults. |
| `src/error.rs` | `ApiError`, JSON error responses, internal-error logging. |
| `src/models.rs` | Category enum, request validation, response rows, unit tests. |
| `src/handlers.rs` | Event creation, summaries, health/readiness. |

Handlers only ever see validated input. Database failures are converted to `ApiError`
which logs the SQLx error and returns a generic message.

## 3. Database Schema

- `users` — unique `(provider, provider_id)`; `email` optional but unique.
- `events` — `user_id` cascades on delete; `(user_id, logged_at DESC)` and
  `(user_id, category, logged_at DESC)` indexes; title check constraints.
- `user_summaries` — primary key `(user_id, category)`; `total_count >= 0`,
  `streak_days >= 0`; `updated_at` for change tracking.
- `schema.sql` is idempotent and includes an upgrade `ALTER TABLE` for databases created
  by the original scaffold.

## 4. Event Write Flow

```
POST /api/v1/events
  ├─ validate + trim payload (title, content, metadata size)
  ├─ BEGIN
  ├─ INSERT INTO events ... RETURNING ...
  │    └─ FK violation (SQLSTATE 23503) -> 400 "unknown user_id"
  ├─ UPSERT user_summaries (atomic increment, streak, last timestamp)
  └─ COMMIT
```

The UPSERT is a single statement, so concurrent writers serialize on the summary row and
no application-level lock is required.

### Streak Rules (UTC)

| Previous `last_logged_at` | New `streak_days` |
| --- | --- |
| `NULL` (first event) | `1` |
| Same UTC date | unchanged |
| Previous UTC date | `+1` |
| Older than that | reset to `1` |

`total_count` always increments. `last_logged_at` uses `GREATEST` so an out-of-order
timestamp can never move it backwards.

## 5. Summary Read Flow

```
GET /api/v1/summaries/{user_id}
  ├─ SELECT EXISTS(users) -> 404 if absent
  └─ unnest(enum_range(event_category)) LEFT JOIN user_summaries
       -> exactly six rows, ordered by enum definition order
```

The left join keeps every client dashboard stable before the first event exists.

## 6. Configuration

| Variable | Default | Notes |
| --- | --- | --- |
| `DATABASE_URL` | local compose URL | SQLx connection string. |
| `BIND_ADDR` | `0.0.0.0:8080` | API listen address. |
| `DATABASE_MAX_CONNECTIONS` | `20` | Pool size. |
| `CORS_ALLOWED_ORIGINS` | localhost origins | Comma-separated; `*` enables any origin (not for production). |
| `RUST_LOG` | `event_register=info,tower_http=info` | `tracing` filter. |
| `EVENT_REGISTER_API_BASE` (build-time) | `/api/v1` | Web client API base override. |

## 7. Deployment Topology

- `postgres` — health-checked; the schema runs on an empty volume, and `deploy.sh`
  reapplies it idempotently on every deploy.
- `api` — waits for a healthy database, runs as UID 10001, has a `/health/ready` check.
- `nginx` — waits for a healthy API; serves `frontend/dist`; redirects HTTP to HTTPS.
- Host ports bind to loopback for PostgreSQL and the API; only 80/443 are public.
- `deploy.sh` optionally seeds demo data and installs a systemd **user** unit templated with
  the real project path and compose binary.

## 8. Security Decisions

- TLS 1.2/1.3, modern cipher set, CSP allowing `wasm-unsafe-eval` for the WASM client.
- CORS is an explicit allow-list (no wildcard by default).
- API never returns SQL errors or connection strings.
- Runtime container is non-root and includes only CA certificates and curl.
- HSTS is commented out until the bootstrap self-signed certificate is replaced.
- **Known gap:** authentication/authorization (FR-1). Until it is implemented, anyone who
  can reach the API can act as any known `user_id`. Do not expose this stack publicly.

## 9. Trade-offs and Future Work

- Runtime SQLx queries trade compile-time SQL checking for a build that does not need a
  live database. `sqlx::query!` can be restored with committed `.sqlx` offline data.
- Summaries are eagerly aggregated for O(1) dashboard reads; event history queries are
  out of scope for Milestone 1.
- A single API instance is assumed. The atomic UPSERT already supports multiple instances.
- Authentication, rate limiting, and an events list endpoint are the next milestones.
