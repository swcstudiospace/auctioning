#!/usr/bin/env python3
"""outbid.lol collector — mirrors every ranked business and its dollar value
into the auctioning.lol catalog (POST /v1/outbid/sync).

outbid.lol is a Next.js (App Router) leaderboard. Each ranking entry is
embedded in the page's React Server Components payload, which any client can
request with the `RSC: 1` header. The site sits behind Vercel's bot
checkpoint, so plain HTTP is tried first and a real headless Chromium
(Playwright) takes over when the checkpoint answers 429. In browser mode the
same RSC fetches run *inside* the page, so one solved challenge covers the
whole crawl.

What one collection covers:
  /                       all-time top board (rank + amountCents) and today's board
  /category/<slug>        top board per category (28 categories)
  /daily/<yyyy-mm-dd>     daily boards for the last --days days
  /product/<host>         (optional, --products) sitemap entries not seen on any
                          board — description/category/clicks, amount unknown (0)

Entries are merged by host; the highest amount wins. The API mirrors dollars
into RP at 1 RP = $1 and only ever credits increases, so re-running is safe.

Usage:
  # dry-run: print what would be pushed
  ./outbid_collector.py

  # push once
  INGEST_SECRET=... ./outbid_collector.py --api https://api-auctioning.swcstudio.space --push

  # autosync (what deploy/vps/docker-compose.yml runs)
  INGEST_SECRET=... ./outbid_collector.py --api http://api:8000 --push --loop --interval 3600

  # replay a saved collection
  ./outbid_collector.py --snapshot outbid-2026-09-06.json --push
"""

from __future__ import annotations

import argparse
import datetime as dt
import json
import os
import re
import sys
import time
import urllib.error
import urllib.request
from typing import Any, Callable, Iterable

COLLECTOR = "outbid_collector/1.0"
OUTBID = "https://outbid.lol"
UA = (
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 "
    "(KHTML, like Gecko) Chrome/128.0.0.0 Safari/537.36"
)
CHECKPOINT_MARK = "Vercel Security Checkpoint"
MAX_BATCH = 5000

# Fallback when /categories cannot be parsed; refreshed from the live page.
CATEGORIES_FALLBACK = [
    "leaderboards-attention", "seo-ai-visibility", "marketing-advertising",
    "productivity-personal-tools", "ai-agents-infrastructure", "other",
    "crypto-web3-investing", "developer-tools", "health-fitness-wellness",
    "business-finance-legal", "games-entertainment", "ecommerce-retail",
    "travel-local-lifestyle", "directories-launch-discovery",
    "agencies-studios-services", "ai-media-generation", "education-learning",
    "social-media-creator-tools", "people-profiles", "design-creative",
    "hiring-jobs-careers", "domains-web-assets", "security-privacy-compliance",
    "sales-lead-generation", "media-news", "real-estate-property",
    "writing-content", "audio-voice-podcasting",
]

ENTRY_RE = re.compile(r'\{[^{}]*"amountCents":\d+[^{}]*\}')
PRODUCT_RE = re.compile(r'\{[^{}]*"absoluteRank":[^{}]*"rankingEntryId":[^{}]*\}')
SLUG_RE = re.compile(r'"slug":"([a-z0-9-]+)"')
LOC_RE = re.compile(r"<loc>([^<]+)</loc>")


def log(msg: str) -> None:
    print(f"[{dt.datetime.now(dt.timezone.utc).strftime('%H:%M:%S')}] {msg}", file=sys.stderr, flush=True)


# ---------------------------------------------------------------------------
# Fetching: plain HTTP first, Playwright when the checkpoint blocks us.
# ---------------------------------------------------------------------------


class Blocked(Exception):
    """Vercel's bot checkpoint answered instead of the page."""


class HttpFetcher:
    name = "http"

    def __init__(self, delay: float) -> None:
        self.delay = delay

    def fetch(self, path: str, rsc: bool = True) -> str:
        req = urllib.request.Request(
            OUTBID + path,
            headers={
                "User-Agent": UA,
                "Accept": "*/*",
                "Accept-Language": "en-US,en;q=0.9",
                **({"RSC": "1"} if rsc else {}),
            },
        )
        try:
            with urllib.request.urlopen(req, timeout=30) as resp:
                body = resp.read().decode("utf-8", "replace")
        except urllib.error.HTTPError as e:
            body = e.read().decode("utf-8", "replace") if e.fp else ""
            if e.code == 429 or CHECKPOINT_MARK in body:
                raise Blocked(f"HTTP {e.code}") from None
            raise
        if CHECKPOINT_MARK in body:
            raise Blocked("checkpoint page")
        time.sleep(self.delay)
        return body

    def close(self) -> None:
        pass


