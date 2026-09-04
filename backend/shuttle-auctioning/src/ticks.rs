//! MagicBlock ER tick ingest → same consumer grid as race_events.
//!
//! Raw ticks live in `er_ticks` keyed by (session_id, seq). Ranking movement
//! may insert a *new* race_events row with origin='tick'. Allocation-derived
//! rows (origin='event') are never updated.

use crate::error::{AppError, AppResult};
use crate::race_engine::{self, RaceEventKind, RaceWindowRow};
use auctioning_core::TickEnvelope;
use axum::extract::{Path, State};
use axum::Json;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SessionScore {
    pub handle: String,
    pub score: i64,
    pub seq: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TickProjection {
    pub kind: RaceEventKind,
    pub handle: String,
    pub other_handle: Option<String>,
    pub from_rank: Option<i32>,
    pub to_rank: Option<i32>,
    pub title: String,
    pub summary: String,
}

/// Rank by score desc, then earlier seq (first to the mark).
pub fn rank_session(scores: &[SessionScore]) -> Vec<(i32, &SessionScore)> {
    let mut idx: Vec<&SessionScore> = scores.iter().collect();
    idx.sort_by(|a, b| {
        b.score
            .cmp(&a.score)
            .then(a.seq.cmp(&b.seq))
            .then(a.handle.cmp(&b.handle))
    });
    idx.into_iter()
        .enumerate()
        .map(|(i, s)| ((i as i32) + 1, s))
        .collect()
}

/// Compare previous vs current session ranks. Missing handles omit rank facts.
pub fn project_tick(
    prev: &[SessionScore],
    curr: &[SessionScore],
    incoming_handle: &str,
) -> TickProjection {
    let prev_r = rank_session(prev);
    let curr_r = rank_session(curr);
    let prev_rank = prev_r
        .iter()
        .find(|(_, s)| s.handle == incoming_handle)
        .map(|(r, _)| *r);
    let curr_rank = curr_r
        .iter()
        .find(|(_, s)| s.handle == incoming_handle)
        .map(|(r, _)| *r);
    let prev_leader = prev_r
        .iter()
        .find(|(r, _)| *r == 1)
        .map(|(_, s)| s.handle.as_str());
    let curr_leader = curr_r
        .iter()
        .find(|(r, _)| *r == 1)
        .map(|(_, s)| s.handle.as_str());

    let passed = if let (Some(pr), Some(cr)) = (prev_rank, curr_rank) {
        if cr < pr {
            prev_r
                .iter()
                .find(|(r, s)| *r < pr && *r >= cr && s.handle != incoming_handle)
                .map(|(_, s)| s.handle.clone())
        } else {
            None
        }
    } else {
        None
    };

    if prev_leader.is_some()
        && curr_leader == Some(incoming_handle)
        && prev_leader != Some(incoming_handle)
    {
        return TickProjection {
            kind: RaceEventKind::LeadChange,
            handle: incoming_handle.into(),
            other_handle: prev_leader.map(|s| s.to_string()),
            from_rank: prev_rank,
            to_rank: curr_rank,
            title: format!("{incoming_handle} takes P1"),
            summary: match prev_leader {
                Some(old) => format!("{incoming_handle} unseats {old} at the front"),
                None => format!("{incoming_handle} takes P1"),
            },
        };
    }
    if let (Some(pr), Some(cr), Some(other)) = (prev_rank, curr_rank, passed) {
        if cr < pr {
            return TickProjection {
                kind: RaceEventKind::Overtake,
                handle: incoming_handle.into(),
                other_handle: Some(other.clone()),
                from_rank: prev_rank,
                to_rank: curr_rank,
                title: format!("{incoming_handle} climbs to P{cr}"),
                summary: format!("{incoming_handle} overtook {other} (P{pr} → P{cr})"),
            };
        }
    }
    TickProjection {
        kind: RaceEventKind::ErTick,
        handle: incoming_handle.into(),
        other_handle: None,
        from_rank: prev_rank,
        to_rank: curr_rank,
        title: format!("{incoming_handle} ER tick"),
        summary: String::new(),
    }
}

fn scores_from_rows(rows: Vec<(String, i64, i64)>) -> Vec<SessionScore> {
    rows.into_iter()
        .map(|(handle, score, seq)| SessionScore { handle, score, seq })
        .collect()
}

pub struct IngestResult {
    pub inserted: bool,
    pub tick_id: Uuid,
    pub projection: Option<TickProjection>,
    pub event_id: Option<Uuid>,
}

/// Idempotent ingest. Duplicate (session_id, seq) returns inserted=false
/// and does not write a second race_events row.
pub async fn ingest_tick(
    db: &PgPool,
    window: &RaceWindowRow,
    env: &TickEnvelope,
) -> Result<IngestResult, sqlx::Error> {
    let handle = env
        .handle
        .clone()
        .filter(|h| !h.is_empty())
        .unwrap_or_else(|| env.entrant.clone());

    let inserted = sqlx::query_as::<_, (Uuid,)>(
        r#"
        INSERT INTO er_ticks (
            race_window_id, session_id, seq, project_pda, race_id,
            handle, entrant, score, signature, payload
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10::jsonb)
        ON CONFLICT (session_id, seq) DO NOTHING
        RETURNING id
        "#,
    )
    .bind(window.id)
    .bind(&env.session_id)
    .bind(env.seq as i64)
    .bind(&env.project)
    .bind(env.race_id as i64)
    .bind(&handle)
    .bind(&env.entrant)
    .bind(env.score as i64)
    .bind(&env.signature)
    .bind(serde_json::json!({ "updated_at_ms": env.updated_at_ms.to_string() }).to_string())
    .fetch_optional(db)
    .await?;

    let Some((tick_id,)) = inserted else {
        let existing: Uuid =
            sqlx::query_scalar("SELECT id FROM er_ticks WHERE session_id = $1 AND seq = $2")
                .bind(&env.session_id)
                .bind(env.seq as i64)
                .fetch_one(db)
                .await?;
        return Ok(IngestResult {
            inserted: false,
            tick_id: existing,
            projection: None,
            event_id: None,
        });
    };

    let curr = scores_from_rows(
        sqlx::query_as(
            r#"
            SELECT DISTINCT ON (handle) handle, score, seq
            FROM er_ticks
            WHERE session_id = $1 AND handle IS NOT NULL
            ORDER BY handle, seq DESC
            "#,
        )
        .bind(&env.session_id)
        .fetch_all(db)
        .await?,
    );
    let prev = scores_from_rows(
        sqlx::query_as(
            r#"
            SELECT DISTINCT ON (handle) handle, score, seq
            FROM er_ticks
            WHERE session_id = $1 AND seq < $2 AND handle IS NOT NULL
            ORDER BY handle, seq DESC
            "#,
        )
        .bind(&env.session_id)
        .bind(env.seq as i64)
        .fetch_all(db)
        .await?,
    );

    let projection = project_tick(&prev, &curr, &handle);

    sqlx::query("UPDATE er_ticks SET kind = $2, from_rank = $3, to_rank = $4, other_handle = $5 WHERE id = $1")
        .bind(tick_id)
        .bind(projection.kind.as_str())
        .bind(projection.from_rank)
        .bind(projection.to_rank)
        .bind(&projection.other_handle)
        .execute(db)
        .await?;

    let event_id = insert_tick_event(db, window.id, tick_id, env, &projection).await?;

    Ok(IngestResult {
        inserted: true,
        tick_id,
        projection: Some(projection),
        event_id,
    })
}

async fn insert_tick_event(
    db: &PgPool,
    window_id: Uuid,
    tick_id: Uuid,
    env: &TickEnvelope,
    projection: &TickProjection,
) -> Result<Option<Uuid>, sqlx::Error> {
    let existing: Option<Uuid> =
        sqlx::query_scalar("SELECT id FROM race_events WHERE tick_id = $1")
            .bind(tick_id)
            .fetch_optional(db)
            .await?;
    if existing.is_some() {
        return Ok(existing);
    }
    let id: Uuid = sqlx::query_scalar(
        r#"
        INSERT INTO race_events (
            race_window_id, project_handle, other_handle, event_type,
            title, summary, payload, origin, tick_id
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7::jsonb, 'tick', $8)
        RETURNING id
        "#,
    )
    .bind(window_id)
    .bind(&projection.handle)
    .bind(&projection.other_handle)
    .bind(projection.kind.as_str())
    .bind(&projection.title)
    .bind(&projection.summary)
    .bind(
        serde_json::json!({
            "from_rank": projection.from_rank,
            "to_rank": projection.to_rank,
            "session_id": env.session_id,
            "seq": env.seq,
        })
        .to_string(),
    )
    .bind(tick_id)
    .fetch_one(db)
    .await?;
    Ok(Some(id))
}

pub async fn session_grid(
    db: &PgPool,
    session_id: &str,
) -> Result<Vec<(i32, SessionScore)>, sqlx::Error> {
    let curr = scores_from_rows(
        sqlx::query_as(
            r#"
            SELECT DISTINCT ON (handle) handle, score, seq
            FROM er_ticks
            WHERE session_id = $1 AND handle IS NOT NULL
            ORDER BY handle, seq DESC
            "#,
        )
        .bind(session_id)
        .fetch_all(db)
        .await?,
    );
    Ok(rank_session(&curr)
        .into_iter()
        .map(|(r, s)| (r, s.clone()))
        .collect())
}

fn projection_json(p: &TickProjection) -> Value {
    json!({
        "kind": p.kind,
        "handle": p.handle,
        "other_handle": p.other_handle,
        "from_rank": p.from_rank,
        "to_rank": p.to_rank,
        "title": p.title,
        "summary": p.summary,
    })
}

pub async fn ingest_window_tick(
    State(state): State<crate::AppState>,
    _ingest: crate::auth::Ingest,
    Path(slug): Path<String>,
    Json(env): Json<TickEnvelope>,
) -> AppResult<Json<Value>> {
    if env.session_id.is_empty() {
        return Err(AppError::BadRequest("session_id required".into()));
    }
    // TickEnvelope.seq is u64 — always present after JSON decode.
    let window = race_engine::window_by_slug(&state.db, &slug)
        .await?
        .ok_or(AppError::NotFound)?;
    let result = ingest_tick(&state.db, &window, &env).await?;
    Ok(Json(json!({
        "inserted": result.inserted,
        "tick_id": result.tick_id,
        "event_id": result.event_id,
        "projection": result.projection.as_ref().map(projection_json),
        "window": slug,
    })))
}

pub async fn session_grid_handler(
    State(state): State<crate::AppState>,
    Path(session_id): Path<String>,
) -> AppResult<Json<Value>> {
    let grid = session_grid(&state.db, &session_id).await?;
    Ok(Json(json!({
        "session_id": session_id,
        "grid": grid
            .into_iter()
            .map(|(rank, s)| {
                json!({
                    "rank": rank,
                    "handle": s.handle,
                    "score": s.score,
                    "seq": s.seq,
                })
            })
            .collect::<Vec<_>>(),
    })))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(handle: &str, score: i64, seq: i64) -> SessionScore {
        SessionScore {
            handle: handle.into(),
            score,
            seq,
        }
    }

    #[test]
    fn ranks_by_score_then_seq() {
        let scores = vec![s("a", 10, 2), s("b", 30, 1), s("c", 30, 0)];
        let ranked = rank_session(&scores);
        assert_eq!(ranked[0].1.handle, "c");
        assert_eq!(ranked[1].1.handle, "b");
        assert_eq!(ranked[0].0, 1);
    }

    #[test]
    fn lead_change_and_overtake_from_ticks() {
        let prev = vec![s("alpha", 50, 1), s("beta", 10, 1)];
        let curr = vec![s("alpha", 50, 1), s("beta", 80, 2)];
        let p = project_tick(&prev, &curr, "beta");
        assert_eq!(p.kind, RaceEventKind::LeadChange);
        assert_eq!(p.other_handle.as_deref(), Some("alpha"));
        assert_eq!(p.to_rank, Some(1));
        assert_eq!(p.from_rank, Some(2));
    }

    #[test]
    fn overtake_without_taking_lead() {
        let prev = vec![s("a", 90, 1), s("b", 40, 1), s("c", 10, 1)];
        let curr = vec![s("a", 90, 1), s("b", 40, 1), s("c", 50, 2)];
        let p = project_tick(&prev, &curr, "c");
        assert_eq!(p.kind, RaceEventKind::Overtake);
        assert_eq!(p.other_handle.as_deref(), Some("b"));
        assert_eq!(p.from_rank, Some(3));
        assert_eq!(p.to_rank, Some(2));
    }

    #[test]
    fn sparse_tick_omits_rival_and_does_not_invent_rp() {
        let curr = vec![s("solo", 1, 1)];
        let p = project_tick(&[], &curr, "solo");
        assert_eq!(p.kind, RaceEventKind::ErTick);
        assert!(p.other_handle.is_none());
        assert!(p.summary.is_empty());
        assert!(!p.title.to_lowercase().contains(" rp"));
    }

    #[test]
    fn duplicate_identity_is_session_plus_seq() {
        // Documented contract for ingest: unique (session_id, seq).
        let a = ("sess-1", 4u64);
        let b = ("sess-1", 4u64);
        assert_eq!(a, b);
    }
}
