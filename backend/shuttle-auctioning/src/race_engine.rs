//! Race-engine core: ranks, velocity, momentum, overtakes, photo finishes.
//!
//! All derived from the append-only `project_allocations` ledger. Nothing
//! here writes free RP on-chain or implies cash-out. Persistence is optional
//! (snapshots + narrative events) so the news layer can consume later.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

/// One allocation sample fed into the pure scorer.
#[derive(Debug, Clone)]
pub struct AllocationSample {
    pub project_handle: String,
    pub amount: i64,
    pub created_at: DateTime<Utc>,
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

    let vel_start = cfg.now - chrono::Duration::seconds(cfg.velocity_secs);
    let prev_vel_start = vel_start - chrono::Duration::seconds(cfg.velocity_secs);

    for a in allocations {
        if a.created_at < cfg.window_start || a.created_at > cfg.window_end {
            continue;
        }
        if a.amount <= 0 {
            continue;
        }
        *totals.entry(a.project_handle.as_str()).or_insert(0) += a.amount;
        last_at
            .entry(a.project_handle.as_str())
            .and_modify(|t| {
                if a.created_at > *t {
                    *t = a.created_at;
                }
            })
            .or_insert(a.created_at);
        if a.created_at >= vel_start && a.created_at <= cfg.now {
            *velocity.entry(a.project_handle.as_str()).or_insert(0) += a.amount;
        } else if a.created_at >= prev_vel_start && a.created_at < vel_start {
            *prev_velocity.entry(a.project_handle.as_str()).or_insert(0) += a.amount;
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

    let leader_rp = handles
        .first()
        .and_then(|h| totals.get(h).copied())
        .unwrap_or(0);

    let mut grid: Vec<GridSlot> = Vec::with_capacity(handles.len());
    for (i, handle) in handles.iter().enumerate() {
        let race_rp = totals.get(handle).copied().unwrap_or(0);
        let vel = velocity.get(handle).copied().unwrap_or(0);
        let pvel = prev_velocity.get(handle).copied().unwrap_or(0);
        let next_rp = handles
            .get(i + 1)
            .and_then(|n| totals.get(n).copied());
        grid.push(GridSlot {
            handle: (*handle).to_string(),
            rank: (i as i32) + 1,
            race_rp,
            velocity: vel,
            momentum: vel - pvel,
            gap_to_leader: leader_rp - race_rp,
            gap_to_next: next_rp.map(|n| race_rp - n),
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
                        None => format!(
                            "{} moved P{} → P{}",
                            slot.handle, prev.rank, slot.rank
                        ),
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

    (grid, events)
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
}

/// Ensure a live weekly Grand Prix exists so the grid is never empty-state.
pub async fn ensure_default_window(db: &PgPool) -> Result<RaceWindowRow, sqlx::Error> {
    let week_start = crate::ledger::current_week_start();
    let week_end = crate::ledger::next_week_start(Utc::now());
    let slug = format!("weekly-gp-{}", week_start.format("%Y-%m-%d"));

    sqlx::query(
        r#"
        INSERT INTO race_windows (slug, name, race_type, status, starts_at, ends_at, rules)
        VALUES ($1, $2, 'GRAND_PRIX', 'live', $3, $4, '{"photo_finish_gap":5,"velocity_secs":3600}'::jsonb)
        ON CONFLICT (slug) DO NOTHING
        "#,
    )
    .bind(&slug)
    .bind(format!("Weekly Grand Prix {}", week_start.format("%Y-%m-%d")))
    .bind(week_start)
    .bind(week_end)
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

async fn load_allocations(
    db: &PgPool,
    tag: Option<&str>,
    window_start: DateTime<Utc>,
    window_end: DateTime<Utc>,
) -> Result<Vec<AllocationSample>, sqlx::Error> {
    let rows: Vec<(String, i64, DateTime<Utc>)> = if let Some(tag) = tag {
        sqlx::query_as(
            r#"
            SELECT a.project_handle, a.amount, a.created_at
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
            SELECT project_handle, amount, created_at
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
        .map(|(project_handle, amount, created_at)| AllocationSample {
            project_handle,
            amount,
            created_at,
        })
        .collect())
}

async fn load_previous_snapshot(
    db: &PgPool,
    window_id: Uuid,
) -> Result<Vec<PreviousSlot>, sqlx::Error> {
    let latest: Option<DateTime<Utc>> = sqlx::query_scalar(
        "SELECT MAX(snapshot_at) FROM rank_snapshots WHERE race_window_id = $1",
    )
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
    let allocs = load_allocations(db, window.tag.as_deref(), cfg.window_start, now).await?;
    let prev = load_previous_snapshot(db, window.id).await?;
    Ok(compute_grid(&allocs, &prev, &cfg))
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
    let allocs = load_allocations(db, window.tag.as_deref(), cfg.window_start, now).await?;
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

pub async fn events_for_window(
    db: &PgPool,
    window_id: Uuid,
    limit: i64,
) -> Result<Vec<RaceEventRow>, sqlx::Error> {
    sqlx::query_as::<_, RaceEventRow>(
        r#"
        SELECT id, race_window_id, project_handle, other_handle,
               event_type, title, summary, is_narrative_worthy, created_at
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
        AllocationSample {
            project_handle: handle.into(),
            amount,
            created_at: ts(min),
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
    }
}
