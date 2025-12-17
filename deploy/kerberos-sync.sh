#!/usr/bin/env bash
set -euo pipefail

if [[ $# -lt 2 ]]; then
  echo "Usage: $0 user|device <id>" >&2
  exit 1
fi

type="$1"
id="$2"
SERVER="${SERVER:-https://localhost:8443}"
TOKEN="${UD_ADMIN_TOKEN:-}"
COMPOSE="${COMPOSE_FILE:-deploy/docker-compose.yml}"
KDC_SERVICE="${KDC_SERVICE:-krb5-kdc}"

if [[ -z "$TOKEN" ]]; then
  echo "UD_ADMIN_TOKEN must be set" >&2
  exit 1
fi

case "$type" in
  user)
    url="$SERVER/v1/kerberos/users/$id/commands"
    ;;
  device)
    url="$SERVER/v1/kerberos/devices/$id/commands"
    ;;
  *)
    echo "type must be user or device" >&2
    exit 1
    ;;
esac

cmds=$(curl -sk -X POST "$url" -H "Authorization: Bearer $TOKEN")
if [[ -z "$cmds" ]]; then
  echo "no commands returned" >&2
  exit 1
fi

join() { local IFS=$'\n'; echo "$*"; }

if ! command -v docker &>/dev/null; then
  echo "docker not available; printing commands only" >&2
  echo "$cmds"
  exit 0
fi

echo "$cmds" | jq -r '.commands[]' | while read -r line; do
  echo "Running in KDC: $line"
  docker compose -f "$COMPOSE" exec -T "$KDC_SERVICE" bash -lc "$line"
done
