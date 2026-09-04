//! Project catalog + seeding for auctioning.lol.
//!
//! The `projects` table is the import target for outbid.lol listings (and any
//! other public board). Imports are idempotent upserts on `stable_id`, so
//! reseeds are safe. `project_allocations` is the immutable per-project RP
//! ledger that rank/velocity/overtake detection will read; free-RP
//! allocations carry the exact `lot_id` they drained for end-to-end
//! provenance.

use crate::ledger::{RpSource, WalletError, MAX_WALLET_LEN};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize)]
pub struct Project {
    pub handle: String,
    pub owner_wallet: Option<String>,
    pub source: String,
    pub source_ref: Option<String>,
    pub display_name: Option<String>,
    pub blurb: Option<String>,
    pub stable_id: Option<String>,
    pub url: Option<String>,
    pub tags: Vec<String>,
    pub total_rp: i64,
    #[serde(default)]
    pub clicks: i64,
}

/// One incoming listing from a board snapshot / import payload.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct ImportProject {
    /// Deterministic import key, e.g. "outbid:beanz-coffee-brisbane".
    pub stable_id: String,
    /// URL/handle slug; defaults to a sanitized stable_id suffix.
    #[serde(default)]
    pub handle: Option<String>,
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub blurb: Option<String>,
    /// Category tags for auto-assignment into tracks.
    #[serde(default)]
    pub tags: Vec<String>,
    /// Which board this came from: outbid_import | board_import | manual.
    #[serde(default)]
    pub source: Option<String>,
    /// Free-form pointer back to the snapshot row.
    #[serde(default)]
    pub source_ref: Option<String>,
    /// Owner wallet if already known (optional).
    #[serde(default)]
    pub owner_wallet: Option<String>,
}

impl ImportProject {
    fn derived_handle(&self) -> String {
        let raw = self.handle.clone().unwrap_or_else(|| {
            self.stable_id
                .rsplit(':')
                .next()
                .unwrap_or(&self.stable_id)
                .to_string()
        });
        // Handle hygiene: lowercase [a-z0-9-_], capped at 32 chars to mirror
        // the on-chain handle limit.
        let mut cleaned: String = raw
            .to_lowercase()
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                    c
                } else {
                    '-'
                }
            })
            .collect();
        cleaned.truncate(32);
        while cleaned.len() < 3 {
            cleaned.push('0');
        }
        cleaned
    }

    fn normalized_source(&self) -> &'static str {
        match self.source.as_deref() {
            Some("outbid_snapshot") => "outbid_snapshot",
            Some("board_import") => "board_import",
            Some("manual") => "manual",
            _ => "outbid_import",
        }
    }
}

pub const MAX_TAGS: usize = 12;
const MAX_TAG_LEN: usize = 48;

fn clean_tags(tags: &[String]) -> Vec<String> {
    let mut out: Vec<String> = tags
        .iter()
        .map(|t| {
            t.trim()
                .to_lowercase()
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
                .take(MAX_TAG_LEN)
                .collect::<String>()
        })
        .filter(|t: &String| !t.is_empty())
        .take(MAX_TAGS)
        .collect();
    out.sort();
    out.dedup();
    out
}

fn valid_wallet(w: &str) -> bool {
    !(w.is_empty() || w.len() > MAX_WALLET_LEN)
}

