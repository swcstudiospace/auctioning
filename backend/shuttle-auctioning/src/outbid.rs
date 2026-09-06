//! outbid.lol mirror: every ranking entry on outbid.lol becomes a catalog
//! project, and the dollars it has paid there are mirrored into race RP at the
//! advertised 1 RP = $1 rate.
//!
//! The collector (`tools/outbid/outbid_collector.py`) scrapes outbid.lol —
//! which sits behind Vercel's bot checkpoint, so a real browser does the
//! fetching — and POSTs `/v1/outbid/sync` with the ingest secret. This module
//! is the only writer; it never talks to outbid.lol itself.
//!
//! Money path: `mirrored_rp` on the project row is the RP already credited
//! from outbid dollars. Each sync credits only `floor(amount_cents / 100) -
//! mirrored_rp` when that is positive, as an `outbid_mirror` allocation from
//! the fixed [`MIRROR_WALLET`]. The allocation ledger stays append-only and
//! community fuel added on auctioning.lol is never touched. outbid amounts
//! only ever rise (a rank is *claimed* by paying more), so a decrease is
//! logged and ignored rather than clawed back.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

/// Synthetic supporter for mirror allocations. Base58-safe (no 0/O/I/l),
/// 44 chars, never a real key. Inserted by migration 0010.
pub const MIRROR_WALLET: &str = "outbidmirror1111111111111111111111111111111";

/// Largest batch one push may carry (mirrors the catalog import cap).
pub const MAX_BATCH: usize = 5_000;

/// Advertised rate: $1 buys 1 RP (see `handlers::rp_from_cents`).
pub fn rp_from_cents(cents: i64) -> i64 {
    cents.max(0) / 100
}

/// One outbid.lol ranking entry as the collector reports it.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct OutbidEntry {
    /// outbid's ranking-entry uuid.
    pub id: String,
    /// e.g. "website:see.io". Optional when only a URL is known.
    #[serde(default)]
    pub identity_key: Option<String>,
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub source_url: Option<String>,
    #[serde(default)]
    pub product_url: Option<String>,
    #[serde(default)]
    pub image_url: Option<String>,
    #[serde(default)]
    pub category_slug: Option<String>,
    #[serde(default)]
    pub category_name: Option<String>,
    /// Dollars paid on outbid.lol, in cents. 0 when unknown (product page only).
    #[serde(default)]
    pub amount_cents: i64,
    #[serde(default)]
    pub click_count: Option<i64>,
    #[serde(default)]
    pub created_at: Option<DateTime<Utc>>,
    /// Position on outbid's all-time board, when the collector saw it there.
    #[serde(default)]
    pub rank: Option<i32>,
    #[serde(default)]
    pub category_rank: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct SyncRequest {
    #[serde(default)]
    pub collector: Option<String>,
    #[serde(default)]
    pub collected_at: Option<DateTime<Utc>>,
    pub entries: Vec<OutbidEntry>,
}

#[derive(Debug, Serialize)]
pub struct SyncOutcome {
    pub run_id: Uuid,
    pub entries_seen: usize,
    pub created: usize,
    pub updated: usize,
    pub rp_credited: i64,
    pub amount_cents_total: i64,
    /// Entries that could not be keyed to a host (no identity/url).
    pub skipped: usize,
}

/// Derive the listing host from what outbid gives us. Lowercase, no `www.`,
/// no port, no path. Returns None when nothing usable is present.
pub fn listing_host(entry: &OutbidEntry) -> Option<String> {
    let from_identity = entry
        .identity_key
        .as_deref()
        .and_then(|k| k.split_once(':').map(|(_, v)| v).or(Some(k)))
        .and_then(clean_host);
    if from_identity.is_some() {
        return from_identity;
    }
    entry
        .source_url
        .as_deref()
        .and_then(host_of_url)
        .or_else(|| {
            entry
                .product_url
                .as_deref()
                .and_then(|u| u.trim_end_matches('/').rsplit('/').next())
                .and_then(clean_host)
        })
}

