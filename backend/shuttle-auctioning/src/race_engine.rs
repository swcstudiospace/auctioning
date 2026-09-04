//! Race-engine core: ranks, velocity, momentum, overtakes, photo finishes.
//!
//! All derived from the append-only `project_allocations` ledger. Nothing
//! here writes free RP on-chain or implies cash-out. Persistence is optional
//! (snapshots + narrative events) so the news layer can consume later.

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

/// One allocation sample fed into the pure scorer.
#[derive(Debug, Clone)]
pub struct AllocationSample {
    pub project_handle: String,
    pub amount: i64,
    pub created_at: DateTime<Utc>,
    /// paid | free_weekly | bonus | event_multiplier
    pub source: String,
}

/// Previous snapshot slot used to detect overtakes / lead changes.
#[derive(Debug, Clone)]
pub struct PreviousSlot {
    pub project_handle: String,
    pub rank: i32,
    pub race_rp: i64,
}

#[derive(Debug, Clone)]
pub struct EngineConfig {
    pub now: DateTime<Utc>,
    pub window_start: DateTime<Utc>,
    pub window_end: DateTime<Utc>,
    /// Lookback for velocity (seconds). Default 3600.
    pub velocity_secs: i64,
    /// Gap at which a pair is a photo finish. Default 5.
    pub photo_finish_gap: i64,
    /// Minimum rank climb (places gained) to count as a dark-horse rise.
    pub dark_horse_min_climb: i32,
}