/// Upsert a batch of listings. Returns (imported, updated) counts.
/// Idempotent on `stable_id`; existing rows keep their owner/RP and only
/// descriptive fields get refreshed.
pub async fn import_projects(
    db: &PgPool,
    items: &[ImportProject],
) -> Result<(usize, usize), sqlx::Error> {
    let mut tx = db.begin().await?;
    let mut imported = 0usize;
    let mut updated = 0usize;

    for item in items {
        if item.stable_id.trim().is_empty() || item.stable_id.len() > 200 {
            return Err(sqlx::Error::Configuration(
                "stable_id must be 1-200 chars".into(),
            ));
        }
        if let Some(w) = &item.owner_wallet {
            if !crate::ledger::valid_wallet(w) {
                return Err(WalletError::Invalid.into());
            }
            // Ensure the wallet row exists so the FK holds.
            sqlx::query("INSERT INTO wallets (wallet) VALUES ($1) ON CONFLICT DO NOTHING")
                .bind(w)
                .execute(&mut *tx)
                .await?;
        }

        let handle = item.derived_handle();
        let source = item.normalized_source();
        let tags = clean_tags(&item.tags);

        // Does a project with this stable_id (or colliding handle) exist?
        let existing: Option<(Option<String>,)> =
            sqlx::query_as("SELECT stable_id FROM projects WHERE stable_id = $1 OR handle = $2")
                .bind(&item.stable_id)
                .bind(&handle)
                .fetch_optional(&mut *tx)
                .await?;

        if existing.is_some() {
            sqlx::query(
                r#"
                UPDATE projects SET
                    display_name = COALESCE($3, display_name),
                    url = COALESCE($4, url),
                    blurb = COALESCE($5, blurb),
                    tags = CASE WHEN $6::text[] = '{}' THEN tags ELSE $6 END,
                    source_ref = COALESCE($7, source_ref),
                    owner_wallet = COALESCE(owner_wallet, $8),
                    stable_id = COALESCE(stable_id, $1)
                WHERE stable_id = $1 OR handle = $2
                "#,
            )
            .bind(&item.stable_id)
            .bind(&handle)
            .bind(&item.display_name)
            .bind(&item.url)
            .bind(&item.blurb)
            .bind(&tags)
            .bind(&item.source_ref)
            .bind(&item.owner_wallet)
            .execute(&mut *tx)
            .await?;
            updated += 1;
        } else {
            sqlx::query(
                r#"
                INSERT INTO projects
                    (handle, owner_wallet, source, source_ref, display_name, blurb,
                     stable_id, url, tags, total_rp)
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, 0)
                "#,
            )
            .bind(&handle)
            .bind(&item.owner_wallet)
            .bind(source)
            .bind(&item.source_ref)
            .bind(&item.display_name)
            .bind(&item.blurb)
            .bind(&item.stable_id)
            .bind(&item.url)
            .bind(&tags)
            .execute(&mut *tx)
            .await?;
            imported += 1;
        }
    }

    tx.commit().await?;
    Ok((imported, updated))
}

/// Load a seed file (JSON array of ImportProject) — convenience wrapper used
/// by the CLI seeding script and the import endpoint's file path.
pub fn parse_import_payload(json: &str) -> Result<Vec<ImportProject>, serde_json::Error> {
    serde_json::from_str(json)
}

/// Public listing: one URL, starts at 0 RP. Never overwrites an existing row
/// (outbid import or a prior submit). Returns the existing project instead.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct SubmitSite {
    pub url: String,
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(default)]
    pub blurb: Option<String>,
    #[serde(default)]
    pub owner_wallet: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Debug, serde::Serialize)]
pub struct SubmitOutcome {
    pub created: bool,
    pub project: Project,
}

fn parse_site_url(raw: &str) -> Result<(String, String), String> {
    let s = raw.trim();
    if s.is_empty() || s.len() > 2048 {
        return Err("url must be 1-2048 chars".into());
    }
    if s.contains(char::is_whitespace) {
        return Err("url must not contain spaces".into());
    }
    let with_scheme = if s.contains("://") {
        s.to_string()
    } else {
        format!("https://{s}")
    };
    let lower = with_scheme.to_lowercase();
    if !(lower.starts_with("http://") || lower.starts_with("https://")) {
        return Err("url must be http or https".into());
    }
    let rest = lower.split("://").nth(1).unwrap_or("");
    let hostport = rest
        .split('/')
        .next()
        .unwrap_or("")
        .split('?')
        .next()
        .unwrap_or("");
    if hostport.contains('@') {
        return Err("url must not include credentials".into());
    }
    let host_only = hostport.split(':').next().unwrap_or("").trim_matches('.');
    let host = host_only.strip_prefix("www.").unwrap_or(host_only);
    if host.is_empty() || !host.contains('.') || host.len() > 253 {
        return Err("url host looks invalid".into());
    }
    if !host
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-')
    {
        return Err("url host looks invalid".into());
    }
    if host == "localhost"
        || host.ends_with(".local")
        || host.ends_with(".internal")
        || host.starts_with("127.")
        || host.starts_with("10.")
        || host.starts_with("192.168.")
        || host.starts_with("169.254.")
    {
        return Err("that host cannot be ranked".into());
    }
    if host.chars().all(|c| c.is_ascii_digit() || c == '.') {
        return Err("ip addresses cannot be ranked".into());
    }
    Ok((format!("https://{host}/"), host.to_string()))
}