class BrowserFetcher:
    """Headless Chromium via Playwright. The checkpoint is a JS challenge; a
    real browser solves it once, then in-page fetch() reuses the clearance."""

    name = "browser"

    def __init__(self, delay: float, headless: bool = True) -> None:
        from playwright.sync_api import sync_playwright  # lazy: optional dep

        self.delay = delay
        self._pw = sync_playwright().start()
        self._browser = self._pw.chromium.launch(headless=headless)
        self._ctx = self._browser.new_context(user_agent=UA, locale="en-US")
        self._page = self._ctx.new_page()
        self._page.goto(OUTBID + "/", wait_until="domcontentloaded", timeout=60_000)
        for _ in range(60):
            if CHECKPOINT_MARK not in (self._page.title() or ""):
                break
            time.sleep(1)
        else:
            raise Blocked("checkpoint did not clear in 60s")
        log("browser: checkpoint cleared")

    def fetch(self, path: str, rsc: bool = True) -> str:
        js = """async ([path, rsc]) => {
            const r = await fetch(path, { headers: rsc ? { RSC: '1' } : {} });
            return [r.status, await r.text()];
        }"""
        status, body = self._page.evaluate(js, [path, rsc])
        if status == 429 or CHECKPOINT_MARK in body:
            raise Blocked(f"in-page fetch {status}")
        if status >= 400:
            raise urllib.error.HTTPError(OUTBID + path, status, "fetch failed", {}, None)  # type: ignore[arg-type]
        time.sleep(self.delay)
        return body

    def close(self) -> None:
        try:
            self._ctx.close()
            self._browser.close()
            self._pw.stop()
        except Exception:  # noqa: BLE001 — best effort
            pass


def make_fetcher(mode: str, delay: float) -> HttpFetcher | BrowserFetcher:
    if mode == "browser":
        return BrowserFetcher(delay)
    return HttpFetcher(delay)


# ---------------------------------------------------------------------------
# Parsing the RSC payload.
# ---------------------------------------------------------------------------


def _date(v: Any) -> str | None:
    """RSC encodes dates as "$D2026-08-24T08:48:56.493Z"."""
    if not isinstance(v, str):
        return None
    v = v[2:] if v.startswith("$D") else v
    return v if re.match(r"^\d{4}-\d{2}-\d{2}T", v) else None


def parse_entries(payload: str) -> list[dict]:
    out: list[dict] = []
    for m in ENTRY_RE.finditer(payload):
        try:
            raw = json.loads(m.group(0))
        except json.JSONDecodeError:
            continue
        if not raw.get("id"):
            continue
        out.append(
            {
                "id": str(raw["id"]),
                "identity_key": raw.get("identityKey"),
                "display_name": raw.get("displayName"),
                "description": raw.get("description"),
                "source_url": raw.get("sourceUrl"),
                "image_url": raw.get("imageUrl"),
                "category_slug": raw.get("categorySlug"),
                "amount_cents": int(raw.get("amountCents") or 0),
                "click_count": raw.get("clickCount"),
                "created_at": _date(raw.get("createdAt")),
                "category_rank": raw.get("categoryRank"),
            }
        )
    return out


def parse_product(payload: str, host: str) -> dict | None:
    m = PRODUCT_RE.search(payload)
    if not m:
        return None
    try:
        raw = json.loads(m.group(0))
    except json.JSONDecodeError:
        return None
    return {
        "id": str(raw.get("rankingEntryId") or f"product:{host}"),
        "identity_key": f"website:{raw.get('listingHost') or host}",
        "display_name": raw.get("identityLabel"),
        "description": raw.get("description"),
        "source_url": raw.get("outboundUrl"),
        "product_url": raw.get("productUrl"),
        "image_url": raw.get("identityImageUrl"),
        "category_slug": raw.get("categorySlug"),
        "category_name": raw.get("categoryName"),
        "amount_cents": 0,  # product pages do not expose the amount
        "click_count": raw.get("clickCount"),
        "created_at": _date(raw.get("createdAt")),
        "rank": raw.get("absoluteRank"),
    }


def parse_categories(payload: str) -> list[str]:
    seen: list[str] = []
    for s in SLUG_RE.findall(payload):
        if s not in seen:
            seen.append(s)
    return seen or CATEGORIES_FALLBACK


def host_of(entry: dict) -> str | None:
    key = entry.get("identity_key") or ""
    if ":" in key:
        key = key.split(":", 1)[1]
    src = key or (entry.get("source_url") or "")
    src = re.sub(r"^https?://", "", src).split("/")[0].split("?")[0].split("@")[-1].split(":")[0]
    src = src.strip().strip(".").lower()
    if src.startswith("www."):
        src = src[4:]
    if not src or "." not in src or not re.match(r"^[a-z0-9.-]+$", src):
        return None
    return src


