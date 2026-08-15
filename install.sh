#!/usr/bin/env bash
# One-click install: curl -fsSL https://raw.githubusercontent.com/t2vi/spoolbook/main/install.sh | bash
#
# Downloads just docker-compose.yml (not the whole source tree) into ./spoolbook/ and pulls the
# pre-built ghcr.io images — no local build needed. Matches the community-scripts.org UX this is
# modeled on.
set -euo pipefail

if ! command -v docker >/dev/null; then
  echo "Docker not found — install it first: https://docs.docker.com/engine/install/"
  exit 1
fi
if ! docker compose version >/dev/null 2>&1; then
  echo "Docker Compose plugin not found — install it first: https://docs.docker.com/compose/install/"
  exit 1
fi

mkdir -p spoolbook && cd spoolbook

if [[ ! -f docker-compose.yml ]]; then
  echo "Fetching docker-compose.yml..."
  curl -fsSL -o docker-compose.yml https://raw.githubusercontent.com/t2vi/spoolbook/main/docker-compose.yml
fi

if [[ ! -f .env ]]; then
  echo "No .env found — setting one up."
  # Reads from /dev/tty, not stdin — when piped from curl, stdin is the script itself, not
  # the terminal, so a plain `read` here would get EOF instead of the user's input.
  read -rsp "Choose an admin password (gates editing/deleting/sending prints): " admin_pass < /dev/tty
  echo
  if [[ -z "$admin_pass" ]]; then
    echo "Password can't be empty."
    exit 1
  fi
  printf 'SPOOLBOOK_ADMIN_PASSWORD=%s\n' "$admin_pass" > .env
  echo ".env created."
fi

echo "Pulling images..."
docker compose pull

echo "Starting spoolbook..."
docker compose up -d

echo
echo "spoolbook is starting. Once healthy, it's at: http://$(hostname -I 2>/dev/null | awk '{print $1}' || echo localhost):5070"
echo "Check status: docker compose ps"
echo "Logs: docker compose logs -f"