pub async fn submit_site(db: &PgPool, req: SubmitSite) -> Result<SubmitOutcome, sqlx::Error> {
    let (canonical, host) =
        parse_site_url(&req.url).map_err(|e| sqlx::Error::Configuration(e.into()))?;

    let item = ImportProject {
        stable_id: format!("manual:{host}"),
        handle: Some(host.replace('.', "-")),
        display_name: req
            .display_name
            .as_ref()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .or_else(|| Some(host.clone())),
        url: Some(canonical.clone()),
        blurb: req
            .blurb
            .as_ref()
            .map(|s| s.trim().chars().take(280).collect::<String>())
            .filter(|s| !s.is_empty()),
        tags: req.tags,
        source: Some("manual".into()),
        source_ref: Some(canonical.clone()),
        owner_wallet: req
            .owner_wallet
            .as_ref()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty()),
    };
    if let Some(w) = &item.owner_wallet {
        if !valid_wallet(w) {
            return Err(WalletError::Invalid.into());
        }
    }

    let mut handle = item.derived_handle();
    let tags = clean_tags(&item.tags);
    let outbid_id = format!("outbid:{host}");

    let mut tx = db.begin().await?;

    let existing: Option<Project> = sqlx::query_as::<_, Project>(
        r#"
        SELECT handle, owner_wallet, source, source_ref, display_name, blurb,
               stable_id, url, tags, total_rp, clicks
        FROM projects
        WHERE handle = $1
           OR stable_id = $2
           OR stable_id = $3
           OR lower(trim(trailing '/' from coalesce(url, '')))
              = lower(trim(trailing '/' from $4::text))
        LIMIT 1
        "#,
    )
    .bind(&handle)
    .bind(&item.stable_id)
    .bind(&outbid_id)
    .bind(&canonical)
    .fetch_optional(&mut *tx)
    .await?;

    if let Some(project) = existing {
        let existing_host = project
            .url
            .as_deref()
            .and_then(|u| parse_site_url(u).ok().map(|(_, h)| h));
        let same_host = existing_host.as_deref() == Some(host.as_str())
            || project.stable_id.as_deref() == Some(item.stable_id.as_str())
            || project.stable_id.as_deref() == Some(outbid_id.as_str());
        if same_host {
            tx.commit().await?;
            return Ok(SubmitOutcome {
                created: false,
                project,
            });
        }
        let mut n = 2u32;
        loop {
            let mut base = handle.clone();
            if base.len() > 30 {
                base.truncate(30);
            }
            let candidate = format!("{base}-{n}");
            let taken: Option<(String,)> =
                sqlx::query_as("SELECT handle FROM projects WHERE handle = $1")
                    .bind(&candidate)
                    .fetch_optional(&mut *tx)
                    .await?;
            if taken.is_none() {
                handle = candidate;
                break;
            }
            n += 1;
            if n > 50 {
                return Err(sqlx::Error::Configuration(
                    "could not allocate handle".into(),
                ));
            }
        }
    }

    if let Some(w) = &item.owner_wallet {
        sqlx::query("INSERT INTO wallets (wallet) VALUES ($1) ON CONFLICT DO NOTHING")
            .bind(w)
            .execute(&mut *tx)
            .await?;
    }

    sqlx::query(
        r#"
        INSERT INTO projects
            (handle, owner_wallet, source, source_ref, display_name, blurb,
             stable_id, url, tags, total_rp)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, 0)
        "#,
    )
    .bind(&handle)
    .bind(&item.owner_wallet)
    .bind("manual")
    .bind(&item.source_ref)
    .bind(&item.display_name)
    .bind(&item.blurb)
    .bind(&item.stable_id)
    .bind(&canonical)
    .bind(&tags)
    .execute(&mut *tx)
    .await?;

    let project = sqlx::query_as::<_, Project>(
        r#"
        SELECT handle, owner_wallet, source, source_ref, display_name, blurb,
               stable_id, url, tags, total_rp, clicks
        FROM projects WHERE handle = $1
        "#,
    )
    .bind(&handle)
    .fetch_one(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(SubmitOutcome {
        created: true,
        project,
    })
}