impl EngineConfig {
    pub fn default_for_window(
        now: DateTime<Utc>,
        window_start: DateTime<Utc>,
        window_end: DateTime<Utc>,
    ) -> Self {
        Self {
            now,
            window_start,
            window_end,
            velocity_secs: 3_600,
            photo_finish_gap: 5,
            dark_horse_min_climb: 5,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GridSlot {
    pub handle: String,
    pub rank: i32,
    pub race_rp: i64,
    pub velocity: i64,
    pub momentum: i64,
    pub gap_to_leader: i64,
    pub gap_to_next: Option<i64>,
    /// Percent pace vs the previous equal velocity window. None if prior was 0.
    pub pace_pct: Option<i64>,
    /// Lifetime board rank, if known. Differs from `rank` ⇒ "not overall".
    pub lifetime_rank: Option<i32>,
    /// RP allocated in the last 900s (inside the race window).
    pub burst_rp: i64,
    /// Consecutive velocity windows at or above median velocity (0..=2).
    pub sustain_windows: i32,
    /// Sum of allocations with source == "paid".
    pub paid_rp: i64,
    /// race_rp - paid_rp (community / promo fuel).
    pub community_rp: i64,
    /// Exactly one of HOT|REIGN|DARK_HORSE|PHOTO|COOLING, else None.
    pub badge: Option<String>,
    /// Handle this slot most recently passed, if an overtake fired.
    pub last_overtake: Option<String>,
    /// Board clicks; 0 until a click table exists.
    pub clicks: i64,
    pub hover_footer: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RaceEventKind {
    Overtake,
    PhotoFinish,
    LeadChange,
    DarkHorseRise,
    RaceStart,
    RaceFinish,
    SignificantSpend,
    /// MagicBlock ER tick that did not map to a named overtake/lead change.
    ErTick,
}

impl RaceEventKind {
    pub fn as_str(self) -> &'static str {
        match self {
            RaceEventKind::Overtake => "overtake",
            RaceEventKind::PhotoFinish => "photo_finish",
            RaceEventKind::LeadChange => "lead_change",
            RaceEventKind::DarkHorseRise => "dark_horse_rise",
            RaceEventKind::RaceStart => "race_start",
            RaceEventKind::RaceFinish => "race_finish",
            RaceEventKind::SignificantSpend => "significant_spend",
            RaceEventKind::ErTick => "er_tick",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "overtake" => Some(Self::Overtake),
            "photo_finish" => Some(Self::PhotoFinish),
            "lead_change" => Some(Self::LeadChange),
            "dark_horse_rise" => Some(Self::DarkHorseRise),
            "race_start" => Some(Self::RaceStart),
            "race_finish" => Some(Self::RaceFinish),
            "significant_spend" => Some(Self::SignificantSpend),
            "er_tick" => Some(Self::ErTick),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DerivedEvent {
    pub kind: RaceEventKind,
    pub project_handle: String,
    pub other_handle: Option<String>,
    pub title: String,
    pub summary: String,
    pub from_rank: Option<i32>,
    pub to_rank: Option<i32>,
    pub rp_delta: Option<i64>,
}

/// Compute the live grid and narrative events from allocations + last snapshot.
///
/// Ranking: `race_rp` desc, then earliest last-allocation timestamp (first to
/// the mark). Velocity = RP in the last `velocity_secs`. Momentum = current
/// velocity minus the previous equal-length window.
pub fn compute_grid(
    allocations: &[AllocationSample],
    previous: &[PreviousSlot],
    cfg: &EngineConfig,
) -> (Vec<GridSlot>, Vec<DerivedEvent>) {
    use std::collections::HashMap;

    let mut totals: HashMap<&str, i64> = HashMap::new();
    let mut last_at: HashMap<&str, DateTime<Utc>> = HashMap::new();
    let mut velocity: HashMap<&str, i64> = HashMap::new();
    let mut prev_velocity: HashMap<&str, i64> = HashMap::new();
    let mut burst: HashMap<&str, i64> = HashMap::new();
    let mut paid: HashMap<&str, i64> = HashMap::new();

    let vel_start = cfg.now - chrono::Duration::seconds(cfg.velocity_secs);
    let prev_vel_start = vel_start - chrono::Duration::seconds(cfg.velocity_secs);
    let burst_start = cfg.now - chrono::Duration::seconds(900);

    for a in allocations {
        if a.created_at < cfg.window_start || a.created_at > cfg.window_end {
            continue;
        }
        if a.amount <= 0 {
            continue;
        }
        let handle = a.project_handle.as_str();
        *totals.entry(handle).or_insert(0) += a.amount;
        last_at
            .entry(handle)
            .and_modify(|t| {
                if a.created_at > *t {
                    *t = a.created_at;
                }
            })
            .or_insert(a.created_at);
        if a.source == "paid" {
            *paid.entry(handle).or_insert(0) += a.amount;
        }
        if a.created_at >= burst_start && a.created_at <= cfg.now {
            *burst.entry(handle).or_insert(0) += a.amount;
        }
        if a.created_at >= vel_start && a.created_at <= cfg.now {
            *velocity.entry(handle).or_insert(0) += a.amount;
        } else if a.created_at >= prev_vel_start && a.created_at < vel_start {
            *prev_velocity.entry(handle).or_insert(0) += a.amount;
        }
    }

    let mut handles: Vec<&str> = totals.keys().copied().collect();
    handles.sort_by(|a, b| {
        let ra = totals.get(a).copied().unwrap_or(0);
        let rb = totals.get(b).copied().unwrap_or(0);
        rb.cmp(&ra).then_with(|| {
            let ta = last_at.get(a).copied().unwrap_or(cfg.now);
            let tb = last_at.get(b).copied().unwrap_or(cfg.now);
            ta.cmp(&tb).then_with(|| a.cmp(b))
        })
    });

    // P1 paid floor: a zero-paid leader yields to the best-ranked paid slot.
    if let Some(leader) = handles.first().copied() {
        if paid.get(leader).copied().unwrap_or(0) == 0 {
            if let Some(idx) = handles
                .iter()
                .position(|h| paid.get(h).copied().unwrap_or(0) > 0)
            {
                handles.swap(0, idx);
            }
        }
    }

    let mut vel_values: Vec<i64> = handles
        .iter()
        .map(|h| velocity.get(h).copied().unwrap_or(0))
        .collect();
    vel_values.sort_unstable();
    let median_vel = if vel_values.is_empty() {
        0
    } else if vel_values.len() % 2 == 1 {
        vel_values[vel_values.len() / 2]
    } else {
        let mid = vel_values.len() / 2;
        (vel_values[mid - 1] + vel_values[mid]) / 2
    };

    let leader_rp = handles
        .first()
        .and_then(|h| totals.get(h).copied())
        .unwrap_or(0);

    let mut grid: Vec<GridSlot> = Vec::with_capacity(handles.len());
    for (i, handle) in handles.iter().enumerate() {
        let race_rp = totals.get(handle).copied().unwrap_or(0);
        let vel = velocity.get(handle).copied().unwrap_or(0);
        let pvel = prev_velocity.get(handle).copied().unwrap_or(0);
        let next_rp = handles.get(i + 1).and_then(|n| totals.get(n).copied());
        let paid_rp = paid.get(handle).copied().unwrap_or(0);
        let sustain_windows = i32::from(vel >= median_vel) + i32::from(pvel >= median_vel);
        grid.push(GridSlot {
            handle: (*handle).to_string(),
            rank: (i as i32) + 1,
            race_rp,
            velocity: vel,
            momentum: vel - pvel,
            gap_to_leader: leader_rp - race_rp,
            gap_to_next: next_rp.map(|n| race_rp - n),
            pace_pct: pace_pct_change(vel, pvel),
            lifetime_rank: None,
            burst_rp: burst.get(handle).copied().unwrap_or(0),
            sustain_windows,
            paid_rp,
            community_rp: race_rp - paid_rp,
            badge: None,
            last_overtake: None,
            clicks: 0,
            hover_footer: String::new(),
        });
    }

    let prev_by: HashMap<&str, &PreviousSlot> = previous
        .iter()
        .map(|p| (p.project_handle.as_str(), p))
        .collect();

    let mut events: Vec<DerivedEvent> = Vec::new();

    if previous.is_empty() && !grid.is_empty() {
        events.push(DerivedEvent {
            kind: RaceEventKind::RaceStart,
            project_handle: grid[0].handle.clone(),
            other_handle: None,
            title: "Lights out".into(),
            summary: format!("{} cars on the grid as the window opens.", grid.len()),
            from_rank: None,
            to_rank: Some(1),
            rp_delta: None,
        });
    }

    // Overtakes + dark horse + lead change vs previous snapshot.
    for slot in &grid {
        if let Some(prev) = prev_by.get(slot.handle.as_str()) {
            if slot.rank < prev.rank {
                // Who did they pass? Anyone who used to be ahead and is now behind.
                let passed = previous.iter().find(|p| {
                    p.rank < prev.rank && p.rank >= slot.rank && p.project_handle != slot.handle
                });
                let other = passed.map(|p| p.project_handle.clone());
                events.push(DerivedEvent {
                    kind: RaceEventKind::Overtake,
                    project_handle: slot.handle.clone(),
                    other_handle: other.clone(),
                    title: format!("{} climbs to P{}", slot.handle, slot.rank),
                    summary: match &other {
                        Some(o) => format!(
                            "{} overtook {} (P{} → P{})",
                            slot.handle, o, prev.rank, slot.rank
                        ),
                        None => format!("{} moved P{} → P{}", slot.handle, prev.rank, slot.rank),
                    },
                    from_rank: Some(prev.rank),
                    to_rank: Some(slot.rank),
                    rp_delta: Some(slot.race_rp - prev.race_rp),
                });

                if prev.rank - slot.rank >= cfg.dark_horse_min_climb && slot.rank <= 8 {
                    events.push(DerivedEvent {
                        kind: RaceEventKind::DarkHorseRise,
                        project_handle: slot.handle.clone(),
                        other_handle: None,
                        title: format!("Dark horse: {}", slot.handle),
                        summary: format!(
                            "{} jumped {} places into P{}",
                            slot.handle,
                            prev.rank - slot.rank,
                            slot.rank
                        ),
                        from_rank: Some(prev.rank),
                        to_rank: Some(slot.rank),
                        rp_delta: Some(slot.race_rp - prev.race_rp),
                    });
                }
            }
        }
    }

    let prev_leader = previous.iter().find(|p| p.rank == 1);
    let new_leader = grid.first();
    if let (Some(old), Some(now_lead)) = (prev_leader, new_leader) {
        if old.project_handle != now_lead.handle {
            events.push(DerivedEvent {
                kind: RaceEventKind::LeadChange,
                project_handle: now_lead.handle.clone(),
                other_handle: Some(old.project_handle.clone()),
                title: format!("{} takes P1", now_lead.handle),
                summary: format!(
                    "{} unseats {} at the front",
                    now_lead.handle, old.project_handle
                ),
                from_rank: Some(1),
                to_rank: Some(1),
                rp_delta: None,
            });
        }
    }

    // Photo finishes: adjacent pair with a tiny gap and both on the board.
    for pair in grid.windows(2) {
        let a = &pair[0];
        let b = &pair[1];
        if let Some(gap) = a.gap_to_next {
            if gap <= cfg.photo_finish_gap && a.race_rp > 0 && b.race_rp > 0 {
                events.push(DerivedEvent {
                    kind: RaceEventKind::PhotoFinish,
                    project_handle: a.handle.clone(),
                    other_handle: Some(b.handle.clone()),
                    title: format!("Photo finish: {} vs {}", a.handle, b.handle),
                    summary: format!(
                        "P{} {} and P{} {} split by {} RP",
                        a.rank, a.handle, b.rank, b.handle, gap
                    ),
                    from_rank: Some(a.rank),
                    to_rank: Some(b.rank),
                    rp_delta: Some(gap),
                });
            }
        }
    }

    let n = grid.len();
    let max_vel = grid.iter().map(|s| s.velocity).max().unwrap_or(0);
    let max_vel_count = grid.iter().filter(|s| s.velocity == max_vel).count();
    let mut vels_desc: Vec<i64> = grid.iter().map(|s| s.velocity).collect();
    vels_desc.sort_unstable_by(|a, b| b.cmp(a));
    let hot_threshold = if n >= 10 {
        Some(vels_desc[n / 10 - 1])
    } else {
        None
    };

    let mut last_pass: HashMap<String, String> = HashMap::new();
    for ev in &events {
        if ev.kind == RaceEventKind::Overtake {
            if let Some(other) = &ev.other_handle {
                last_pass.insert(ev.project_handle.clone(), other.clone());
            }
        }
    }

    for slot in &mut grid {
        slot.last_overtake = last_pass.get(&slot.handle).cloned();

        let climbed = prev_by
            .get(slot.handle.as_str())
            .map(|p| p.rank - slot.rank)
            .unwrap_or(0);
        let photo = slot
            .gap_to_next
            .map(|g| g <= cfg.photo_finish_gap)
            .unwrap_or(false);
        let hot = if n >= 10 {
            slot.velocity > 0 && hot_threshold.map(|t| slot.velocity >= t).unwrap_or(false)
        } else {
            slot.velocity > 0 && slot.velocity == max_vel && max_vel_count == 1
        };
        let reign = slot.rank == 1
            && prev_by
                .get(slot.handle.as_str())
                .map(|p| p.rank == 1)
                .unwrap_or(false);
        let cooling = matches!(slot.pace_pct, Some(x) if x < 0) && slot.momentum < 0;

        slot.badge = if photo {
            Some("PHOTO".into())
        } else if climbed >= cfg.dark_horse_min_climb {
            Some("DARK_HORSE".into())
        } else if hot {
            Some("HOT".into())
        } else if reign {
            Some("REIGN".into())
        } else if cooling {
            Some("COOLING".into())
        } else {
            None
        };

        let took = events.iter().any(|e| {
            e.project_handle == slot.handle
                && (e.kind == RaceEventKind::Overtake || e.kind == RaceEventKind::LeadChange)
                && e.to_rank == Some(slot.rank)
        });
        slot.hover_footer = if took && slot.burst_rp > 0 {
            format!("Took P{} after {} burst.", slot.rank, slot.burst_rp)
        } else if slot.rank == 1 {
            format!("Held P1. Gap {}.", slot.gap_to_next.unwrap_or(0))
        } else if climbed >= cfg.dark_horse_min_climb {
            format!("Dark horse: +{} in window on {}.", climbed, slot.race_rp)
        } else if slot.community_rp > slot.paid_rp && slot.gap_to_next.is_some() {
            format!(
                "Community RP kept the gap at {}.",
                slot.gap_to_next.unwrap()
            )
        } else if photo {
            format!(
                "Photo finish: {} RP from P{}.",
                slot.gap_to_next.unwrap_or(0),
                slot.rank + 1
            )
        } else if cooling {
            format!(
                "Cooling: pace down {}% over window.",
                slot.pace_pct.unwrap_or(0).abs()
            )
        } else {
            format!("{} P{} · {} race RP", slot.handle, slot.rank, slot.race_rp)
        };
    }

    (grid, events)
}

/// Percent change in pace. Never invents a percentage from a zero prior window.
pub fn pace_pct_change(current: i64, previous: i64) -> Option<i64> {
    if previous <= 0 {
        None
    } else {
        Some(((current - previous) * 100) / previous)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrendContrast {
    pub window_rank: i32,
    pub lifetime_rank: Option<i32>,
    pub pace_pct: Option<i64>,
    pub not_overall: bool,
}

pub fn trend_contrast(
    window_rank: i32,
    lifetime_rank: Option<i32>,
    velocity: i64,
    prev_velocity: i64,
) -> TrendContrast {
    TrendContrast {
        window_rank,
        lifetime_rank,
        pace_pct: pace_pct_change(velocity, prev_velocity),
        not_overall: lifetime_rank.map(|l| l != window_rank).unwrap_or(false),
    }
}

pub fn attach_lifetime_ranks(window_grid: &mut [GridSlot], lifetime: &[GridSlot]) {
    use std::collections::HashMap;
    let map: HashMap<&str, i32> = lifetime
        .iter()
        .map(|s| (s.handle.as_str(), s.rank))
        .collect();
    for slot in window_grid {
        slot.lifetime_rank = map.get(slot.handle.as_str()).copied();
    }
}

// ---------------------------------------------------------------------------
// Persistence (optional — GET endpoints can compute without writing)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, sqlx::FromRow, Serialize)]
pub struct RaceWindowRow {
    pub id: Uuid,
    pub slug: String,
    pub name: String,
    pub race_type: String,
    pub status: String,
    pub tag: Option<String>,
    pub starts_at: DateTime<Utc>,
    pub ends_at: DateTime<Utc>,
}

#[derive(Debug, Clone, sqlx::FromRow, Serialize)]
pub struct RaceEventRow {
    pub id: Uuid,
    pub race_window_id: Uuid,
    pub project_handle: Option<String>,
    pub other_handle: Option<String>,
    pub event_type: String,
    pub title: String,
    pub summary: Option<String>,
    pub is_narrative_worthy: bool,
    pub created_at: DateTime<Utc>,
    pub payload: serde_json::Value,
}

/// Close windows whose clock has run out so the calendar is not a live lie.
/// Scoring sessions get a final snapshot first so championship can score them.
pub async fn archive_expired_windows(db: &PgPool, now: DateTime<Utc>) -> Result<u64, sqlx::Error> {
    let due: Vec<RaceWindowRow> = sqlx::query_as(
        r#"
        SELECT id, slug, name, race_type, status, tag, starts_at, ends_at
        FROM race_windows
        WHERE status IN ('live', 'scheduled', 'qualifying', 'final_lap')
          AND ends_at <= $1
        "#,
    )
    .bind(now)
    .fetch_all(db)
    .await?;

    for w in &due {
        if is_scoring_race_type(&w.race_type) {
            if let Err(e) = persist_snapshot(db, w).await {
                tracing::warn!(error = %e, slug = %w.slug, "final snapshot before archive failed");
            }
        }
    }

    let res = sqlx::query(
        r#"
        UPDATE race_windows
        SET status = 'archived'
        WHERE status IN ('live', 'scheduled', 'qualifying', 'final_lap')
          AND ends_at <= $1
        "#,
    )
    .bind(now)
    .execute(db)
    .await?;
    Ok(res.rows_affected())
}

pub fn is_scoring_race_type(t: &str) -> bool {
    matches!(
        t.to_ascii_uppercase().as_str(),
        "GRAND_TOUR" | "GRAND_PRIX" | "GREEN_FLAG" | "SPRINT" | "PACE_LAP"
    )
}

/// Archived scoring windows with no snapshot still need a finishing board.
pub async fn backfill_archived_finals(db: &PgPool) -> Result<u64, sqlx::Error> {
    let rows: Vec<RaceWindowRow> = sqlx::query_as(
        r#"
        SELECT id, slug, name, race_type, status, tag, starts_at, ends_at
        FROM race_windows w
        WHERE w.status IN ('archived', 'finished')
          AND w.race_type IN ('GRAND_TOUR','GRAND_PRIX','GREEN_FLAG','SPRINT','PACE_LAP')
          AND NOT EXISTS (
            SELECT 1 FROM rank_snapshots s WHERE s.race_window_id = w.id
          )
        "#,
    )
    .fetch_all(db)
    .await?;
    let mut n = 0u64;
    for w in rows {
        persist_snapshot(db, &w).await?;
        n += 1;
    }
    Ok(n)
}

/// Saturday 16:00–17:00 UTC this week, or next Saturday if that hour already ended.
pub fn saturday_sprint_bounds(now: DateTime<Utc>) -> (DateTime<Utc>, DateTime<Utc>) {
    let monday = crate::ledger::week_start_of(now);
    let this_start = monday + Duration::days(5) + Duration::hours(16);
    let this_end = this_start + Duration::hours(1);
    if now < this_end {
        (this_start, this_end)
    } else {
        let next = this_start + Duration::weeks(1);
        (next, next + Duration::hours(1))
    }
}

/// Ensure a Green Flag sprint exists on the calendar (scheduled or live).
pub async fn ensure_default_sprint(db: &PgPool) -> Result<RaceWindowRow, sqlx::Error> {
    let now = Utc::now();
    let (start, end) = saturday_sprint_bounds(now);
    let slug = format!("green-flag-{}", start.format("%Y-%m-%d"));
    let status = if now < start { "scheduled" } else { "live" };

    sqlx::query(
        r#"
        INSERT INTO race_windows (slug, name, race_type, status, starts_at, ends_at, rules)
        VALUES ($1, $2, 'GREEN_FLAG', $3, $4, $5, '{"photo_finish_gap":5,"velocity_secs":3600}'::jsonb)
        ON CONFLICT (slug) DO NOTHING
        "#,
    )
    .bind(&slug)
    .bind(format!("Green Flag {}", start.format("%Y-%m-%d")))
    .bind(status)
    .bind(start)
    .bind(end)
    .execute(db)
    .await?;

    sqlx::query_as::<_, RaceWindowRow>(
        r#"
        SELECT id, slug, name, race_type, status, tag, starts_at, ends_at
        FROM race_windows WHERE slug = $1
        "#,
    )
    .bind(&slug)
    .fetch_one(db)
    .await
}

/// Ensure a live weekly Grand Tour exists so the grid is never empty-state.
/// Also archives expired windows and seeds this week's Saturday Green Flag.
pub async fn ensure_default_window(db: &PgPool) -> Result<RaceWindowRow, sqlx::Error> {
    let _ = archive_expired_windows(db, Utc::now()).await?;
    if let Err(e) = backfill_archived_finals(db).await {
        tracing::warn!(error = %e, "backfill_archived_finals failed");
    }
    let week_start = crate::ledger::current_week_start();
    let week_end = crate::ledger::next_week_start(Utc::now());
    let slug = format!("grand-tour-{}", week_start.format("%Y-%m-%d"));

    sqlx::query(
        r#"
        INSERT INTO race_windows (slug, name, race_type, status, starts_at, ends_at, rules)
        VALUES ($1, $2, 'GRAND_TOUR', 'live', $3, $4, '{"photo_finish_gap":5,"velocity_secs":3600}'::jsonb)
        ON CONFLICT (slug) DO NOTHING
        "#,
    )
    .bind(&slug)
    .bind(format!("Grand Tour {}", week_start.format("%Y-%m-%d")))
    .bind(week_start)
    .bind(week_end)
    .execute(db)
    .await?;

    if let Err(e) = ensure_default_sprint(db).await {
        tracing::warn!(error = %e, "ensure_default_sprint failed");
    }

    sqlx::query_as::<_, RaceWindowRow>(
        r#"
        SELECT id, slug, name, race_type, status, tag, starts_at, ends_at
        FROM race_windows WHERE slug = $1
        "#,
    )
    .bind(&slug)
    .fetch_one(db)
    .await
}

pub async fn list_windows(db: &PgPool, limit: i64) -> Result<Vec<RaceWindowRow>, sqlx::Error> {
    sqlx::query_as::<_, RaceWindowRow>(
        r#"
        SELECT id, slug, name, race_type, status, tag, starts_at, ends_at
        FROM race_windows
        ORDER BY starts_at DESC
        LIMIT $1
        "#,
    )
    .bind(limit.clamp(1, 100))
    .fetch_all(db)
    .await
}

pub async fn window_by_slug(db: &PgPool, slug: &str) -> Result<Option<RaceWindowRow>, sqlx::Error> {
    sqlx::query_as::<_, RaceWindowRow>(
        r#"
        SELECT id, slug, name, race_type, status, tag, starts_at, ends_at
        FROM race_windows WHERE slug = $1
        "#,
    )
    .bind(slug)
    .fetch_optional(db)
    .await
}

pub async fn window_by_id(db: &PgPool, id: Uuid) -> Result<Option<RaceWindowRow>, sqlx::Error> {
    sqlx::query_as::<_, RaceWindowRow>(
        r#"
        SELECT id, slug, name, race_type, status, tag, starts_at, ends_at
        FROM race_windows WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(db)
    .await
}

async fn load_allocations(
    db: &PgPool,
    tag: Option<&str>,
    window_start: DateTime<Utc>,
    window_end: DateTime<Utc>,
) -> Result<Vec<AllocationSample>, sqlx::Error> {
    let rows: Vec<(String, i64, DateTime<Utc>, String)> = if let Some(tag) = tag {
        sqlx::query_as(
            r#"
            SELECT a.project_handle, a.amount, a.created_at, a.source
            FROM project_allocations a
            JOIN projects p ON p.handle = a.project_handle
            WHERE a.created_at >= $1 AND a.created_at <= $2
              AND $3 = ANY (p.tags)
            "#,
        )
        .bind(window_start)
        .bind(window_end)
        .bind(tag)
        .fetch_all(db)
        .await?
    } else {
        sqlx::query_as(
            r#"
            SELECT project_handle, amount, created_at, source
            FROM project_allocations
            WHERE created_at >= $1 AND created_at <= $2
            "#,
        )
        .bind(window_start)
        .bind(window_end)
        .fetch_all(db)
        .await?
    };
    Ok(rows
        .into_iter()
        .map(
            |(project_handle, amount, created_at, source)| AllocationSample {
                project_handle,
                amount,
                created_at,
                source,
            },
        )
        .collect())
}

async fn load_previous_snapshot(
    db: &PgPool,
    window_id: Uuid,
) -> Result<Vec<PreviousSlot>, sqlx::Error> {
    let latest: Option<DateTime<Utc>> =
        sqlx::query_scalar("SELECT MAX(snapshot_at) FROM rank_snapshots WHERE race_window_id = $1")
            .bind(window_id)
            .fetch_one(db)
            .await?;
    let Some(at) = latest else {
        return Ok(Vec::new());
    };
    let rows: Vec<(String, i32, i64)> = sqlx::query_as(
        r#"
        SELECT project_handle, rank, race_rp
        FROM rank_snapshots
        WHERE race_window_id = $1 AND snapshot_at = $2
        "#,
    )
    .bind(window_id)
    .bind(at)
    .fetch_all(db)
    .await?;
    Ok(rows
        .into_iter()
        .map(|(project_handle, rank, race_rp)| PreviousSlot {
            project_handle,
            rank,
            race_rp,
        })
        .collect())
}

fn config_from_window(w: &RaceWindowRow, now: DateTime<Utc>) -> EngineConfig {
    EngineConfig {
        now,
        window_start: w.starts_at,
        window_end: w.ends_at.min(now),
        velocity_secs: 3_600,
        photo_finish_gap: 5,
        dark_horse_min_climb: 5,
    }
}

/// Compute the live grid for a window without writing a snapshot.
pub async fn grid_for_window(
    db: &PgPool,
    window: &RaceWindowRow,
) -> Result<(Vec<GridSlot>, Vec<DerivedEvent>), sqlx::Error> {
    let now = Utc::now();
    let cfg = config_from_window(window, now);
    let allocs =
        load_allocations(db, window.tag.as_deref(), cfg.window_start, cfg.window_end).await?;
    let prev = load_previous_snapshot(db, window.id).await?;
    let (mut grid, events) = compute_grid(&allocs, &prev, &cfg);
    let lifetime = lifetime_grid(db).await?;
    attach_lifetime_ranks(&mut grid, &lifetime);
    Ok((grid, events))
}

/// Lifetime board treated as an open-ended race (velocity still last hour).
pub async fn lifetime_grid(db: &PgPool) -> Result<Vec<GridSlot>, sqlx::Error> {
    let now = Utc::now();
    let start = DateTime::<Utc>::from_timestamp(0, 0).unwrap_or(now);
    let allocs = load_allocations(db, None, start, now).await?;
    let cfg = EngineConfig::default_for_window(now, start, now);
    let (grid, _) = compute_grid(&allocs, &[], &cfg);
    Ok(grid)
}

/// Persist a snapshot + newly derived events. Returns the grid written.
pub async fn persist_snapshot(
    db: &PgPool,
    window: &RaceWindowRow,
) -> Result<(Vec<GridSlot>, Vec<DerivedEvent>), sqlx::Error> {
    let now = Utc::now();
    let cfg = config_from_window(window, now);
    let allocs =
        load_allocations(db, window.tag.as_deref(), cfg.window_start, cfg.window_end).await?;
    let prev = load_previous_snapshot(db, window.id).await?;
    let (grid, events) = compute_grid(&allocs, &prev, &cfg);

    let mut tx = db.begin().await?;
    let snap_at = now;
    for slot in &grid {
        sqlx::query(
            r#"
            INSERT INTO rank_snapshots
                (race_window_id, project_handle, rank, race_rp,
                 gap_to_leader, gap_to_next, velocity, momentum, snapshot_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            "#,
        )
        .bind(window.id)
        .bind(&slot.handle)
        .bind(slot.rank)
        .bind(slot.race_rp)
        .bind(slot.gap_to_leader)
        .bind(slot.gap_to_next)
        .bind(slot.velocity)
        .bind(slot.momentum)
        .bind(snap_at)
        .execute(&mut *tx)
        .await?;
    }

    for ev in &events {
        sqlx::query(
            r#"
            INSERT INTO race_events
                (race_window_id, project_handle, other_handle, event_type,
                 title, summary, payload, is_narrative_worthy)
            VALUES ($1, $2, $3, $4, $5, $6, $7::jsonb, TRUE)
            "#,
        )
        .bind(window.id)
        .bind(&ev.project_handle)
        .bind(&ev.other_handle)
        .bind(ev.kind.as_str())
        .bind(&ev.title)
        .bind(&ev.summary)
        .bind(
            serde_json::json!({
                "from_rank": ev.from_rank,
                "to_rank": ev.to_rank,
                "rp_delta": ev.rp_delta,
            })
            .to_string(),
        )
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    Ok((grid, events))
}

/// Same as persist_snapshot, but skip the write when nothing happened.
pub async fn persist_snapshot_if_events(
    db: &PgPool,
    window: &RaceWindowRow,
) -> Result<(Vec<GridSlot>, Vec<DerivedEvent>), sqlx::Error> {
    let (_grid, events) = grid_for_window(db, window).await?;
    if events.is_empty() {
        return Ok((_grid, events));
    }
    persist_snapshot(db, window).await
}

pub async fn events_for_window(
    db: &PgPool,
    window_id: Uuid,
    limit: i64,
) -> Result<Vec<RaceEventRow>, sqlx::Error> {
    sqlx::query_as::<_, RaceEventRow>(
        r#"
        SELECT id, race_window_id, project_handle, other_handle,
               event_type, title, summary, is_narrative_worthy, created_at,
               COALESCE(payload, '{}'::jsonb) AS payload
        FROM race_events
        WHERE race_window_id = $1
        ORDER BY created_at DESC
        LIMIT $2
        "#,
    )
    .bind(window_id)
    .bind(limit.clamp(1, 200))
    .fetch_all(db)
    .await
}

pub async fn event_by_id(db: &PgPool, id: Uuid) -> Result<Option<RaceEventRow>, sqlx::Error> {
    sqlx::query_as::<_, RaceEventRow>(
        r#"
        SELECT id, race_window_id, project_handle, other_handle,
               event_type, title, summary, is_narrative_worthy, created_at,
               COALESCE(payload, '{}'::jsonb) AS payload
        FROM race_events
        WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(db)
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn ts(min: i64) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 8, 25, 12, 0, 0).unwrap() + chrono::Duration::minutes(min)
    }

    fn cfg() -> EngineConfig {
        EngineConfig::default_for_window(ts(60), ts(0), ts(120))
    }

    fn alloc(handle: &str, amount: i64, min: i64) -> AllocationSample {
        alloc_src(handle, amount, min, "paid")
    }

    fn alloc_src(handle: &str, amount: i64, min: i64, source: &str) -> AllocationSample {
        AllocationSample {
            project_handle: handle.into(),
            amount,
            created_at: ts(min),
            source: source.into(),
        }
    }

    #[test]
    fn ranks_by_race_rp_then_earliest_last_mark() {
        let allocs = vec![
            alloc("alpha", 10, 10),
            alloc("beta", 30, 20),
            alloc("gamma", 30, 15), // same RP as beta, earlier last mark → ahead
        ];
        let (grid, _) = compute_grid(&allocs, &[], &cfg());
        assert_eq!(
            grid.iter().map(|s| s.handle.as_str()).collect::<Vec<_>>(),
            vec!["gamma", "beta", "alpha"]
        );
        assert_eq!(grid[0].rank, 1);
        assert_eq!(grid[0].gap_to_leader, 0);
        assert_eq!(grid[2].gap_to_leader, 20);
    }

    #[test]
    fn velocity_and_momentum_use_sliding_windows() {
        // now = ts(60). velocity window = last 3600s = ts(0)..ts(60).
        // prev velocity = ts(-60)..ts(0). Race window must include both hours
        // so pre-hour fuel still counts toward race_rp + momentum.
        let wide = EngineConfig::default_for_window(ts(60), ts(-120), ts(120));
        let allocs = vec![
            alloc("alpha", 40, -30), // previous hour
            alloc("alpha", 10, 10),  // current hour
            alloc("beta", 5, 50),
        ];
        let (grid, _) = compute_grid(&allocs, &[], &wide);
        let a = grid.iter().find(|s| s.handle == "alpha").unwrap();
        assert_eq!(a.race_rp, 50);
        assert_eq!(a.velocity, 10);
        assert_eq!(a.momentum, 10 - 40);
        let b = grid.iter().find(|s| s.handle == "beta").unwrap();
        assert_eq!(b.velocity, 5);
        assert_eq!(b.momentum, 5);
    }

    #[test]
    fn detects_overtake_lead_change_and_dark_horse() {
        let allocs = vec![
            alloc("alpha", 10, 10),
            alloc("beta", 50, 20),
            alloc("gamma", 40, 25),
        ];
        let previous = vec![
            PreviousSlot {
                project_handle: "alpha".into(),
                rank: 1,
                race_rp: 80,
            },
            PreviousSlot {
                project_handle: "gamma".into(),
                rank: 2,
                race_rp: 20,
            },
            PreviousSlot {
                project_handle: "beta".into(),
                rank: 10,
                race_rp: 5,
            },
        ];
        let mut c = cfg();
        c.dark_horse_min_climb = 5;
        let (grid, events) = compute_grid(&allocs, &previous, &c);
        assert_eq!(grid[0].handle, "beta");
        assert!(events.iter().any(|e| e.kind == RaceEventKind::LeadChange
            && e.project_handle == "beta"
            && e.other_handle.as_deref() == Some("alpha")));
        assert!(events
            .iter()
            .any(|e| e.kind == RaceEventKind::Overtake && e.project_handle == "beta"));
        assert!(events
            .iter()
            .any(|e| e.kind == RaceEventKind::DarkHorseRise && e.project_handle == "beta"));
    }

    #[test]
    fn photo_finish_when_gap_at_or_under_threshold() {
        let allocs = vec![alloc("alpha", 100, 10), alloc("beta", 97, 12)];
        let (grid, events) = compute_grid(&allocs, &[], &cfg());
        assert_eq!(grid[0].gap_to_next, Some(3));
        assert!(events.iter().any(|e| e.kind == RaceEventKind::PhotoFinish
            && e.project_handle == "alpha"
            && e.other_handle.as_deref() == Some("beta")));
    }

    #[test]
    fn no_photo_finish_when_gap_is_wide() {
        let allocs = vec![alloc("alpha", 100, 10), alloc("beta", 10, 12)];
        let (_, events) = compute_grid(&allocs, &[], &cfg());
        assert!(!events.iter().any(|e| e.kind == RaceEventKind::PhotoFinish));
    }

    #[test]
    fn empty_board_is_quiet() {
        let (grid, events) = compute_grid(&[], &[], &cfg());
        assert!(grid.is_empty());
        assert!(events.is_empty());
    }

    #[test]
    fn first_snapshot_emits_race_start() {
        let allocs = vec![alloc("alpha", 1, 1)];
        let (_, events) = compute_grid(&allocs, &[], &cfg());
        assert_eq!(events[0].kind, RaceEventKind::RaceStart);
    }

    #[test]
    fn ignores_out_of_window_and_non_positive_amounts() {
        let allocs = vec![
            alloc("alpha", 10, -100), // before window
            alloc("alpha", 0, 10),
            alloc("alpha", -5, 11),
            alloc("beta", 4, 12),
        ];
        let (grid, _) = compute_grid(&allocs, &[], &cfg());
        assert_eq!(grid.len(), 1);
        assert_eq!(grid[0].handle, "beta");
    }

    #[test]
    fn event_kind_round_trip() {
        assert_eq!(RaceEventKind::Overtake.as_str(), "overtake");
        assert_eq!(RaceEventKind::PhotoFinish.as_str(), "photo_finish");
        assert_eq!(
            RaceEventKind::parse("lead_change"),
            Some(RaceEventKind::LeadChange)
        );
        assert_eq!(RaceEventKind::parse("nope"), None);
    }

    #[test]
    fn pace_pct_400_from_10_to_50() {
        assert_eq!(pace_pct_change(50, 10), Some(400));
        assert_eq!(pace_pct_change(10, 10), Some(0));
        assert_eq!(pace_pct_change(5, 10), Some(-50));
        assert_eq!(pace_pct_change(100, 0), None);
    }

    #[test]
    fn overtake_in_window_is_not_overall_when_lifetime_differs() {
        let t = trend_contrast(1, Some(4), 50, 10);
        assert!(t.not_overall);
        assert_eq!(t.pace_pct, Some(400));
        assert_eq!(t.window_rank, 1);
        assert_eq!(t.lifetime_rank, Some(4));
        let same = trend_contrast(1, Some(1), 50, 10);
        assert!(!same.not_overall);
    }

    #[test]
    fn attach_lifetime_ranks_fills_slots() {
        let mut window = vec![GridSlot {
            handle: "openclaw".into(),
            rank: 1,
            race_rp: 50,
            velocity: 50,
            momentum: 40,
            gap_to_leader: 0,
            gap_to_next: None,
            pace_pct: Some(400),
            lifetime_rank: None,
            burst_rp: 0,
            sustain_windows: 0,
            paid_rp: 0,
            community_rp: 0,
            badge: None,
            last_overtake: None,
            clicks: 0,
            hover_footer: String::new(),
        }];
        let life = vec![
            GridSlot {
                handle: "hermes".into(),
                rank: 1,
                race_rp: 900,
                velocity: 0,
                momentum: 0,
                gap_to_leader: 0,
                gap_to_next: None,
                pace_pct: None,
                lifetime_rank: None,
                burst_rp: 0,
                sustain_windows: 0,
                paid_rp: 0,
                community_rp: 0,
                badge: None,
                last_overtake: None,
                clicks: 0,
                hover_footer: String::new(),
            },
            GridSlot {
                handle: "openclaw".into(),
                rank: 4,
                race_rp: 100,
                velocity: 0,
                momentum: 0,
                gap_to_leader: 0,
                gap_to_next: None,
                pace_pct: None,
                lifetime_rank: None,
                burst_rp: 0,
                sustain_windows: 0,
                paid_rp: 0,
                community_rp: 0,
                badge: None,
                last_overtake: None,
                clicks: 0,
                hover_footer: String::new(),
            },
        ];
        attach_lifetime_ranks(&mut window, &life);
        assert_eq!(window[0].lifetime_rank, Some(4));
    }

    #[test]
    fn p1_paid_floor_swaps_community_leader() {
        let allocs = vec![
            alloc_src("crowd", 100, 10, "bonus"),
            alloc_src("sponsor", 10, 20, "paid"),
        ];
        let (grid, _) = compute_grid(&allocs, &[], &cfg());
        assert_eq!(grid[0].handle, "sponsor");
        assert_eq!(grid[0].rank, 1);
        assert_eq!(grid[0].paid_rp, 10);
        assert_eq!(grid[1].handle, "crowd");
        assert_eq!(grid[1].rank, 2);
        assert_eq!(grid[1].paid_rp, 0);
        assert_eq!(grid[1].community_rp, 100);
        assert_eq!(grid[0].gap_to_leader, 0);
        assert_eq!(grid[1].gap_to_leader, grid[0].race_rp - grid[1].race_rp);
        assert_eq!(grid[0].gap_to_next, Some(grid[0].race_rp - grid[1].race_rp));
    }

    #[test]
    fn burst_counts_only_last_fifteen_minutes() {
        // now = ts(60). burst window = last 900s = ts(45)..=ts(60).
        let allocs = vec![
            alloc("alpha", 40, 10), // 50 min ago — race RP, not burst
            alloc("alpha", 9, 50),  // 10 min ago — burst
            alloc("alpha", 3, 44),  // 16 min ago — just outside burst
        ];
        let (grid, _) = compute_grid(&allocs, &[], &cfg());
        assert_eq!(grid[0].race_rp, 52);
        assert_eq!(grid[0].burst_rp, 9);
    }

    #[test]
    fn badge_photo_when_gap_at_or_under_five() {
        let allocs = vec![alloc("alpha", 100, 10), alloc("beta", 97, 12)];
        let (grid, _) = compute_grid(&allocs, &[], &cfg());
        assert_eq!(grid[0].gap_to_next, Some(3));
        assert_eq!(grid[0].badge.as_deref(), Some("PHOTO"));
    }

    #[test]
    fn hover_footer_held_p1_for_lone_leader() {
        let allocs = vec![alloc("alpha", 1, 1)];
        let (grid, _) = compute_grid(&allocs, &[], &cfg());
        assert_eq!(grid.len(), 1);
        assert_eq!(grid[0].rank, 1);
        assert_eq!(grid[0].hover_footer, "Held P1. Gap 0.");
    }

    #[test]
    fn saturday_sprint_before_hour_uses_this_week() {
        // Wednesday 12:00 UTC in a week whose Monday is 2026-08-24.
        let wed = DateTime::parse_from_rfc3339("2026-08-26T12:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let (start, end) = saturday_sprint_bounds(wed);
        assert_eq!(start.to_rfc3339(), "2026-08-29T16:00:00+00:00");
        assert_eq!(end.to_rfc3339(), "2026-08-29T17:00:00+00:00");
    }

    #[test]
    fn saturday_sprint_after_hour_uses_next_week() {
        let after = DateTime::parse_from_rfc3339("2026-08-29T17:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let (start, end) = saturday_sprint_bounds(after);
        assert_eq!(start.to_rfc3339(), "2026-09-05T16:00:00+00:00");
        assert_eq!(end.to_rfc3339(), "2026-09-05T17:00:00+00:00");
    }

    #[test]
    fn scoring_types_match_championship_sql() {
        assert!(is_scoring_race_type("GRAND_TOUR"));
        assert!(is_scoring_race_type("sprint"));
        assert!(is_scoring_race_type("Green_Flag"));
        assert!(!is_scoring_race_type("CHAMPIONSHIP"));
        assert!(!is_scoring_race_type("TITLE_FIGHT"));
    }
}
