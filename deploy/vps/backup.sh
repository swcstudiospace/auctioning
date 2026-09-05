#!/usr/bin/env bash
# Nightly logical dump of the auctioning database. Keeps 14 daily dumps.
# Restore: zcat FILE | docker compose exec -T postgres psql -U auctioning auctioning
set -euo pipefail
cd /opt/auctioning/deploy/vps

DEST=/var/backups/auctioning
KEEP=14
mkdir -p "$DEST"
chmod 700 "$DEST"

stamp=$(date -u +%Y%m%dT%H%M%SZ)
out="$DEST/auctioning-$stamp.sql.gz"
docker compose --env-file /etc/auctioning/api.env exec -T postgres pg_dump -U auctioning --no-owner --no-privileges auctioning \
  | gzip -9 > "$out.tmp"
mv "$out.tmp" "$out"
sha256sum "$out" > "$out.sha256"

ls -1t "$DEST"/auctioning-*.sql.gz | tail -n +$((KEEP + 1)) | while read -r old; do
  rm -f "$old" "$old.sha256"
done
echo "backup written: $out ($(du -h "$out" | cut -f1))"