#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize)]
pub struct ProjectWithRank {
    pub handle: String,
    pub owner_wallet: Option<String>,
    pub source: String,
    pub source_ref: Option<String>,
    pub display_name: Option<String>,
    pub blurb: Option<String>,
    pub stable_id: Option<String>,
    pub url: Option<String>,
    pub tags: Vec<String>,
    pub total_rp: i64,
    pub rank: i64,
    #[serde(default)]
    pub clicks: i64,
}

/// Ranked board: highest total_rp first, ties broken by earliest creation.
/// Legacy helper used by tests and any caller that still wants a capped slice.
pub async fn list_projects(db: &PgPool, limit: i64) -> Result<Vec<ProjectWithRank>, sqlx::Error> {
    let page = list_projects_page_inner(db, 1, limit.clamp(1, 500), None, None, true).await?;
    Ok(page.projects)
}

/// Paginated board with optional tag / free-text filters.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ProjectListPage {
    pub projects: Vec<ProjectWithRank>,
    pub total: i64,
    pub page: i64,
    pub per_page: i64,
    pub tags: Vec<String>,
}

pub async fn list_projects_page(
    db: &PgPool,
    page: i64,
    per_page: i64,
    tag: Option<&str>,
    q: Option<&str>,
) -> Result<ProjectListPage, sqlx::Error> {
    list_projects_page_inner(db, page, per_page, tag, q, false).await
}

async fn list_projects_page_inner(
    db: &PgPool,
    page: i64,
    per_page: i64,
    tag: Option<&str>,
    q: Option<&str>,
    allow_large: bool,
) -> Result<ProjectListPage, sqlx::Error> {
    let page = page.max(1);
    let max_per = if allow_large { 500 } else { 50 };
    let per_page = per_page.clamp(1, max_per);
    let offset = (page - 1).saturating_mul(per_page);

    let tag_bind: Option<String> = tag
        .map(|t| clean_tags(&[t.to_string()]))
        .and_then(|mut v| v.pop());
    let q_bind: Option<String> = q
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.chars().take(80).collect::<String>());

    let total: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)::bigint
        FROM projects p
        WHERE ($1::text IS NULL OR $1 = ANY(p.tags))
          AND (
            $2::text IS NULL
            OR position(
                 lower($2) in lower(
                   coalesce(p.display_name, '') || ' ' || p.handle || ' '
                   || coalesce(p.blurb, '') || ' ' || coalesce(p.url, '')
                 )
               ) > 0
          )
        "#,
    )
    .bind(&tag_bind)
    .bind(&q_bind)
    .fetch_one(db)
    .await?;

    let projects = sqlx::query_as::<_, ProjectWithRank>(
        r#"
        SELECT handle, owner_wallet, source, source_ref, display_name, blurb,
               stable_id, url, tags, total_rp, rank, clicks
        FROM (
            SELECT p.*,
                   row_number() OVER (ORDER BY p.total_rp DESC, p.created_at ASC, p.handle ASC) AS rank
            FROM projects p
            WHERE ($1::text IS NULL OR $1 = ANY(p.tags))
              AND (
                $2::text IS NULL
                OR position(
                     lower($2) in lower(
                       coalesce(p.display_name, '') || ' ' || p.handle || ' '
                       || coalesce(p.blurb, '') || ' ' || coalesce(p.url, '')
                     )
                   ) > 0
              )
        ) ranked
        ORDER BY rank, handle
        LIMIT $3 OFFSET $4
        "#,
    )
    .bind(&tag_bind)
    .bind(&q_bind)
    .bind(per_page)
    .bind(offset)
    .fetch_all(db)
    .await?;

    let tags: Vec<String> = sqlx::query_scalar(
        r#"
        SELECT t
        FROM (
            SELECT DISTINCT unnest(tags) AS t
            FROM projects
        ) s
        WHERE t IS NOT NULL AND t <> ''
        ORDER BY t
        "#,
    )
    .fetch_all(db)
    .await?;

    Ok(ProjectListPage {
        projects,
        total,
        page,
        per_page,
        tags,
    })
}

