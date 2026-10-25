# Functional & Non-Functional Specification: EventRegister

> **Implementation status:** Milestone 1 (single-user event logging, summaries, streaks,
> Web/WASM and WeChat clients, container deployment) is implemented. Federated
> authentication and per-user authorization (FR-1) are **Milestone 2 and not implemented
> yet**. See [`docs/SPEC-REVIEW.md`](docs/SPEC-REVIEW.md) for the current gap analysis.

## 1. Objectives

- Provide sub-50 ms event logging and summary retrieval on a local deployment.
- Support multi-platform access: a browser client compiled to WebAssembly and a WeChat
  Mini Program.
- Keep data consistent under concurrency using atomic SQL transactions and `UPSERT`
  aggregation.
- Deploy as a rootless Podman stack with automatic restart at boot.

## 2. Scope

**Milestone 1 (implemented):** one demo identity, event creation, category summaries,
streak calculation, health checks, idempotent schema bootstrap, Web and WeChat clients.

**Milestone 2 (planned):** federated login, session handling, per-user authorization,
account management, WeChat code exchange, rate limiting.

## 3. Functional Requirements

| ID | Requirement |
| --- | --- |
| **FR-1** | Users authenticate through OAuth 2.0/OIDC (Google, Facebook, Outlook) or WeChat. Every API request is authorized for the authenticated user. **Milestone 2 — not implemented.** |
| **FR-2** | Exactly six event categories exist: `chore`, `work`, `emotion`, `thinking`, `writing`, `inventing`. Unknown categories are rejected. |
| **FR-3** | `POST /api/v1/events` creates one event for a user. The event and its summary increment commit atomically. |
| **FR-4** | Event titles are required, trimmed, and at most 255 characters. Notes are optional and at most 20,000 characters. Optional metadata must be a JSON object of at most 16 KiB. |
| **FR-5** | `GET /api/v1/summaries/{user_id}` returns **all six categories**, including zero-count rows for categories with no events. An unknown user returns `404`. |
| **FR-6** | Each summary exposes `total_count`, `last_logged_at`, `streak_days`, and `aggregate_metrics`. |
| **FR-7** | Streaks are measured in UTC calendar days. A second event on the same day does not change the streak; an event on the next day increments it by one; a gap of two or more days resets it to one. |
| **FR-8** | The Web client can create events and display all summaries; the WeChat client has equivalent functionality. |
| **FR-9** | `GET /health` returns `200` with the body `OK`. `GET /health/ready` returns `200` when PostgreSQL is reachable and `500` otherwise. |
| **FR-10** | API errors use a JSON body `{"error": "<message>"}`. Internal SQL details and connection strings are never returned to clients. |

## 4. API Contract

Base path: `/api/v1`. All request and response bodies are JSON unless noted.

### `POST /api/v1/events`

```json
{
  "user_id": "a0eebc99-9c0b-4ef8-bb6d-6bb9bd380a11",
  "category": "writing",
  "title": "Finished module draft",
  "content": "Optional notes",
  "metadata": { "mood": "focused" }
}
```

| Status | Meaning |
| --- | --- |
| `201 Created` | Event inserted and summary updated. Body is the created event. |
| `400 Bad Request` | Validation failed, unknown `user_id`, or malformed JSON. |
| `500 Internal Server Error` | Unexpected server failure (details are logged, not returned). |

### `GET /api/v1/summaries/{user_id}`

```json
[
  {
    "category": "chore",
    "total_count": 0,
    "last_logged_at": null,
    "streak_days": 0,
    "aggregate_metrics": {}
  }
]
```

| Status | Meaning |
| --- | --- |
| `200 OK` | Six summaries in enum order, including zero-count categories. |
| `404 Not Found` | User does not exist. |
| `500 Internal Server Error` | Database failure. |

### Health checks

| Endpoint | Success | Failure |
| --- | --- | --- |
| `GET /health` | `200 OK`, body `OK` | — |
| `GET /health/ready` | `200 OK`, body `READY` | `500` with JSON error |

## 5. Non-Functional Requirements

| ID | Requirement |
| --- | --- |
| **NFR-1 Performance** | Server-side p95 latency below 50 ms for both endpoints under 20 concurrent local clients. |
| **NFR-2 Consistency** | 50 concurrent events for the same user and category produce `total_count = 50` with no lost updates. |
| **NFR-3 Security** | TLS 1.2+ only; restrictive CORS; non-root API container; no credentials in the repository; bounded request payloads. Authentication is required once FR-1 lands. |
| **NFR-4 Availability** | Containers restart unless stopped; PostgreSQL health gates API startup; the API shuts down gracefully on `SIGTERM`/`SIGINT`. |
| **NFR-5 Operability** | Schema application is idempotent; demo seeding is opt-out; configuration is environment-driven; logs are structured. |
| **NFR-6 Portability** | Runs on rootless Podman Compose on amd64 and arm64 hosts. |
| **NFR-7 Compatibility** | Current Chrome, Firefox, and Safari; WeChat base library 3.x. |

## 6. Data Model

- `users` — identity mapping (`id`, `email`, `provider`, `provider_id`, `display_name`).
- `events` — individual logs (`id`, `user_id`, `category`, `title`, `content`, `metadata`,
  `logged_at`, `created_at`).
- `user_summaries` — pre-aggregated metrics keyed by `(user_id, category)`.
- See [`DESIGN.md`](DESIGN.md) for indexes, constraints, and transaction details.

## 7. Acceptance Criteria

| ID | Criterion |
| --- | --- |
| **AC-1** | `./deploy.sh` completes on a clean host and `https://localhost` serves the web client. |
| **AC-2** | `curl -fsS http://127.0.0.1:8080/health` prints `OK`. |
| **AC-3** | Posting a valid event returns `201`; the matching summary count increases by exactly one. |
| **AC-4** | A blank title returns `400` and creates no row. |
| **AC-5** | Summaries always contain six categories, even for a user with no events. |
| **AC-6** | Two events on the same UTC day leave the streak unchanged; the next day increments it. |
| **AC-7** | 50 concurrent posts yield `total_count = 50` (`docs/SPEC-REVIEW.md` includes a command). |
| **AC-8** | `./scripts/smoke-test.sh` exits `0`. |

## 8. Out of Scope

- Event editing/deletion and a paginated event history endpoint.
- Team accounts, sharing, admin dashboards, or analytics beyond per-category summaries.
- Offline-first synchronization and push notifications.
- Horizontal scaling beyond one API instance (the schema is ready for it).

## 9. Milestone 2 Decisions

- Session format (HttpOnly cookie vs. signed bearer token).
- Provider allow-list and account-linking rules.
- WeChat `code` exchange endpoint and secret storage.
- Per-user authorization on events/summaries and rate limiting.
