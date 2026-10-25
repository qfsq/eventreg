#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$ROOT_DIR"

if [ -f .env ]; then
    set -a
    # shellcheck disable=SC1091
    . ./.env
    set +a
fi

DB_USER="${POSTGRES_USER:-postgres}"
DB_NAME="${POSTGRES_DB:-event_register}"
SEED_DEMO="${SEED_DEMO:-1}"
PROJECT_DIR="$ROOT_DIR"

info() { printf '\n==> %s\n' "$*"; }
warn() { printf 'warning: %s\n' "$*" >&2; }

info "Checking prerequisites"
command -v podman >/dev/null 2>&1 || {
    echo "Error: podman is required." >&2
    exit 1
}
command -v openssl >/dev/null 2>&1 || {
    echo "Error: openssl is required to generate bootstrap certificates." >&2
    exit 1
}

if command -v podman-compose >/dev/null 2>&1; then
    COMPOSE=(podman-compose)
    COMPOSE_UNIT="$(command -v podman-compose)"
elif podman compose version >/dev/null 2>&1; then
    COMPOSE=(podman compose)
    COMPOSE_UNIT="$(command -v podman) compose"
else
    echo "Error: neither 'podman-compose' nor the 'podman compose' plugin was found." >&2
    exit 1
fi
printf 'Using compose tool: %s\n' "${COMPOSE[*]}"

info "[1/7] Enabling rootless Podman socket"
systemctl --user enable --now podman.socket 2>/dev/null ||
    warn "could not enable podman.socket; continuing without it"

info "[2/7] Preparing TLS certificates"
mkdir -p nginx/certs
chmod 700 nginx/certs
if [ ! -s nginx/certs/fullchain.pem ] || [ ! -s nginx/certs/privkey.pem ]; then
    echo "Generating a self-signed certificate for local bootstrapping..."
    if ! openssl req -x509 -nodes -days 365 -newkey rsa:2048 \
        -keyout nginx/certs/privkey.pem \
        -out nginx/certs/fullchain.pem \
        -subj "/CN=localhost" \
        -addext "subjectAltName=DNS:localhost,DNS:api.yourdomain.com,IP:127.0.0.1" \
        >/dev/null 2>&1; then
        openssl req -x509 -nodes -days 365 -newkey rsa:2048 \
            -keyout nginx/certs/privkey.pem \
            -out nginx/certs/fullchain.pem \
            -subj "/CN=localhost"
    fi
    chmod 600 nginx/certs/privkey.pem
    warn "replace nginx/certs/*.pem with a real certificate before exposing this host"
fi

info "[3/7] Preparing the web client"
if command -v trunk >/dev/null 2>&1; then
    (cd frontend && trunk build --release)
elif [ -f frontend/dist/index.html ]; then
    echo "Trunk not found; using the existing frontend/dist build."
else
    mkdir -p frontend/dist
    cat > frontend/dist/index.html <<'HTML'
<!doctype html>
<html lang="en">
  <head><meta charset="utf-8"><title>EventRegister</title></head>
  <body style="font-family: system-ui; max-width: 40rem; margin: 4rem auto">
    <h1>EventRegister</h1>
    <p>The web client has not been built yet. Install Trunk, then run:</p>
    <pre>cd frontend &amp;&amp; trunk build --release</pre>
  </body>
</html>
HTML
    warn "trunk was not found; installed a placeholder page in frontend/dist"
fi

info "[4/7] Building images and starting the stack"
"${COMPOSE[@]}" up --build -d

info "[5/7] Waiting for PostgreSQL"
for _ in $(seq 1 60); do
    if podman exec event_register_db \
        pg_isready -U "$DB_USER" -d "$DB_NAME" >/dev/null 2>&1; then
        break
    fi
    sleep 2
    printf '    waiting for PostgreSQL...\n'
done
podman exec event_register_db pg_isready -U "$DB_USER" -d "$DB_NAME" >/dev/null

info "[6/7] Applying schema"
podman exec -i event_register_db \
    psql -v ON_ERROR_STOP=1 -U "$DB_USER" -d "$DB_NAME" < schema.sql

case "${SEED_DEMO,,}" in
    1 | true | yes)
        info "Seeding the development demo user (set SEED_DEMO=0 to skip)"
        podman exec -i event_register_db \
            psql -v ON_ERROR_STOP=1 -U "$DB_USER" -d "$DB_NAME" < seeds/dev.sql
        ;;
    *)
        echo "Skipping demo seed."
        ;;
esac

info "[7/7] Installing the systemd user service"
if command -v systemctl >/dev/null 2>&1 && systemctl --user show-environment >/dev/null 2>&1; then
    UNIT_DIR="$HOME/.config/systemd/user"
    mkdir -p "$UNIT_DIR"
    sed \
        -e "s|@PROJECT_DIR@|\"$PROJECT_DIR\"|g" \
        -e "s|@COMPOSE@|$COMPOSE_UNIT|g" \
        systemd/event-register.service >"$UNIT_DIR/event-register.service"
    systemctl --user daemon-reload
    systemctl --user enable --now event-register.service
    loginctl enable-linger "$(id -un)" 2>/dev/null ||
        warn "could not enable linger for $(id -un)"
else
    warn "systemd user session not available; skipping service installation"
fi

cat <<EOF

==================================================
 Deployment complete
   Web UI:  https://localhost
   API:     http://127.0.0.1:${API_PORT:-8080}/health
   Logs:    ${COMPOSE[*]} logs -f
   Stop:    ${COMPOSE[*]} down
==================================================
EOF
