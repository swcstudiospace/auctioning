# outbid.lol collector

Mirrors every business on outbid.lol, with the dollars it has paid there, into
the auctioning.lol catalog. The API credits **1 RP per $1** (floor), increase
only, so re-running is always safe. Full runbook: `docs/RUNBOOK.md` §6.

```bash
# dry-run (prints the collection)
./outbid_collector.py --no-products | jq '.entries[:3]'

# push to the API
INGEST_SECRET=... ./outbid_collector.py --api https://api-auctioning.swcstudio.space --push

# autosync every hour (this is what deploy/vps/docker-compose.yml runs)
INGEST_SECRET=... ./outbid_collector.py --api http://api:8000 --push --loop --interval 3600

# replay a saved collection
./outbid_collector.py --snapshot outbid-2026-09-06.json --api ... --push
```

Requirements: Python 3.11+. Plain HTTP is tried first; when outbid.lol's Vercel
checkpoint answers 429 the collector switches to headless Chromium, which needs
`pip install playwright && playwright install chromium` (the compose service
uses the official Playwright image, so nothing to install there).

What a run reads (all through the Next.js RSC payload, `RSC: 1`):

| Page | Gives |
|---|---|
| `/` | all-time top 50 (rank + amountCents) and today's board |
| `/category/<slug>` ×28 | top ~50 per category with amounts |
| `/daily/<date>` × `--days` | daily boards with amounts |
| `/product/<host>` (`--products`) | sitemap listings not on any board: description, category, clicks; amount unknown → 0 RP |

Entries are merged by host (highest amount wins) and pushed to
`POST /v1/outbid/sync` in batches of ≤5000. `GET /v1/outbid/status` shows the
totals and the last runs.