pub async fn get_project(db: &PgPool, handle: &str) -> Result<Option<Project>, sqlx::Error> {
    sqlx::query_as::<_, Project>(
        r#"
        SELECT handle, owner_wallet, source, source_ref, display_name, blurb,
               stable_id, url, tags, total_rp, clicks
        FROM projects WHERE handle = $1
        "#,
    )
    .bind(handle)
    .fetch_optional(db)
    .await
}

/// First-party outbound click. Returns the new count, or None if unknown handle.
pub async fn record_click(db: &PgPool, handle: &str) -> Result<Option<i64>, sqlx::Error> {
    let row: Option<(i64,)> = sqlx::query_as(
        "UPDATE projects SET clicks = clicks + 1 WHERE handle = $1 RETURNING clicks",
    )
    .bind(handle)
    .fetch_optional(db)
    .await?;
    Ok(row.map(|r| r.0))
}

#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize)]
pub struct AllocationRow {
    pub id: Uuid,
    pub project_handle: String,
    pub supporter_wallet: String,
    pub amount: i64,
    pub bucket: String,
    pub source: String,
    pub lot_id: Option<Uuid>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, serde::Serialize)]
pub struct SupportOutcome {
    pub allocation: AllocationRow,
    /// Split actually applied (free drains FIFO lots first).
    pub from_free: i64,
    pub from_paid: i64,
    pub project_total_rp: i64,
}