fn host_of_url(url: &str) -> Option<String> {
    let rest = url.split("://").nth(1).unwrap_or(url);
    let hostport = rest.split(['/', '?', '#']).next().unwrap_or("");
    let host = hostport.rsplit('@').next().unwrap_or(hostport);
    clean_host(host.split(':').next().unwrap_or(host))
}

fn clean_host(raw: &str) -> Option<String> {
    let h = raw.trim().trim_matches('.').to_ascii_lowercase();
    let h = h.strip_prefix("www.").unwrap_or(&h).to_string();
    if h.is_empty()
        || h.len() > 253
        || !h.contains('.')
        || !h
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-')
    {
        return None;
    }
    Some(h)
}

/// Same convention as `catalog::submit_site`: `outbid:<host>` / `<host with
/// dots as dashes>`, capped at the 32-char handle limit.
pub fn keys_for_host(host: &str) -> (String, String) {
    let stable_id = format!("outbid:{host}");
    let mut handle: String = host
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '-'
            }
        })
        .collect();
    handle.truncate(32);
    while handle.len() < 3 {
        handle.push('0');
    }
    (stable_id, handle)
}

fn clip(s: Option<&str>, max: usize) -> Option<String> {
    s.map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.chars().take(max).collect())
}

/// Apply one collector push. Atomic: either every entry lands or none do,
/// and the audit row records the outcome either way.
pub async fn apply_sync(db: &PgPool, req: &SyncRequest) -> Result<SyncOutcome, sqlx::Error> {
    if req.entries.len() > MAX_BATCH {
        return Err(sqlx::Error::Configuration(
            format!("batch too large (max {MAX_BATCH})").into(),
        ));
    }
    let collector = req.collector.as_deref().unwrap_or("unknown");
    let run_id: Uuid = sqlx::query_scalar(
        "INSERT INTO outbid_sync_runs (collector, collected_at, entries_seen) VALUES ($1, $2, $3) RETURNING id",
    )
    .bind(collector.chars().take(64).collect::<String>())
    .bind(req.collected_at)
    .bind(req.entries.len() as i32)
    .fetch_one(db)
    .await?;

    match apply_inner(db, req).await {
        Ok(mut outcome) => {
            outcome.run_id = run_id;
            sqlx::query(
                r#"
                UPDATE outbid_sync_runs SET finished_at = now(), status = 'ok',
                    projects_created = $2, projects_updated = $3, rp_credited = $4,
                    amount_cents_total = $5
                WHERE id = $1
                "#,
            )
            .bind(run_id)
            .bind(outcome.created as i32)
            .bind(outcome.updated as i32)
            .bind(outcome.rp_credited)
            .bind(outcome.amount_cents_total)
            .execute(db)
            .await?;
            if outcome.rp_credited > 0 {
                crate::race_engine::invalidate_lifetime_cache();
            }
            Ok(outcome)
        }
        Err(e) => {
            let msg: String = e.to_string().chars().take(500).collect();
            let _ = sqlx::query(
                "UPDATE outbid_sync_runs SET finished_at = now(), status = 'failed', error = $2 WHERE id = $1",
            )
            .bind(run_id)
            .bind(msg)
            .execute(db)
            .await;
            Err(e)
        }
    }
}

