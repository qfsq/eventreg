# Spec Review — EventRegister

**Date:** 2026-10-25  
**Reviewed:** the original 11-line `SPEC.md` against `backend/`, `frontend/`, `wechat/`,
`schema.sql`, and the deployment files.

## 1. Verdict

The data model and the atomic UPSERT idea were sound, but the original scaffold could not
run end to end. Three defects were release blockers:

1. The event handler did not compile (`payload.category as EventCategory` is an invalid
   non-primitive cast).
2. `sqlx::query!` compile-time macros required `DATABASE_URL` at build time, so the
   container image could not build without a database or a committed `.sqlx` cache.
3. Clients used a hard-coded demo UUID, but `users` was never seeded, so the first event
   always failed the foreign key constraint.

The spec also promised streaks and authentication that the code did not implement.

## 2. Requirement Matrix (after the changes in this pass)

| Requirement | Status | Notes |
| --- | --- | --- |
| FR-1 OAuth/OIDC + WeChat login, per-user authorization | **Missing** | Milestone 2. The demo user is a development-only workaround. Do not expose the API publicly. |
| FR-2 Six strict categories | **Implemented** | PostgreSQL enum + Serde/SQLx lowercase mapping; unknown values return `400` from Serde. |
| FR-3 Atomic event + summary write | **Implemented** | One transaction; summary increment and event insert commit together. |
| FR-4 Validation and limits | **Implemented** | Trimmed title/content, length limits, metadata object/size checks. |
| FR-5 Six summaries, zero rows included, `404` unknown user | **Implemented** | `unnest(enum_range(...)) LEFT JOIN`. |
| FR-6 Summary fields | **Implemented** | `total_count`, `last_logged_at`, `streak_days`, `aggregate_metrics`. |
| FR-7 UTC streak rules | **Implemented** | Same day = unchanged, next day = `+1`, gap = reset to `1`; verified in SQL. |
| FR-8 Web + WeChat parity | **Implemented** | Web has validation/loading/error states; WeChat now has `app.json`, WXML, WXSS, and config. |
| FR-9 Health/readiness | **Implemented** | `/health` returns `OK`; `/health/ready` checks PostgreSQL. |
| FR-10 JSON errors, no internal leakage | **Implemented** | `ApiError` logs details and returns `{"error": ...}`. |
| NFR-1 p95 < 50 ms | **Unverified** | No benchmark run in this environment; indexes and single-row UPSERT are in place. |
| NFR-2 50 concurrent writes = 50 | **Implemented by design** | Concurrency command below; automate in CI next. |
| NFR-3 TLS/CORS/non-root/no secrets | **Partial** | TLS, CORS allow-list, non-root container done; authentication still missing. |
| NFR-4 Restart, health gating, graceful shutdown | **Implemented** | Compose healthchecks, `depends_on: condition`, SIGTERM handler. |
| NFR-5 Idempotent schema, env config | **Implemented** | `schema.sql` can be applied repeatedly; demo seed is opt-out. |
| NFR-6 amd64 + arm64 | **Unverified** | Multi-arch upstream images; only amd64 exercised here. |
| NFR-7 Browser + WeChat compatibility | **Partial** | Web build depends on Trunk/wasm32; WeChat client not run in DevTools here. |

## 3. Defects Found and Fixed

| Area | Original problem | Fix |
| --- | --- | --- |
| Compile | `payload.category as EventCategory` | Bind the enum value directly. |
| Compile | `sqlx::query!` needs `DATABASE_URL`/`.sqlx` | Switched to runtime `query_as` with `FromRow`. |
| Compile | `EventResponse` lacked `FromRow` | Added `sqlx::FromRow`. |
| Runtime | No seeded user, FK violation | `seeds/dev.sql`, applied by `deploy.sh`. |
| Runtime | `streak_days` never updated | Correct UTC-day `CASE` in the UPSERT. |
| Runtime | Summaries empty for new users | Enum left join returns all six categories. |
| Runtime | Raw SQL errors returned to clients | `ApiError` with logged internals and JSON body. |
| API | Create returned `200` | Returns `201 Created`. |
| API | No input validation | Trim + length + metadata validation, with unit tests. |
| API | `CORS = Any` | Env-driven allow-list; `*` only when explicitly configured. |
| API | No readiness check | `/health/ready` verifies the pool. |
| API | No graceful shutdown | SIGINT/SIGTERM handler. |
| Frontend | Not buildable (no `index.html`, `main.rs`, crate-type, Trunk config) | Added all; documented `trunk serve` / `trunk build`. |
| Frontend | Hard-coded `https://localhost` and demo UUID | Same-origin API default; UUID read from `localStorage` with demo fallback. |
| Frontend | Accept-anything request handling | Status checks, JSON error display, submit/loading state. |
| Frontend | Initial select value did not match state | Options bound to the category signal. |
| WeChat | Invalid mini program (no `app.json`/WXML/WXSS) | Added complete page, config, project settings, styles. |
| WeChat | Picker index was a string; API status assumed 200 | Numeric index; accepts any 2xx. |
| Nginx | Did not serve the web client | Serves `frontend/dist`, proxies `/api` and `/health`, adds CSP/caching. |
| Compose | API raced the database; API port public | Health-gated dependencies, loopback-only DB/API ports. |
| Schema | Not idempotent; `deploy.sh` re-ran it | `IF NOT EXISTS`, guarded enum, upgrade `ALTER`. |
| Deploy | `podman exec -it` fails without a TTY | Uses `-i`, `ON_ERROR_STOP=1`, retries, and a real readiness gate. |
| Deploy | systemd unit path assumed `~/event_register_project` | Unit is templated with the actual project directory and compose path. |
| Tooling | No editor configuration | Added `.vscode/` tasks, launch, settings, extensions, `.editorconfig`, `.gitignore`. |
| Docs | Spec/design/tests were sketches | Expanded into testable requirements, API contract, and verification steps. |