/// Allocate wallet RP to a project ("fueling" it in the race).
///
/// Atomic across three ledgers: the supporter's wallet cache, their FIFO lots
/// (free drained earliest-expiry-first via ledger::spend_inner), and the
/// project's immutable allocation ledger with cached total_rp rollup.
/// Refuses unknown projects and insufficient balances without partial writes.
pub async fn allocate_to_project(
    db: &PgPool,
    wallet: &str,
    project_handle: &str,
    amount: i64,
    reason: Option<&str>,
) -> Result<Option<SupportOutcome>, sqlx::Error> {
    use sqlx::Postgres;

    if amount <= 0 {
        return Err(sqlx::Error::Configuration(
            "allocation amount must be positive".into(),
        ));
    }
    if !crate::ledger::valid_wallet(wallet) {
        return Err(WalletError::Invalid.into());
    }

    let mut tx: sqlx::Transaction<'static, Postgres> = db.begin().await?;

    // Lock the project row first (consistent lock order: project → wallet).
    let project_total_before: Option<i64> =
        sqlx::query_scalar("SELECT total_rp FROM projects WHERE handle = $1 FOR UPDATE")
            .bind(project_handle)
            .fetch_optional(&mut *tx)
            .await?;
    let Some(_before) = project_total_before else {
        tx.rollback().await?;
        return Ok(None); // unknown project
    };

    // Drain the supporter's balance FIFO (free lots first, then paid).
    let spend_reason = reason
        .filter(|r| !r.is_empty())
        .map(|r| format!("support:{project_handle}:{r}"))
        .unwrap_or_else(|| format!("support:{project_handle}"));
    let Some(breakdown) =
        crate::ledger::spend_inner(&mut tx, wallet, amount, &spend_reason).await?
    else {
        tx.rollback().await?;
        return Ok(None); // insufficient funds / unknown wallet
    };

    // Determine typed source + provenance for the allocation row.
    // A single-lot drain keeps its exact lot; mixed drains keep the dominant
    // lot but always list every lot in the breakdown response.
    let (source, bucket, lot_id): (&'static str, &'static str, Option<Uuid>) =
        if !breakdown.lots.is_empty() {
            let dominant = breakdown.lots.iter().max_by_key(|l| l.amount).unwrap();
            let src = match RpSource::parse(&dominant.source) {
                Some(RpSource::FreeWeekly) => "free_weekly",
                Some(RpSource::Bonus) | None => "bonus",
                Some(RpSource::EventMultiplier) => "event_multiplier",
                Some(RpSource::Paid) => "paid",
            };
            (src, "free", Some(dominant.lot_id))
        } else if breakdown.from_paid > 0 {
            ("paid", "paid", None)
        } else {
            return Err(sqlx::Error::Configuration(
                "spend produced neither free nor paid legs".into(),
            ));
        };

    let allocation: AllocationRow = sqlx::query_as::<_, AllocationRow>(
        r#"
        INSERT INTO project_allocations
            (project_handle, supporter_wallet, amount, bucket, source, lot_id)
        VALUES ($1, $2, $3, $4, $5, $6)
        RETURNING id, project_handle, supporter_wallet, amount, bucket, source, lot_id, created_at
        "#,
    )
    .bind(project_handle)
    .bind(wallet)
    .bind(amount)
    .bind(bucket)
    .bind(source)
    .bind(lot_id)
    .fetch_one(&mut *tx)
    .await?;

    let new_total: i64 = sqlx::query_scalar(
        "UPDATE projects SET total_rp = total_rp + $1 WHERE handle = $2 RETURNING total_rp",
    )
    .bind(amount)
    .bind(project_handle)
    .fetch_one(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(Some(SupportOutcome {
        allocation,
        from_free: breakdown.from_free,
        from_paid: breakdown.from_paid,
        project_total_rp: new_total,
    }))
}

/// Recent allocation history for a project (newest first).
pub async fn allocations_for(
    db: &PgPool,
    project_handle: &str,
    limit: i64,
) -> Result<Vec<AllocationRow>, sqlx::Error> {
    sqlx::query_as::<_, AllocationRow>(
        r#"
        SELECT id, project_handle, supporter_wallet, amount, bucket, source, lot_id, created_at
        FROM project_allocations
        WHERE project_handle = $1
        ORDER BY created_at DESC LIMIT $2
        "#,
    )
    .bind(project_handle)
    .bind(limit.clamp(1, 200))
    .fetch_all(db)
    .await
}

/// Convenience: count of imported projects by source (for seed verification).
pub async fn counts_by_source(db: &PgPool) -> Result<Vec<(String, i64)>, sqlx::Error> {
    sqlx::query_as("SELECT source, COUNT(*) FROM projects GROUP BY source ORDER BY source")
        .fetch_all(db)
        .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derived_handle_sanitizes_and_caps() {
        let p = ImportProject {
            stable_id: "outbid:Beanz Coffee!".into(),
            handle: None,
            display_name: None,
            url: None,
            blurb: None,
            tags: vec![],
            source: Some("outbid_import".into()),
            source_ref: None,
            owner_wallet: None,
        };
        assert_eq!(p.derived_handle(), "beanz-coffee-");

        let long = ImportProject {
            stable_id: format!("board:{}", "x".repeat(80)),
            handle: None,
            display_name: None,
            url: None,
            blurb: None,
            tags: vec![],
            source: None,
            source_ref: None,
            owner_wallet: None,
        };
        assert_eq!(long.derived_handle().len(), 32);
    }

    #[test]
    fn short_stable_ids_get_padded_handles() {
        let p = ImportProject {
            stable_id: "outbid:x".into(),
            handle: None,
            display_name: None,
            url: None,
            blurb: None,
            tags: vec![],
            source: None,
            source_ref: None,
            owner_wallet: None,
        };
        assert_eq!(p.derived_handle(), "x00");
    }

    #[test]
    fn tags_are_cleaned_sorted_deduped_capped() {
        let tags = clean_tags(&[
            " AI ".to_string(),
            "ai".to_string(),
            "Dev Tools!!".to_string(),
            "".to_string(),
        ]);
        assert_eq!(tags, vec!["ai", "dev-tools"]);
        assert!(clean_tags(&[]).is_empty());
    }

    #[test]
    fn source_vocab_maps_safely() {
        assert_eq!(
            ImportProject {
                stable_id: "s".into(),
                handle: None,
                display_name: None,
                url: None,
                blurb: None,
                tags: vec![],
                source: Some("weird".into()),
                source_ref: None,
                owner_wallet: None,
            }
            .normalized_source(),
            "outbid_import"
        );
        assert_eq!(RpSource::parse("paid"), Some(RpSource::Paid));
    }

    #[test]
    fn parse_import_payload_reads_json_array() {
        let payload = r#"[
            {"stable_id": "outbid:a", "display_name": "A", "tags": ["ai"]},
            {"stable_id": "outbid:b", "url": "https://b.example"}
        ]"#;
        let items = parse_import_payload(payload).expect("valid payload");
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].stable_id, "outbid:a");
        assert!(parse_import_payload("{\"nope\":1}").is_err());
    }
}