async fn apply_inner(db: &PgPool, req: &SyncRequest) -> Result<SyncOutcome, sqlx::Error> {
    let mut tx = db.begin().await?;
    let mut outcome = SyncOutcome {
        run_id: Uuid::nil(),
        entries_seen: req.entries.len(),
        created: 0,
        updated: 0,
        rp_credited: 0,
        amount_cents_total: 0,
        skipped: 0,
    };

    // Collapse duplicates inside one push (an entry can appear on the
    // all-time board, its category board and a daily board): keep the
    // highest amount and the richest metadata.
    let mut seen: std::collections::HashMap<String, OutbidEntry> = Default::default();
    let mut order: Vec<String> = Vec::new();
    for e in &req.entries {
        let Some(host) = listing_host(e) else {
            outcome.skipped += 1;
            continue;
        };
        match seen.get_mut(&host) {
            None => {
                order.push(host.clone());
                seen.insert(host, e.clone());
            }
            Some(kept) => merge_into(kept, e),
        }
    }

    for host in order {
        let e = &seen[&host];
        let (stable_id, handle) = keys_for_host(&host);
        let amount_cents = e.amount_cents.max(0);
        outcome.amount_cents_total += amount_cents;
        let tags: Vec<String> = e
            .category_slug
            .as_deref()
            .map(catalog_tag)
            .filter(|s| !s.is_empty())
            .into_iter()
            .collect();
        let url = e
            .source_url
            .as_deref()
            .and_then(|u| clip(Some(u), 2048))
            .or_else(|| Some(format!("https://{host}/")));

        // Adopt any existing row for this host: an earlier outbid import, a
        // manual submit of the same site, or an entry-id match after a rename.
        let existing: Option<(String, i64, i64)> = sqlx::query_as(
            r#"
            SELECT handle, total_rp, mirrored_rp FROM projects
            WHERE stable_id = $1
               OR outbid_entry_id = $2
               OR handle = $3
               OR stable_id = $4
               OR lower(trim(trailing '/' from coalesce(url, ''))) = $5
            ORDER BY (stable_id = $1) DESC, (outbid_entry_id = $2) DESC
            LIMIT 1
            FOR UPDATE
            "#,
        )
        .bind(&stable_id)
        .bind(&e.id)
        .bind(&handle)
        .bind(format!("manual:{host}"))
        .bind(format!("https://{host}"))
        .fetch_optional(&mut *tx)
        .await?;

        let (project_handle, mirrored_before) = match existing {
            Some((h, _total, mirrored)) => {
                sqlx::query(
                    r#"
                    UPDATE projects SET
                        display_name = COALESCE($2, display_name),
                        blurb = COALESCE($3, blurb),
                        url = COALESCE(url, $4),
                        image_url = COALESCE($5, image_url),
                        tags = CASE WHEN $6::text[] = '{}' THEN tags
                                    ELSE (SELECT array_agg(DISTINCT t) FROM unnest(tags || $6) t) END,
                        source_ref = COALESCE($7, source_ref),
                        outbid_entry_id = $8,
                        outbid_amount_cents = GREATEST(outbid_amount_cents, $9),
                        outbid_rank = COALESCE($10, outbid_rank),
                        outbid_category = COALESCE($11, outbid_category),
                        outbid_category_rank = COALESCE($12, outbid_category_rank),
                        outbid_clicks = GREATEST(outbid_clicks, COALESCE($13, 0)),
                        outbid_listed_at = COALESCE(outbid_listed_at, $14),
                        outbid_synced_at = now()
                    WHERE handle = $1
                    "#,
                )
                .bind(&h)
                .bind(clip(e.display_name.as_deref(), 200))
                .bind(clip(e.description.as_deref(), 1000))
                .bind(&url)
                .bind(clip(e.image_url.as_deref(), 2048))
                .bind(&tags)
                .bind(clip(e.product_url.as_deref(), 2048))
                .bind(&e.id)
                .bind(amount_cents)
                .bind(e.rank)
                .bind(clip(e.category_slug.as_deref(), 64))
                .bind(e.category_rank)
                .bind(e.click_count)
                .bind(e.created_at)
                .execute(&mut *tx)
                .await?;
                outcome.updated += 1;
                (h, mirrored)
            }
            None => {
                sqlx::query(
                    r#"
                    INSERT INTO projects
                        (handle, source, source_ref, display_name, blurb, stable_id, url,
                         image_url, tags, total_rp, outbid_entry_id, outbid_amount_cents,
                         outbid_rank, outbid_category, outbid_category_rank, outbid_clicks,
                         outbid_listed_at, outbid_synced_at)
                    VALUES ($1, 'outbid_import', $2, $3, $4, $5, $6, $7, $8, 0, $9, $10,
                            $11, $12, $13, COALESCE($14, 0), $15, now())
                    "#,
                )
                .bind(&handle)
                .bind(clip(e.product_url.as_deref(), 2048))
                .bind(clip(e.display_name.as_deref(), 200).or_else(|| Some(host.clone())))
                .bind(clip(e.description.as_deref(), 1000))
                .bind(&stable_id)
                .bind(&url)
                .bind(clip(e.image_url.as_deref(), 2048))
                .bind(&tags)
                .bind(&e.id)
                .bind(amount_cents)
                .bind(e.rank)
                .bind(clip(e.category_slug.as_deref(), 64))
                .bind(e.category_rank)
                .bind(e.click_count)
                .bind(e.created_at)
                .execute(&mut *tx)
                .await?;
                outcome.created += 1;
                (handle.clone(), 0)
            }
        };

        // Mirror dollars → RP. Only the increase is credited.
        let target_rp = rp_from_cents(amount_cents);
        let delta = target_rp - mirrored_before;
        if delta > 0 {
            sqlx::query(
                r#"
                INSERT INTO project_allocations
                    (project_handle, supporter_wallet, amount, bucket, source)
                VALUES ($1, $2, $3, 'paid', 'outbid_mirror')
                "#,
            )
            .bind(&project_handle)
            .bind(MIRROR_WALLET)
            .bind(delta)
            .execute(&mut *tx)
            .await?;
            sqlx::query(
                "UPDATE projects SET total_rp = total_rp + $2, mirrored_rp = $3 WHERE handle = $1",
            )
            .bind(&project_handle)
            .bind(delta)
            .bind(target_rp)
            .execute(&mut *tx)
            .await?;
            outcome.rp_credited += delta;
        } else if delta < 0 {
            tracing::warn!(
                handle = %project_handle,
                mirrored = mirrored_before,
                target = target_rp,
                "outbid amount decreased; ledger is append-only, keeping mirrored rp"
            );
        }
    }

    tx.commit().await?;
    Ok(outcome)
}