# ---------------------------------------------------------------------------
# Collection.
# ---------------------------------------------------------------------------


class Collection:
    def __init__(self) -> None:
        self.by_host: dict[str, dict] = {}

    def add(self, entries: Iterable[dict], rank_from_order: bool = False) -> int:
        n = 0
        for i, e in enumerate(entries, start=1):
            host = host_of(e)
            if not host:
                continue
            if rank_from_order and e.get("rank") is None:
                e = {**e, "rank": i}
            kept = self.by_host.get(host)
            if kept is None:
                self.by_host[host] = dict(e)
            else:
                if e.get("amount_cents", 0) > kept.get("amount_cents", 0):
                    kept["amount_cents"] = e["amount_cents"]
                for k, v in e.items():
                    if kept.get(k) in (None, "", 0) and v not in (None, ""):
                        kept[k] = v
                if e.get("rank") is not None and (kept.get("rank") is None or e["rank"] < kept["rank"]):
                    kept["rank"] = e["rank"]
            n += 1
        return n

    def entries(self) -> list[dict]:
        return sorted(self.by_host.values(), key=lambda e: (-e.get("amount_cents", 0), e["id"]))


def collect(fetch: Callable[[str, bool], str], days: int, products: bool, known_hosts: set[str], max_products: int) -> Collection:
    col = Collection()

    home = fetch("/", True)
    home_entries = parse_entries(home)
    # The home payload carries the all-time board first (rank = order) and
    # today's board after it; rank only the all-time slice.
    all_time = [e for e in home_entries]
    n = col.add(all_time[:50], rank_from_order=True)
    n += col.add(all_time[50:])
    log(f"home: {n} entries")

    try:
        cats = parse_categories(fetch("/categories", True))
    except Exception as e:  # noqa: BLE001
        log(f"categories page failed ({e}); using fallback list")
        cats = CATEGORIES_FALLBACK
    for slug in cats:
        try:
            n = col.add(parse_entries(fetch(f"/category/{slug}", True)))
            log(f"category {slug}: {n}")
        except Exception as e:  # noqa: BLE001
            log(f"category {slug} failed: {e}")

    today = dt.datetime.now(dt.timezone.utc).date()
    for d in range(days):
        day = (today - dt.timedelta(days=d)).isoformat()
        try:
            n = col.add(parse_entries(fetch(f"/daily/{day}", True)))
            log(f"daily {day}: {n}")
        except Exception as e:  # noqa: BLE001
            log(f"daily {day} failed: {e}")

    if products:
        try:
            sitemap = fetch("/sitemap.xml", False)
        except Exception as e:  # noqa: BLE001
            log(f"sitemap failed: {e}")
            sitemap = ""
        hosts = [
            loc.rsplit("/product/", 1)[1].strip("/")
            for loc in LOC_RE.findall(sitemap)
            if "/product/" in loc
        ]
        todo = [h for h in hosts if h not in col.by_host and h not in known_hosts][:max_products]
        log(f"sitemap: {len(hosts)} products, {len(todo)} to fetch")
        for i, host in enumerate(todo, start=1):
            try:
                p = parse_product(fetch(f"/product/{host}", True), host)
                if p:
                    col.add([p])
            except Exception as e:  # noqa: BLE001
                log(f"product {host} failed: {e}")
            if i % 100 == 0:
                log(f"products: {i}/{len(todo)}")
    return col


def run_collection(mode: str, delay: float, days: int, products: bool, known_hosts: set[str], max_products: int) -> list[dict]:
    fetcher = make_fetcher(mode, delay)
    try:
        try:
            col = collect(fetcher.fetch, days, products, known_hosts, max_products)
        except Blocked as b:
            if fetcher.name == "browser" or mode == "http":
                raise
            log(f"http blocked ({b}); switching to headless browser")
            fetcher.close()
            fetcher = make_fetcher("browser", delay)
            col = collect(fetcher.fetch, days, products, known_hosts, max_products)
    finally:
        fetcher.close()
    return col.entries()


# ---------------------------------------------------------------------------
# API side.
# ---------------------------------------------------------------------------


def api_json(api: str, path: str, secret: str, body: dict | None = None) -> dict:
    data = json.dumps(body).encode() if body is not None else None
    req = urllib.request.Request(
        f"{api.rstrip('/')}{path}",
        data=data,
        headers={"Content-Type": "application/json", "X-Auctioning-Ingest": secret},
        method="POST" if data is not None else "GET",
    )
    with urllib.request.urlopen(req, timeout=120) as resp:
        return json.loads(resp.read().decode() or "{}")