## 4. Verification Performed

- Applied `schema.sql` **twice** to a fresh PostgreSQL 14 instance: no errors (idempotent).
- Applied `seeds/dev.sql` twice: second run inserts zero rows.
- Exercised the streak UPSERT for first event, same day, consecutive day, and a gap:
  results `1 -> 1 -> +1 -> 1`.
- Verified the summaries query returns all six categories for a user with one category.
- `cargo test` passes (6 tests), `cargo clippy --all-targets -- -D warnings` is clean, and
  `cargo check --all-targets` succeeds with Rust 1.99.1.
- `cargo check --target wasm32-unknown-unknown` and `trunk build --release` both succeed.
- Ran the API against PostgreSQL and executed `scripts/smoke-test.sh`: health, readiness,
  summaries, create (`201`), increment, six categories, blank title (`400`), and unknown
  user (`404`) all behaved as specified.
- Validated `nginx/conf.d/event_register.conf` with `nginx -t` (nginx 1.27).
- `python3` parsed every JSON file and `docker-compose.yml`; `bash -n` parsed the scripts.
- **Not run:** browser end-to-end tests, WeChat DevTools runtime, multi-arch image builds,
  and a formal p95 latency benchmark. Do these before production rollout.

## 5. Concurrency Check (NFR-2 / AC-7)

With the stack running and the demo user seeded:

```bash
USER_ID=a0eebc99-9c0b-4ef8-bb6d-6bb9bd380a11
CATEGORY=writing
seq 1 50 | xargs -P 50 -I{} curl -fsS -o /dev/null \
  -X POST http://127.0.0.1:8080/api/v1/events \
  -H 'content-type: application/json' \
  -d "{\"user_id\":\"$USER_ID\",\"category\":\"$CATEGORY\",\"title\":\"load {}\"}"

podman exec event_register_db psql -U postgres -d event_register -tAc \
  "SELECT total_count FROM user_summaries WHERE user_id='$USER_ID' AND category='$CATEGORY'"
# Expected: 50 more than the count before the test
```

## 6. Remaining Risks and Recommended Next Steps

### P0 — before any public exposure

1. **Implement FR-1.** Add OIDC providers and a WeChat `code` exchange, issue an
   HttpOnly session cookie, and authorize every event/summary query by session user.
   Remove the demo seed and the `localStorage` user override in production.
2. **Add integration tests.** Use `#[sqlx::test]` for event + summary + streak behavior,
   including a concurrent tokio test. Wire `cargo test` and `trunk build` into CI.
3. **Keep both `Cargo.lock` files in version control** for reproducible container builds,
   and add CI that runs `cargo test`, `clippy`, and `trunk build`.

### P1 — hardening

4. Add request rate limiting and an explicit body-size limit (`DefaultBodyLimit`).
5. Return stable machine-readable error codes (`{"error": "...", "code": "..."}`).
6. Add pagination and a `GET /api/v1/events` history endpoint.
7. Run a real latency benchmark (e.g. `oha`, `wrk`, or `k6`) and record p95.

### P2 — maintainability

8. Remove the legacy `g` scaffold generator; it overwrites the improved files and is no
   longer the source of truth.
9. Add a `Cargo.workspace` at the repository root to share a target directory and lock file.
10. Add OpenAPI documentation and generated client types shared by Web and WeChat.
11. Add accessibility checks and localization for the Web UI.