fn merge_into(kept: &mut OutbidEntry, other: &OutbidEntry) {
    if other.amount_cents > kept.amount_cents {
        kept.amount_cents = other.amount_cents;
    }
    macro_rules! fill {
        ($f:ident) => {
            if kept.$f.is_none() {
                kept.$f = other.$f.clone();
            }
        };
    }
    fill!(identity_key);
    fill!(display_name);
    fill!(description);
    fill!(source_url);
    fill!(product_url);
    fill!(image_url);
    fill!(category_slug);
    fill!(category_name);
    fill!(created_at);
    fill!(category_rank);
    kept.rank = match (kept.rank, other.rank) {
        (Some(a), Some(b)) => Some(a.min(b)),
        (a, b) => a.or(b),
    };
    kept.click_count = match (kept.click_count, other.click_count) {
        (Some(a), Some(b)) => Some(a.max(b)),
        (a, b) => a.or(b),
    };
}

/// outbid category slugs are already `[a-z0-9-]`; keep them as-is but apply
/// the catalog's tag hygiene so `?tag=` filters line up.
fn catalog_tag(slug: &str) -> String {
    slug.trim()
        .to_ascii_lowercase()
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '-'
            }
        })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
        .chars()
        .take(48)
        .collect()
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct SyncRun {
    pub id: Uuid,
    pub collector: String,
    pub collected_at: Option<DateTime<Utc>>,
    pub started_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
    pub entries_seen: i32,
    pub projects_created: i32,
    pub projects_updated: i32,
    pub rp_credited: i64,
    pub amount_cents_total: i64,
    pub status: String,
    pub error: Option<String>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct MirrorTotals {
    pub projects: i64,
    pub amount_cents: i64,
    pub mirrored_rp: i64,
    pub last_synced_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize)]
pub struct SyncStatus {
    pub totals: MirrorTotals,
    pub last_run: Option<SyncRun>,
    pub recent_runs: Vec<SyncRun>,
    pub rate: &'static str,
}

