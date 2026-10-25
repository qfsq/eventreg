# Test Cases & Verification Plan

## 1. Automated Tests

### Backend unit tests

```bash
cd backend
cargo test
```

Covers request normalization and rejection (blank/overlong title, blank content, non-object
metadata, category serialization). No database is required.

### API smoke test

With the stack running and the demo user seeded:

```bash
./scripts/smoke-test.sh
```

Checks health, readiness, summaries shape, event creation (`201`), summary increment,
validation (`400`), and unknown-user handling (`404`).

### Static checks

```bash
cd backend && cargo fmt --check && cargo clippy --all-targets -- -D warnings
cd frontend && trunk build --release
python3 -m json.tool wechat/app.json >/dev/null
bash -n deploy.sh scripts/smoke-test.sh
```

## 2. Case Matrix

| ID | Scenario | Steps | Expected |
| --- | --- | --- | --- |
| **TC-BE-01** | Event creation increments summary | `POST /api/v1/events` with `category=writing` | `201`, event returned, `total_count +1`, `last_logged_at` updated |
| **TC-BE-02** | Concurrent UPSERT integrity | 50 parallel posts, same user/category | No lost updates; `total_count` increases by exactly 50 |
| **TC-BE-03** | Blank title rejected | `POST` with `"title": "   "` | `400 {"error":"title must not be empty"}`, no event row |
| **TC-BE-04** | Unknown user rejected | `POST` with random UUID | `400 {"error":"unknown user_id"}`, no event row |
| **TC-BE-05** | Oversized title rejected | 256 characters | `400` |
| **TC-BE-06** | Non-object metadata rejected | `"metadata": []` | `400` |
| **TC-BE-07** | Streak same day | Two events in one UTC day | `total_count +2`, streak unchanged |
| **TC-BE-08** | Streak consecutive day | Previous log yesterday, streak 4 | streak becomes 5 |
| **TC-BE-09** | Streak gap | Previous log three days ago | streak resets to 1 |
| **TC-BE-10** | Category completeness | Summaries for user with one category | Exactly six rows; other categories have `total_count = 0` |
| **TC-BE-11** | Unknown user summaries | `GET` with random UUID | `404` |
| **TC-BE-12** | Readiness failure | Stop PostgreSQL, call `/health/ready` | `500` JSON error |
| **TC-API-01** | Health check | `GET /health` | `200`, body `OK` |
| **TC-API-02** | Readiness | `GET /health/ready` | `200`, body `READY` |
| **TC-API-03** | Error shape | Any failing request | `{"error": "..."}`; no SQL text |
| **TC-UI-01** | Web: log event | Fill form, submit | Button disabled while saving, fields clear, summary refreshes |
| **TC-UI-02** | Web: API failure | Stop API, submit | Visible error banner, no panic |
| **TC-UI-03** | WeChat: log event | Pick category, submit | Toast, fields clear, summaries refresh |
| **TC-DEP-01** | Idempotent deploy | Run `./deploy.sh` twice | Both succeed; no duplicate schema errors |
| **TC-DEP-02** | Auto-start | Reboot/log in | `systemctl --user status event-register` is active, stack is up |

## 3. Concurrency Test

```bash
USER_ID=a0eebc99-9c0b-4ef8-bb6d-6bb9bd380a11
CATEGORY=writing

before=$(podman exec event_register_db psql -U postgres -d event_register -tAc \
  "SELECT COALESCE(total_count,0) FROM user_summaries WHERE user_id='$USER_ID' AND category='$CATEGORY'")

seq 1 50 | xargs -P 50 -I{} curl -fsS -o /dev/null \
  -X POST http://127.0.0.1:8080/api/v1/events \
  -H 'content-type: application/json' \
  -d "{\"user_id\":\"$USER_ID\",\"category\":\"$CATEGORY\",\"title\":\"load {}\"}"

after=$(podman exec event_register_db psql -U postgres -d event_register -tAc \
  "SELECT COALESCE(total_count,0) FROM user_summaries WHERE user_id='$USER_ID' AND category='$CATEGORY'")

test $((after - before)) -eq 50 && echo "PASS: +50"
```

## 4. Manual End-to-End Checklist

1. `cp .env.example .env` and change the database password.
2. `./deploy.sh` completes; open `https://localhost` and accept the self-signed certificate.
3. The summary grid shows all six categories at zero.
4. Log an event; the total becomes 1 and the last-logged date appears.
5. Restart the host (or `systemctl --user restart event-register`); the stack returns.
6. `./scripts/smoke-test.sh` exits `0`.

## 5. Gaps

- No database-backed integration test suite yet. Add `#[sqlx::test]` tests before Milestone 2.
- No automated browser or WeChat DevTools tests.
- No recorded latency benchmark for NFR-1.
