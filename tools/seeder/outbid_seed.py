#!/usr/bin/env python3
"""auctioning.lol project seeder — outbid.lol snapshot importer.

Feeds the Shuttle catalog (/v1/projects/import) from either:

  1. a local snapshot file (JSON array of listings), or
  2. a live outbid.lol board URL (best-effort; Vercel bot protection may
     return 403 — in that case save a manual snapshot and use mode 1).

The payload shape matches backend/shuttle-auctioning/src/catalog.rs
`ImportProject`. Imports are idempotent upserts keyed by stable_id, so
re-running against an updated snapshot never duplicates projects.

Usage:
  # dry-run: print the normalized payload
  ./outbid_seed.py --snapshot seed.sample.json

  # push to the backend (INGEST_SECRET must match the service secret)
  INGEST_SECRET=... ./outbid_seed.py \
      --snapshot seed.sample.json \
      --api https://auctioning-backend.shuttle.app --push

Snapshot row fields (all optional except stable_id):
  stable_id      deterministic key, convention "outbid:<original-id-or-slug>"
  handle/display_name/url/blurb/tags/source/source_ref/owner_wallet
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import sys
import urllib.error
import urllib.request

MAX_BATCH = 5000


def slugify(text: str) -> str:
    s = re.sub(r"[^a-z0-9_-]+", "-", text.lower()).strip("-")
    return (s or "project")[:32]


def stable_id_for(row: dict) -> str | None:
    sid = row.get("stable_id") or row.get("id") or row.get("slug")
    if not sid:
        return None
    sid = str(sid)
    if ":" not in sid:
        # Deterministic fallback: hash whatever identifiers exist.
        basis = json.dumps(row, sort_keys=True, default=str)[:512]
        digest = hashlib.sha1(basis.encode()).hexdigest()[:8]
        sid = f"outbid:{slugify(sid)}-{digest}"
    return sid[:200]


def normalize(rows: list[dict]) -> list[dict]:
    out: list[dict] = []
    seen: set[str] = set()
    for row in rows:
        if not isinstance(row, dict):
            continue
        sid = stable_id_for(row)
        if not sid or sid in seen:
            continue
        seen.add(sid)
        item = {"stable_id": sid}
        for src, dst in (
            ("handle", "handle"),
            ("name", "display_name"),
            ("display_name", "display_name"),
            ("url", "url"),
            ("blurb", "blurb"),
            ("description", "blurb"),
            ("tags", "tags"),
            ("source", "source"),
            ("source_ref", "source_ref"),
            ("owner_wallet", "owner_wallet"),
            ("wallet", "owner_wallet"),
        ):
            val = row.get(src)
            if val is not None and val != "":
                item[dst] = val
        if isinstance(item.get("tags"), str):
            item["tags"] = [t.strip() for t in item["tags"].split(",") if t.strip()]
        item.setdefault("source", "outbid_import")
        out.append(item)
    return out[:MAX_BATCH]


def load_snapshot(path_or_url: str) -> list[dict]:
    if path_or_url.startswith(("http://", "https://")):
        req = urllib.request.Request(
            path_or_url,
            headers={
                "User-Agent": "auctioning-seeder/0.1 (+https://auctioning.lol)",
                "Accept": "application/json,text/html;q=0.8",
            },
        )
        with urllib.request.urlopen(req, timeout=30) as resp:
            raw = resp.read().decode("utf-8", "replace")
    else:
        with open(path_or_url, encoding="utf-8") as fh:
            raw = fh.read()

    text = raw.strip()
    if text.startswith("[") or text.startswith("{"):
        data = json.loads(text)
        if isinstance(data, dict):
            data = data.get("projects") or data.get("items") or data.get("data") or []
        return list(data)

    # HTML fallback: pull embedded __NEXT_DATA__ / JSON arrays heuristically.
    m = re.search(r'<script id="__NEXT_DATA__"[^>]*>(.*?)</script>', raw, re.S)
    if m:
        data = json.loads(m.group(1))
        props = data.get("props", {}).get("pageProps", {})
        for key in ("projects", "listings", "items", "results"):
            if isinstance(props.get(key), list):
                return props[key]
    raise SystemExit("snapshot is neither JSON nor parseable NEXT_DATA HTML")


def push(api_base: str, payload: list[dict], ingest_secret: str) -> int:
    body = json.dumps({"projects": payload}).encode()
    req = urllib.request.Request(
        f"{api_base.rstrip('/')}/v1/projects/import",
        data=body,
        headers={"Content-Type": "application/json", "X-Auctioning-Ingest": ingest_secret},
        method="POST",
    )
    try:
        with urllib.request.urlopen(req, timeout=60) as resp:
            print(resp.read().decode())
            return 0
    except urllib.error.HTTPError as e:
        print(f"import failed: HTTP {e.code}: {e.read().decode(errors='replace')}", file=sys.stderr)
        return 2


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--snapshot", required=True, help="path or URL of the outbid.lol snapshot")
    ap.add_argument("--api", default=os.environ.get("AUCTIONING_API", ""), help="backend base URL")
    ap.add_argument("--push", action="store_true", help="POST to the backend instead of printing")
    args = ap.parse_args()

    rows = load_snapshot(args.snapshot)
    payload = normalize(rows)

    if args.push:
        if not args.api:
            ap.error("--push requires --api or AUCTIONING_API")
        secret = os.environ.get("INGEST_SECRET", "")
        if not secret:
            ap.error("--push requires INGEST_SECRET env var")
        return push(args.api, payload, secret)

    json.dump(payload, sys.stdout, indent=2)
    print(f"\n# {len(payload)} projects normalized (dry-run; pass --push to apply)", file=sys.stderr)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