/// Public read model for `/v1/outbid/status`.
pub async fn status(db: &PgPool) -> Result<SyncStatus, sqlx::Error> {
    let totals = sqlx::query_as::<_, MirrorTotals>(
        r#"
        SELECT COUNT(*)::bigint                        AS projects,
               COALESCE(SUM(outbid_amount_cents), 0)::bigint AS amount_cents,
               COALESCE(SUM(mirrored_rp), 0)::bigint   AS mirrored_rp,
               MAX(outbid_synced_at)                   AS last_synced_at
        FROM projects WHERE outbid_entry_id IS NOT NULL
        "#,
    )
    .fetch_one(db)
    .await?;
    let recent_runs = sqlx::query_as::<_, SyncRun>(
        r#"
        SELECT id, collector, collected_at, started_at, finished_at, entries_seen,
               projects_created, projects_updated, rp_credited, amount_cents_total,
               status, error
        FROM outbid_sync_runs ORDER BY started_at DESC LIMIT 10
        "#,
    )
    .fetch_all(db)
    .await?;
    let last_run = recent_runs.first().map(|r| SyncRun {
        id: r.id,
        collector: r.collector.clone(),
        collected_at: r.collected_at,
        started_at: r.started_at,
        finished_at: r.finished_at,
        entries_seen: r.entries_seen,
        projects_created: r.projects_created,
        projects_updated: r.projects_updated,
        rp_credited: r.rp_credited,
        amount_cents_total: r.amount_cents_total,
        status: r.status.clone(),
        error: r.error.clone(),
    });
    Ok(SyncStatus {
        totals,
        last_run,
        recent_runs,
        rate: "1 RP = $1 (floor of outbid amount)",
    })
}

/// Hosts already mirrored, so the collector can skip product pages it has
/// seen. Cheap: one indexed scan.
pub async fn known_hosts(db: &PgPool) -> Result<Vec<String>, sqlx::Error> {
    let rows: Vec<(String,)> = sqlx::query_as(
        "SELECT substring(stable_id from 8) FROM projects WHERE stable_id LIKE 'outbid:%' AND outbid_entry_id IS NOT NULL",
    )
    .fetch_all(db)
    .await?;
    Ok(rows.into_iter().map(|r| r.0).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(identity: Option<&str>, url: Option<&str>) -> OutbidEntry {
        OutbidEntry {
            id: "e1".into(),
            identity_key: identity.map(String::from),
            display_name: None,
            description: None,
            source_url: url.map(String::from),
            product_url: None,
            image_url: None,
            category_slug: None,
            category_name: None,
            amount_cents: 0,
            click_count: None,
            created_at: None,
            rank: None,
            category_rank: None,
        }
    }

    #[test]
    fn host_prefers_identity_then_url() {
        assert_eq!(
            listing_host(&entry(Some("website:See.io"), None)).as_deref(),
            Some("see.io")
        );
        assert_eq!(
            listing_host(&entry(None, Some("https://www.tutti.so/join?utm=x"))).as_deref(),
            Some("tutti.so")
        );
        assert_eq!(listing_host(&entry(None, Some("not a url"))), None);
        let mut p = entry(None, None);
        p.product_url = Some("https://outbid.lol/product/joni.ai".into());
        assert_eq!(listing_host(&p).as_deref(), Some("joni.ai"));
    }

    #[test]
    fn keys_match_submit_site_convention() {
        let (sid, handle) = keys_for_host("see.io");
        assert_eq!(sid, "outbid:see.io");
        assert_eq!(handle, "see-io");
        let (_, long) = keys_for_host(&format!("{}.com", "a".repeat(60)));
        assert_eq!(long.len(), 32);
    }

    #[test]
    fn rp_is_floor_of_dollars() {
        assert_eq!(rp_from_cents(1_700_000), 17_000);
        assert_eq!(rp_from_cents(199), 1);
        assert_eq!(rp_from_cents(-5), 0);
    }

    #[test]
    fn merge_keeps_max_amount_and_best_rank() {
        let mut a = entry(Some("website:a.io"), None);
        a.amount_cents = 100;
        a.rank = Some(7);
        let mut b = entry(Some("website:a.io"), None);
        b.amount_cents = 500;
        b.rank = Some(3);
        b.description = Some("desc".into());
        merge_into(&mut a, &b);
        assert_eq!(a.amount_cents, 500);
        assert_eq!(a.rank, Some(3));
        assert_eq!(a.description.as_deref(), Some("desc"));
    }

    #[test]
    fn mirror_wallet_is_base58_safe() {
        assert!(crate::ledger::valid_wallet(MIRROR_WALLET));
    }
}
