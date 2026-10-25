#!/usr/bin/env bash
set -euo pipefail

BASE_URL="${BASE_URL:-http://127.0.0.1:8080}"
USER_ID="${USER_ID:-a0eebc99-9c0b-4ef8-bb6d-6bb9bd380a11}"
CATEGORY="${CATEGORY:-writing}"
TMP_FILE="$(mktemp)"
trap 'rm -f "$TMP_FILE"' EXIT

pass() { printf 'ok   %s\n' "$1"; }
fail() {
    printf 'FAIL %s\n' "$1" >&2
    exit 1
}

json_count() {
    python3 -c '
import json, sys
rows = json.load(sys.stdin)
print(next((row["total_count"] for row in rows if row["category"] == sys.argv[1]), 0))
' "$CATEGORY"
}

command -v curl >/dev/null 2>&1 || fail "curl is required"
command -v python3 >/dev/null 2>&1 || fail "python3 is required"

health="$(curl -fsS "$BASE_URL/health")" || fail "health endpoint unreachable at $BASE_URL"
[ "$health" = "OK" ] || fail "health returned '$health' (expected OK)"
pass "health"

ready="$(curl -fsS "$BASE_URL/health/ready")" || fail "readiness endpoint failed"
[ "$ready" = "READY" ] || fail "readiness returned '$ready' (expected READY)"
pass "readiness"

if ! before="$(curl -fsS "$BASE_URL/api/v1/summaries/$USER_ID")"; then
    fail "summaries failed; is the demo user seeded? Run SEED_DEMO=1 ./deploy.sh"
fi
before_count="$(printf '%s' "$before" | json_count)"
pass "summaries loaded (current $CATEGORY count: $before_count)"

code="$(
    curl -sS -o "$TMP_FILE" -w '%{http_code}' \
        -X POST "$BASE_URL/api/v1/events" \
        -H 'content-type: application/json' \
        --data "{\"user_id\":\"$USER_ID\",\"category\":\"$CATEGORY\",\"title\":\"Smoke test $(date -u +%Y-%m-%dT%H:%M:%SZ)\"}"
)"
[ "$code" = "201" ] || fail "create event returned HTTP $code: $(cat "$TMP_FILE")"
pass "create event"

after="$(curl -fsS "$BASE_URL/api/v1/summaries/$USER_ID")" || fail "summaries failed after write"
after_count="$(printf '%s' "$after" | json_count)"
[ "$after_count" -eq "$((before_count + 1))" ] ||
    fail "summary count did not increment ($before_count -> $after_count)"
pass "summary increment ($before_count -> $after_count)"

rows="$(printf '%s' "$after" | python3 -c 'import json,sys; print(len(json.load(sys.stdin)))')"
[ "$rows" = "6" ] || fail "expected six summary rows, got $rows"
pass "all six categories present"

code="$(
    curl -sS -o "$TMP_FILE" -w '%{http_code}' \
        -X POST "$BASE_URL/api/v1/events" \
        -H 'content-type: application/json' \
        --data "{\"user_id\":\"$USER_ID\",\"category\":\"$CATEGORY\",\"title\":\"   \"}"
)"
[ "$code" = "400" ] || fail "blank title returned HTTP $code (expected 400)"
pass "validation rejects blank title"

code="$(
    curl -sS -o "$TMP_FILE" -w '%{http_code}' \
        "$BASE_URL/api/v1/summaries/00000000-0000-0000-0000-000000000000"
)"
[ "$code" = "404" ] || fail "unknown user returned HTTP $code (expected 404)"
pass "unknown user returns 404"

printf '\nAll smoke tests passed.\n'