def known_hosts_from_api(api: str, secret: str) -> set[str]:
    try:
        return set(api_json(api, "/v1/outbid/hosts", secret).get("hosts", []))
    except Exception as e:  # noqa: BLE001
        log(f"known hosts unavailable ({e}); fetching every product page")
        return set()


def push(api: str, secret: str, entries: list[dict]) -> int:
    collected_at = dt.datetime.now(dt.timezone.utc).isoformat()
    total = {"created": 0, "updated": 0, "rp_credited": 0, "skipped": 0}
    for i in range(0, len(entries), MAX_BATCH):
        batch = entries[i : i + MAX_BATCH]
        try:
            out = api_json(
                api,
                "/v1/outbid/sync",
                secret,
                {"collector": COLLECTOR, "collected_at": collected_at, "entries": batch},
            )
        except urllib.error.HTTPError as e:
            log(f"sync failed: HTTP {e.code}: {e.read().decode(errors='replace')}")
            return 2
        for k in total:
            total[k] += int(out.get(k, 0))
        log(f"pushed {len(batch)}: {out}")
    log(f"sync ok: {total}")
    return 0


def load_snapshot(path: str) -> list[dict]:
    with open(path, encoding="utf-8") as fh:
        data = json.load(fh)
    return data["entries"] if isinstance(data, dict) else list(data)


# ---------------------------------------------------------------------------


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--api", default=os.environ.get("AUCTIONING_API", ""), help="auctioning API base URL")
    ap.add_argument("--push", action="store_true", help="POST to /v1/outbid/sync (needs INGEST_SECRET)")
    ap.add_argument("--snapshot", help="replay a saved collection instead of crawling")
    ap.add_argument("--out", help="also write the collection to this JSON file")
    ap.add_argument("--mode", choices=["auto", "http", "browser"], default=os.environ.get("OUTBID_FETCH_MODE", "auto"))
    ap.add_argument("--delay", type=float, default=float(os.environ.get("OUTBID_FETCH_DELAY", "0.3")), help="seconds between fetches")
    ap.add_argument("--days", type=int, default=int(os.environ.get("OUTBID_DAILY_DAYS", "7")), help="daily boards to include")
    ap.add_argument("--products", action="store_true", default=os.environ.get("OUTBID_PRODUCTS", "1") == "1",
                    help="fetch sitemap product pages not seen on a board (default on)")
    ap.add_argument("--no-products", dest="products", action="store_false")
    ap.add_argument("--max-products", type=int, default=int(os.environ.get("OUTBID_MAX_PRODUCTS", "3000")))
    ap.add_argument("--loop", action="store_true", help="run forever (autosync)")
    ap.add_argument("--interval", type=int, default=int(os.environ.get("OUTBID_SYNC_INTERVAL_SECS", "3600")))
    args = ap.parse_args()

    secret = os.environ.get("INGEST_SECRET", "")
    if args.push and not args.api:
        ap.error("--push requires --api or AUCTIONING_API")
    if args.push and not secret:
        ap.error("--push requires INGEST_SECRET")

    while True:
        started = time.monotonic()
        rc = 0
        try:
            if args.snapshot:
                entries = load_snapshot(args.snapshot)
            else:
                known = known_hosts_from_api(args.api, secret) if (args.push and args.products) else set()
                entries = run_collection(args.mode, args.delay, args.days, args.products, known, args.max_products)
            with_amount = sum(1 for e in entries if e.get("amount_cents", 0) > 0)
            dollars = sum(e.get("amount_cents", 0) for e in entries) / 100
            log(f"collected {len(entries)} businesses, {with_amount} with a dollar value (${dollars:,.0f} total)")
            if args.out:
                with open(args.out, "w", encoding="utf-8") as fh:
                    json.dump({"collector": COLLECTOR, "collected_at": dt.datetime.now(dt.timezone.utc).isoformat(), "entries": entries}, fh, indent=1)
            if args.push:
                rc = push(args.api, secret, entries)
            else:
                json.dump({"collector": COLLECTOR, "entries": entries}, sys.stdout, indent=1)
                print(file=sys.stdout)
        except Blocked as b:
            log(f"blocked by outbid.lol checkpoint: {b}")
            rc = 3
        except Exception as e:  # noqa: BLE001
            log(f"collection failed: {type(e).__name__}: {e}")
            rc = 1
        if not args.loop:
            return rc
        wait = max(60, args.interval - int(time.monotonic() - started))
        log(f"next run in {wait}s")
        time.sleep(wait)


if __name__ == "__main__":
    raise SystemExit(main())
