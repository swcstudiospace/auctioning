//! SLICE A — narrative posts from race_events.
//!
//! Templates are the source of truth and run with zero LLM configured.
//! Optional enrichment may polish copy; timeout / error / missing config
//! fall back to the template with no user-visible failure.
//!
//! Copy is derived only from event fields. Missing fields omit a clause;
//! standings are never invented.

use crate::race_engine::{DerivedEvent, RaceEventKind, RaceEventRow, RaceWindowRow};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NarrativeChannel {
    X,
    TiktokScript,
    InstagramCarousel,
    Newsletter,
    Timeline,
}

impl NarrativeChannel {
    pub fn as_str(self) -> &'static str {
        match self {
            NarrativeChannel::X => "x",
            NarrativeChannel::TiktokScript => "tiktok_script",
            NarrativeChannel::InstagramCarousel => "instagram_carousel",
            NarrativeChannel::Newsletter => "newsletter",
            NarrativeChannel::Timeline => "timeline",
        }
    }

    pub fn all() -> [NarrativeChannel; 5] {
        [
            NarrativeChannel::X,
            NarrativeChannel::TiktokScript,
            NarrativeChannel::InstagramCarousel,
            NarrativeChannel::Newsletter,
            NarrativeChannel::Timeline,
        ]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NarrativeSource {
    Template,
    Llm,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NarrativePost {
    pub event_id: String,
    pub channel: NarrativeChannel,
    pub generated_at: DateTime<Utc>,
    pub body: String,
    pub why_clauses: Vec<String>,
    pub source: NarrativeSource,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NarrativeBundle {
    pub event_id: String,
    pub generated_at: DateTime<Utc>,
    pub posts: Vec<NarrativePost>,
}

/// Facts the renderer is allowed to mention. No inferred standings.
#[derive(Debug, Clone)]
pub struct NarrativeInput {
    pub event_id: String,
    pub occurred_at: DateTime<Utc>,
    pub kind: RaceEventKind,
    pub project_handle: String,
    pub other_handle: Option<String>,
    pub title: String,
    pub summary: String,
    pub from_rank: Option<i32>,
    pub to_rank: Option<i32>,
    pub rp_delta: Option<i64>,
    pub window_slug: Option<String>,
    pub window_name: Option<String>,
}

impl NarrativeInput {
    pub fn from_derived(
        ev: &DerivedEvent,
        occurred_at: DateTime<Utc>,
        window: Option<&RaceWindowRow>,
    ) -> Self {
        let event_id = format!(
            "derived:{}:{}:{}",
            ev.kind.as_str(),
            ev.project_handle,
            occurred_at.timestamp()
        );
        Self {
            event_id,
            occurred_at,
            kind: ev.kind,
            project_handle: ev.project_handle.clone(),
            other_handle: ev.other_handle.clone(),
            title: ev.title.clone(),
            summary: ev.summary.clone(),
            from_rank: ev.from_rank,
            to_rank: ev.to_rank,
            rp_delta: ev.rp_delta,
            window_slug: window.map(|w| w.slug.clone()),
            window_name: window.map(|w| w.name.clone()),
        }
    }

    pub fn from_row(row: &RaceEventRow, window: Option<&RaceWindowRow>) -> Option<Self> {
        let kind = RaceEventKind::parse(&row.event_type)?;
        let project = row.project_handle.clone().unwrap_or_default();
        if project.is_empty() && kind != RaceEventKind::RaceStart && kind != RaceEventKind::RaceFinish
        {
            return None;
        }
        let (from_rank, to_rank, rp_delta) = payload_ranks(&row.payload);
        Some(Self {
            event_id: row.id.to_string(),
            occurred_at: row.created_at,
            kind,
            project_handle: if project.is_empty() {
                "the field".into()
            } else {
                project
            },
            other_handle: row.other_handle.clone(),
            title: row.title.clone(),
            summary: row.summary.clone().unwrap_or_default(),
            from_rank,
            to_rank,
            rp_delta,
            window_slug: window.map(|w| w.slug.clone()),
            window_name: window.map(|w| w.name.clone()),
        })
    }
}

fn payload_ranks(payload: &serde_json::Value) -> (Option<i32>, Option<i32>, Option<i64>) {
    let i32_of = |k: &str| {
        payload
            .get(k)
            .and_then(|v| v.as_i64())
            .map(|n| n as i32)
    };
    let i64_of = |k: &str| payload.get(k).and_then(|v| v.as_i64());
    (i32_of("from_rank"), i32_of("to_rank"), i64_of("rp_delta"))
}

#[derive(Debug, thiserror::Error)]
pub enum LlmError {
    #[error("llm not configured")]
    NotConfigured,
    #[error("llm failed: {0}")]
    Failed(String),
}

/// Optional polish. Implementations must not be required for a usable bundle.
pub trait LlmEnricher {
    fn enrich(
        &self,
        channel: NarrativeChannel,
        template: &str,
        input: &NarrativeInput,
    ) -> Result<String, LlmError>;
}

pub struct NoopEnricher;

impl LlmEnricher for NoopEnricher {
    fn enrich(
        &self,
        _channel: NarrativeChannel,
        _template: &str,
        _input: &NarrativeInput,
    ) -> Result<String, LlmError> {
        Err(LlmError::NotConfigured)
    }
}

pub struct FailingEnricher;

impl LlmEnricher for FailingEnricher {
    fn enrich(
        &self,
        _channel: NarrativeChannel,
        _template: &str,
        _input: &NarrativeInput,
    ) -> Result<String, LlmError> {
        Err(LlmError::Failed("timeout".into()))
    }
}

/// Causal "why" clauses built only from present fields.
pub fn why_clauses(input: &NarrativeInput) -> Vec<String> {
    let mut why = Vec::new();
    if let Some(other) = input.other_handle.as_deref() {
        if !other.is_empty() {
            why.push(match input.kind {
                RaceEventKind::Overtake => format!("{} passed {}", input.project_handle, other),
                RaceEventKind::LeadChange => {
                    format!("{} unseated {} at P1", input.project_handle, other)
                }
                RaceEventKind::PhotoFinish => {
                    format!("{} and {} are split by a photo-finish gap", input.project_handle, other)
                }
                _ => format!("{} vs {}", input.project_handle, other),
            });
        }
    }
    match (input.from_rank, input.to_rank) {
        (Some(from), Some(to)) if from != to => {
            why.push(format!("moved P{from} → P{to}"));
        }
        (None, Some(to)) => why.push(format!("now P{to}")),
        (Some(from), None) => why.push(format!("was P{from}")),
        _ => {}
    }
    if let Some(delta) = input.rp_delta {
        if delta != 0 {
            why.push(format!("margin {delta} RP"));
        }
    }
    if let Some(name) = input.window_name.as_deref() {
        if !name.is_empty() {
            why.push(format!("in {name}"));
        }
    } else if let Some(slug) = input.window_slug.as_deref() {
        if !slug.is_empty() {
            why.push(format!("in {slug}"));
        }
    }
    if why.is_empty() && !input.summary.is_empty() {
        why.push(input.summary.clone());
    }
    why
}

fn ts(input: &NarrativeInput) -> String {
    input.occurred_at.to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

fn verb(kind: RaceEventKind) -> &'static str {
    match kind {
        RaceEventKind::Overtake => "overtook",
        RaceEventKind::PhotoFinish => "is locked in a photo finish with",
        RaceEventKind::LeadChange => "took the lead from",
        RaceEventKind::DarkHorseRise => "surged as a dark horse past",
        RaceEventKind::RaceStart => "lights-out for",
        RaceEventKind::RaceFinish => "took the flag in",
        RaceEventKind::SignificantSpend => "put significant RP on",
        RaceEventKind::ErTick => "ticked on the ER against",
    }
}

fn subject_line(input: &NarrativeInput) -> String {
    match input.other_handle.as_deref() {
        Some(other) if !other.is_empty() => {
            format!("{} {} {}", input.project_handle, verb(input.kind), other)
        }
        _ => match input.kind {
            RaceEventKind::RaceStart => format!("Lights out{}", window_suffix(input)),
            RaceEventKind::RaceFinish => format!("Chequered flag{}", window_suffix(input)),
            RaceEventKind::DarkHorseRise => format!("{} is the dark horse", input.project_handle),
            RaceEventKind::SignificantSpend => {
                format!("{} drew a significant spend", input.project_handle)
            }
            _ => {
                if input.title.is_empty() {
                    format!("{} moved", input.project_handle)
                } else {
                    input.title.clone()
                }
            }
        },
    }
}

fn window_suffix(input: &NarrativeInput) -> String {
    input
        .window_name
        .as_deref()
        .or(input.window_slug.as_deref())
        .map(|w| format!(" — {w}"))
        .unwrap_or_default()
}

fn why_sentence(why: &[String]) -> String {
    if why.is_empty() {
        String::new()
    } else {
        format!("Why: {}.", why.join("; "))
    }
}

pub fn render_channel(channel: NarrativeChannel, input: &NarrativeInput, why: &[String]) -> String {
    let clock = ts(input);
    let headline = subject_line(input);
    let why_s = why_sentence(why);
    let id_short: String = input.event_id.chars().take(8).collect();

    match channel {
        NarrativeChannel::X => {
            let mut body = format!("{clock} 🏁 {headline}.");
            if !why_s.is_empty() {
                body.push(' ');
                body.push_str(&why_s);
            }
            body.push_str(&format!(" [{id_short}]"));
            body
        }
        NarrativeChannel::TiktokScript => {
            let mut beats = vec![
                format!("[0s] Clock {clock}."),
                format!("[2s] Super: {headline}."),
            ];
            if !why.is_empty() {
                beats.push(format!("[6s] VO: {}.", why.join(". ")));
            }
            beats.push(format!("[12s] End card: auctioning.lol · event {id_short}."));
            beats.join("\n")
        }
        NarrativeChannel::InstagramCarousel => {
            let mut slides = vec![
                format!("Slide 1 — {headline}"),
                format!("Slide 2 — {clock}"),
            ];
            if !why.is_empty() {
                slides.push(format!("Slide 3 — {}", why.join(" · ")));
            }
            slides.push(format!("Slide 4 — Follow the tape · {id_short}"));
            slides.join("\n")
        }
        NarrativeChannel::Newsletter => {
            let mut graf = format!("At {clock}, {headline}.");
            if !why_s.is_empty() {
                graf.push(' ');
                graf.push_str(&why_s);
            }
            if !input.summary.is_empty() && input.summary != input.title {
                graf.push(' ');
                graf.push_str(&input.summary);
                if !graf.ends_with('.') {
                    graf.push('.');
                }
            }
            graf.push_str(&format!(" (event {id_short})"));
            graf
        }
        NarrativeChannel::Timeline => {
            let mut line = format!("{clock} · {headline}");
            if !why.is_empty() {
                line.push_str(" · ");
                line.push_str(&why.join(" · "));
            }
            line
        }
    }
}

/// Always produces a full five-channel bundle from templates, then optionally
/// polishes. Enricher failure never drops a channel.
pub fn generate_narrative(
    input: &NarrativeInput,
    enricher: Option<&dyn LlmEnricher>,
    generated_at: DateTime<Utc>,
) -> NarrativeBundle {
    let why = why_clauses(input);
    let mut posts = Vec::with_capacity(5);
    for channel in NarrativeChannel::all() {
        let template = render_channel(channel, input, &why);
        let (body, source) = match enricher {
            Some(e) => match e.enrich(channel, &template, input) {
                Ok(polished) if !polished.trim().is_empty() => (polished, NarrativeSource::Llm),
                _ => (template, NarrativeSource::Template),
            },
            None => (template, NarrativeSource::Template),
        };
        posts.push(NarrativePost {
            event_id: input.event_id.clone(),
            channel,
            generated_at,
            body,
            why_clauses: why.clone(),
            source,
        });
    }
    NarrativeBundle {
        event_id: input.event_id.clone(),
        generated_at,
        posts,
    }
}

// ---------------------------------------------------------------------------
// Persistence
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, sqlx::FromRow, Serialize)]
pub struct StoredPost {
    pub id: Uuid,
    pub event_id: Uuid,
    pub channel: String,
    pub body: String,
    pub why_clauses: Vec<String>,
    pub source: String,
    pub created_at: DateTime<Utc>,
}

pub async fn persist_bundle(
    db: &PgPool,
    event_id: Uuid,
    bundle: &NarrativeBundle,
) -> Result<Vec<StoredPost>, sqlx::Error> {
    let mut out = Vec::new();
    let mut tx = db.begin().await?;
    for post in &bundle.posts {
        let row = sqlx::query_as::<_, StoredPost>(
            r#"
            INSERT INTO narrative_posts (event_id, channel, body, why_clauses, source)
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (event_id, channel) DO UPDATE
              SET body = EXCLUDED.body,
                  why_clauses = EXCLUDED.why_clauses,
                  source = EXCLUDED.source
            RETURNING id, event_id, channel, body, why_clauses, source, created_at
            "#,
        )
        .bind(event_id)
        .bind(post.channel.as_str())
        .bind(&post.body)
        .bind(&post.why_clauses)
        .bind(match post.source {
            NarrativeSource::Template => "template",
            NarrativeSource::Llm => "llm",
        })
        .fetch_one(&mut *tx)
        .await?;
        out.push(row);
    }
    sqlx::query("UPDATE race_events SET content_generated = TRUE WHERE id = $1")
        .bind(event_id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(out)
}

pub async fn tape_for_window(
    db: &PgPool,
    window_id: Uuid,
    limit: i64,
) -> Result<Vec<StoredPost>, sqlx::Error> {
    sqlx::query_as::<_, StoredPost>(
        r#"
        SELECT p.id, p.event_id, p.channel, p.body, p.why_clauses, p.source, p.created_at
        FROM narrative_posts p
        JOIN race_events e ON e.id = p.event_id
        WHERE e.race_window_id = $1
        ORDER BY p.created_at DESC
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

    fn at() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 8, 25, 15, 4, 5).unwrap()
    }

    fn overtake() -> NarrativeInput {
        NarrativeInput {
            event_id: "evt-overtake-1".into(),
            occurred_at: at(),
            kind: RaceEventKind::Overtake,
            project_handle: "beta".into(),
            other_handle: Some("alpha".into()),
            title: "beta climbs to P1".into(),
            summary: "beta overtook alpha (P3 → P1)".into(),
            from_rank: Some(3),
            to_rank: Some(1),
            rp_delta: Some(12),
            window_slug: Some("weekly-gp-2026-08-24".into()),
            window_name: Some("Weekly Grand Prix 2026-08-24".into()),
        }
    }

    fn assert_bundle_complete(bundle: &NarrativeBundle, input: &NarrativeInput) {
        assert_eq!(bundle.event_id, input.event_id);
        assert_eq!(bundle.posts.len(), 5);
        let clock = ts(input);
        for post in &bundle.posts {
            assert!(!post.body.trim().is_empty(), "{:?} empty", post.channel);
            assert!(
                post.body.contains(&clock),
                "{:?} missing timestamp: {}",
                post.channel,
                post.body
            );
            assert!(
                post.why_clauses.iter().any(|c| c.contains("passed alpha"))
                    || post.body.contains("alpha"),
                "{:?} missing causal why",
                post.channel
            );
            assert!(
                post.why_clauses.iter().any(|c| c.contains("P3 → P1")),
                "{:?} missing rank why: {:?}",
                post.channel,
                post.why_clauses
            );
            assert!(
                post.why_clauses.iter().any(|c| c.contains("12 RP")),
                "{:?} missing margin",
                post.channel
            );
        }
        let channels: Vec<_> = bundle.posts.iter().map(|p| p.channel).collect();
        assert_eq!(channels, NarrativeChannel::all().to_vec());
    }

    #[test]
    fn templates_cover_every_channel_with_timestamp_and_why() {
        let input = overtake();
        let bundle = generate_narrative(&input, None, at());
        assert!(bundle.posts.iter().all(|p| p.source == NarrativeSource::Template));
        assert_bundle_complete(&bundle, &input);
    }

    #[test]
    fn llm_off_and_llm_fail_still_emit_templates() {
        let input = overtake();
        let off = generate_narrative(&input, None, at());
        let fail = generate_narrative(&input, Some(&FailingEnricher), at());
        let noop = generate_narrative(&input, Some(&NoopEnricher), at());
        assert_eq!(off, fail);
        assert_eq!(off, noop);
        assert!(fail.posts.iter().all(|p| p.source == NarrativeSource::Template));
        assert_bundle_complete(&fail, &input);
    }

    struct EchoEnricher;
    impl LlmEnricher for EchoEnricher {
        fn enrich(
            &self,
            _channel: NarrativeChannel,
            template: &str,
            _input: &NarrativeInput,
        ) -> Result<String, LlmError> {
            Ok(format!("POLISHED::{template}"))
        }
    }

    #[test]
    fn successful_enricher_marks_llm_source() {
        let input = overtake();
        let bundle = generate_narrative(&input, Some(&EchoEnricher), at());
        assert!(bundle.posts.iter().all(|p| p.source == NarrativeSource::Llm));
        assert!(bundle.posts[0].body.starts_with("POLISHED::"));
    }

    #[test]
    fn missing_fields_omit_clauses_never_invent_standings() {
        let input = NarrativeInput {
            event_id: "evt-sparse".into(),
            occurred_at: at(),
            kind: RaceEventKind::DarkHorseRise,
            project_handle: "gamma".into(),
            other_handle: None,
            title: String::new(),
            summary: String::new(),
            from_rank: None,
            to_rank: None,
            rp_delta: None,
            window_slug: None,
            window_name: None,
        };
        let why = why_clauses(&input);
        assert!(why.is_empty(), "no facts → no invented why: {why:?}");
        let bundle = generate_narrative(&input, None, at());
        for post in &bundle.posts {
            assert!(!post.body.to_lowercase().contains("p1"));
            assert!(!post.body.contains("overtook"));
            assert!(post.body.contains("2026-08-25T15:04:05Z"));
            assert!(post.body.contains("gamma") || post.body.contains("dark horse"));
        }
    }

    #[test]
    fn photo_finish_and_lead_change_templates() {
        let pf = NarrativeInput {
            event_id: "evt-pf".into(),
            occurred_at: at(),
            kind: RaceEventKind::PhotoFinish,
            project_handle: "alpha".into(),
            other_handle: Some("beta".into()),
            title: "Photo finish".into(),
            summary: String::new(),
            from_rank: Some(1),
            to_rank: Some(2),
            rp_delta: Some(3),
            window_slug: None,
            window_name: Some("Sprint".into()),
        };
        let bundle = generate_narrative(&pf, None, at());
        let x = &bundle.posts[0].body;
        assert!(x.contains("photo finish"));
        assert!(x.contains("alpha"));
        assert!(x.contains("beta"));
        assert!(bundle.posts[0]
            .why_clauses
            .iter()
            .any(|c| c.contains("3 RP")));

        let lc = NarrativeInput {
            event_id: "evt-lc".into(),
            occurred_at: at(),
            kind: RaceEventKind::LeadChange,
            project_handle: "beta".into(),
            other_handle: Some("alpha".into()),
            title: String::new(),
            summary: String::new(),
            from_rank: Some(1),
            to_rank: Some(1),
            rp_delta: None,
            window_slug: None,
            window_name: None,
        };
        let why = why_clauses(&lc);
        assert!(why.iter().any(|c| c.contains("unseated")));
        assert!(!why.iter().any(|c| c.contains("P1 → P1")));
    }

    #[test]
    fn race_start_and_finish_do_not_require_a_rival() {
        let start = NarrativeInput {
            event_id: "evt-start".into(),
            occurred_at: at(),
            kind: RaceEventKind::RaceStart,
            project_handle: "alpha".into(),
            other_handle: None,
            title: "Lights out".into(),
            summary: "5 cars on the grid".into(),
            from_rank: None,
            to_rank: Some(1),
            rp_delta: None,
            window_slug: Some("gp".into()),
            window_name: None,
        };
        let bundle = generate_narrative(&start, None, at());
        assert!(bundle.posts.iter().all(|p| !p.body.is_empty()));
        assert!(bundle.posts[0].body.contains("2026-08-25T15:04:05Z"));
    }

    #[test]
    fn from_row_reads_payload_and_skips_unknown_kind() {
        let row = RaceEventRow {
            id: Uuid::nil(),
            race_window_id: Uuid::nil(),
            project_handle: Some("beta".into()),
            other_handle: Some("alpha".into()),
            event_type: "overtake".into(),
            title: "t".into(),
            summary: Some("s".into()),
            is_narrative_worthy: true,
            created_at: at(),
            payload: serde_json::json!({"from_rank": 4, "to_rank": 2, "rp_delta": 9}),
        };
        let input = NarrativeInput::from_row(&row, None).unwrap();
        assert_eq!(input.from_rank, Some(4));
        assert_eq!(input.to_rank, Some(2));
        assert_eq!(input.rp_delta, Some(9));
        let bad = RaceEventRow {
            event_type: "not-a-kind".into(),
            ..row
        };
        assert!(NarrativeInput::from_row(&bad, None).is_none());
    }

    #[test]
    fn channel_wire_names_match_check_constraint() {
        assert_eq!(NarrativeChannel::X.as_str(), "x");
        assert_eq!(NarrativeChannel::TiktokScript.as_str(), "tiktok_script");
        assert_eq!(
            NarrativeChannel::InstagramCarousel.as_str(),
            "instagram_carousel"
        );
        assert_eq!(NarrativeChannel::Newsletter.as_str(), "newsletter");
        assert_eq!(NarrativeChannel::Timeline.as_str(), "timeline");
    }
}
