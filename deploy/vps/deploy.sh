#!/usr/bin/env bash
# Pull main, rebuild the API image, roll it, verify readiness.
# Usage: sudo /opt/auctioning/deploy/vps/deploy.sh [git-ref]
set -euo pipefail
REF=${1:-main}
cd /opt/auctioning

before=$(git rev-parse --short HEAD)
git fetch -q origin
git checkout -q "$REF"
git reset -q --hard "origin/$REF" 2>/dev/null || true
after=$(git rev-parse --short HEAD)
echo "deploying $before -> $after ($REF)"

cd deploy/vps
docker compose --env-file /etc/auctioning/api.env build --pull api
docker compose --env-file /etc/auctioning/api.env up -d --remove-orphans

for i in $(seq 1 30); do
  if curl -fsS http://127.0.0.1:8000/readyz >/dev/null 2>&1; then
    echo "ready:"; curl -sS http://127.0.0.1:8000/readyz; echo
    docker image prune -f --filter "label=stage=builder" >/dev/null 2>&1 || true
    exit 0
  fi
  sleep 2
done
echo "API did not become ready; recent logs:" >&2
docker compose --env-file /etc/auctioning/api.env logs --tail=80 api >&2
exit 1
