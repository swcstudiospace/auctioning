//! Narrative publish state machine over `0006` columns.
//!
//! `publish_status` is `draft | approved | published | failed | skipped`.
//! Handlers only record operator decisions — they never call X, TikTok, or
//! any other social network.

use crate::error::{AppError, AppResult};
use axum::extract::{Path, Query, State};
use axum::Json;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PublishStatus {
    Draft,
    Approved,
    Published,
    Failed,
    Skipped,
}

impl PublishStatus {
    pub const ALL: [PublishStatus; 5] = [
        PublishStatus::Draft,
        PublishStatus::Approved,
        PublishStatus::Published,
        PublishStatus::Failed,
        PublishStatus::Skipped,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            PublishStatus::Draft => "draft",
            PublishStatus::Approved => "approved",
            PublishStatus::Published => "published",
            PublishStatus::Failed => "failed",
            PublishStatus::Skipped => "skipped",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "draft" => Some(PublishStatus::Draft),
            "approved" => Some(PublishStatus::Approved),
            "published" => Some(PublishStatus::Published),
            "failed" => Some(PublishStatus::Failed),
            "skipped" => Some(PublishStatus::Skipped),
            _ => None,
        }
    }
}

/// Legal edges: draft→approved|skipped; approved→published|failed|skipped;
/// failed→approved (retry). Everything else is rejected without a write.
pub fn legal_transition(from: PublishStatus, to: PublishStatus) -> bool {
    use PublishStatus::*;
    matches!(
        (from, to),
        (Draft, Approved)
            | (Draft, Skipped)
            | (Approved, Published)
            | (Approved, Failed)
            | (Approved, Skipped)
            | (Failed, Approved)
    )
}

#[derive(Debug, Clone, sqlx::FromRow, Serialize)]
pub struct QueueRow {
    pub id: Uuid,
    pub event_id: Uuid,
    pub channel: String,
    pub body: String,
    pub publish_status: String,
    pub external_post_id: Option<String>,
    pub last_error: Option<String>,
    pub retryable: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct MarkPublishedBody {
    pub external_post_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct QueueQuery {
    pub status: Option<String>,
}

async fn load_row(db: &PgPool, id: Uuid) -> Result<Option<QueueRow>, sqlx::Error> {
    sqlx::query_as::<_, QueueRow>(
        r#"
        SELECT id, event_id, channel, body, publish_status,
               external_post_id, last_error, retryable, created_at
        FROM narrative_posts
        WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(db)
    .await
}

pub async fn set_status(
    db: &PgPool,
    id: Uuid,
    status: PublishStatus,
    external_post_id: Option<&str>,
    last_error: Option<&str>,
    retryable: bool,
) -> Result<Option<QueueRow>, sqlx::Error> {
    let Some(current) = load_row(db, id).await? else {
        return Ok(None);
    };
    let Some(from) = PublishStatus::from_str(&current.publish_status) else {
        return Ok(None);
    };
    if !legal_transition(from, status) {
        return Ok(None);
    }

    sqlx::query_as::<_, QueueRow>(
        r#"
        UPDATE narrative_posts
        SET publish_status = $2,
            external_post_id = $3,
            last_error = $4,
            retryable = $5
        WHERE id = $1 AND publish_status = $6
        RETURNING id, event_id, channel, body, publish_status,
                  external_post_id, last_error, retryable, created_at
        "#,
    )
    .bind(id)
    .bind(status.as_str())
    .bind(external_post_id)
    .bind(last_error)
    .bind(retryable)
    .bind(from.as_str())
    .fetch_optional(db)
    .await
}

pub async fn queue(
    db: &PgPool,
    status_filter: Option<&str>,
    limit: i64,
) -> Result<Vec<QueueRow>, sqlx::Error> {
    let limit = limit.clamp(1, 200);
    match status_filter {
        Some(status) => {
            sqlx::query_as::<_, QueueRow>(
                r#"
                SELECT id, event_id, channel, body, publish_status,
                       external_post_id, last_error, retryable, created_at
                FROM narrative_posts
                WHERE publish_status = $1
                ORDER BY created_at DESC
                LIMIT $2
                "#,
            )
            .bind(status)
            .bind(limit)
            .fetch_all(db)
            .await
        }
        None => {
            sqlx::query_as::<_, QueueRow>(
                r#"
                SELECT id, event_id, channel, body, publish_status,
                       external_post_id, last_error, retryable, created_at
                FROM narrative_posts
                ORDER BY created_at DESC
                LIMIT $1
                "#,
            )
            .bind(limit)
            .fetch_all(db)
            .await
        }
    }
}

async fn apply_status(
    db: &PgPool,
    id: Uuid,
    status: PublishStatus,
    external_post_id: Option<&str>,
    last_error: Option<&str>,
    retryable: bool,
) -> AppResult<QueueRow> {
    let Some(current) = load_row(db, id).await? else {
        return Err(AppError::NotFound);
    };
    match set_status(db, id, status, external_post_id, last_error, retryable).await? {
        Some(row) => Ok(row),
        None => Err(AppError::BadRequest(format!(
            "illegal transition from {} to {}",
            current.publish_status,
            status.as_str()
        ))),
    }
}

pub async fn approve_handler(
    State(state): State<crate::AppState>,
    Path(id): Path<Uuid>,
) -> AppResult<Json<QueueRow>> {
    apply_status(&state.db, id, PublishStatus::Approved, None, None, false)
        .await
        .map(Json)
}

pub async fn skip_handler(
    State(state): State<crate::AppState>,
    Path(id): Path<Uuid>,
) -> AppResult<Json<QueueRow>> {
    apply_status(&state.db, id, PublishStatus::Skipped, None, None, false)
        .await
        .map(Json)
}

/// Records an operator-confirmed publish. Does not post to any network.
pub async fn mark_published_handler(
    State(state): State<crate::AppState>,
    Path(id): Path<Uuid>,
    Json(body): Json<MarkPublishedBody>,
) -> AppResult<Json<QueueRow>> {
    apply_status(
        &state.db,
        id,
        PublishStatus::Published,
        body.external_post_id.as_deref(),
        None,
        false,
    )
    .await
    .map(Json)
}

pub async fn queue_handler(
    State(state): State<crate::AppState>,
    Query(q): Query<QueueQuery>,
) -> AppResult<Json<Vec<QueueRow>>> {
    if let Some(status) = &q.status {
        if PublishStatus::from_str(status).is_none() {
            return Err(AppError::BadRequest(format!(
                "unknown publish_status '{status}'"
            )));
        }
    }
    let rows = queue(&state.db, q.status.as_deref(), 100).await?;
    Ok(Json(rows))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_roundtrip() {
        for status in PublishStatus::ALL {
            assert_eq!(PublishStatus::from_str(status.as_str()), Some(status));
        }
        assert_eq!(PublishStatus::from_str("unknown"), None);
        assert_eq!(PublishStatus::from_str(""), None);
    }

    #[test]
    fn transition_matrix() {
        use PublishStatus::*;
        let legal = [
            (Draft, Approved),
            (Draft, Skipped),
            (Approved, Published),
            (Approved, Failed),
            (Approved, Skipped),
            (Failed, Approved),
        ];
        for from in PublishStatus::ALL {
            for to in PublishStatus::ALL {
                let expect = legal.contains(&(from, to));
                assert_eq!(legal_transition(from, to), expect, "{from:?} -> {to:?}");
            }
        }
    }
}
