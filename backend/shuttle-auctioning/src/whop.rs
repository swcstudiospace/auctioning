//! Whop integration: webhook HMAC verification + membership checks.
//! Whop handles fiat payments and community gating; every paid event is
//! dual-written (private ledger here, public receipt on-chain by the payer).

use crate::auth::constant_time_eq;
use crate::config::MoneyUnit;
use hmac::{Hmac, Mac};
use sha2::Sha256;
use sqlx::PgPool;

type HmacSha256 = Hmac<Sha256>;

/// Verify Whop's webhook signature header (`x-whop-signature`) against the raw
/// body using the webhook signing secret. Constant-time compare.
pub fn verify_webhook_signature(secret: &str, body: &[u8], signature_header: &str) -> bool {
    let Ok(mut mac) = HmacSha256::new_from_slice(secret.as_bytes()) else {
        return false;
    };
    mac.update(body);
    let expected = hex::encode(mac.finalize().into_bytes());

    // Header may carry a single sig or "t=...,v1=..." style; take the last v1.
    let provided = signature_header
        .split(',')
        .filter_map(|part| {
            let p = part.trim();
            if let Some(v) = p.strip_prefix("v1=") {
                Some(v.to_string())
            } else if p.contains('=') {
                None
            } else {
                Some(p.to_string())
            }
        })
        .next_back()
        .unwrap_or_default();

    // Hex digests are case-insensitive; normalise then compare in constant time.
    let provided = provided.to_ascii_lowercase();
    constant_time_eq(provided.as_bytes(), expected.as_bytes())
}

#[derive(Debug, serde::Deserialize)]
pub struct WhopWebhookEvent {
    #[serde(default)]
    pub r#type: String,
    #[serde(default)]
    pub data: serde_json::Value,
}

/// Extract the paying wallet + membership tier from a `payment.succeeded` /
/// `membership.went_valid` event. Whop payload shapes vary by event type;
/// this tolerates both `data.member` and flat payloads.
pub struct PaidEvent {
    pub wallet: Option<String>,
    pub product: Option<String>,
    pub amount_cents: Option<i64>,
    /// Whop payment / membership id — used as the private-ledger tx_id.
    pub payment_id: Option<String>,
}

pub fn parse_paid_event(ev: &WhopWebhookEvent) -> PaidEvent {
    parse_paid_event_with_unit(ev, MoneyUnit::Dollars)
}

/// Same as [`parse_paid_event`] with an explicit amount unit
/// (`WHOP_AMOUNT_UNIT`). `Cents` treats a numeric `amount` as already-cents.
pub fn parse_paid_event_with_unit(ev: &WhopWebhookEvent, unit: MoneyUnit) -> PaidEvent {
    let to_cents = |d: f64| match unit {
        MoneyUnit::Dollars => dollars_to_cents(d),
        MoneyUnit::Cents => d.round() as i64,
    };
    let data = &ev.data;
    let get_str =
        |v: &serde_json::Value, k: &str| v.get(k).and_then(|x| x.as_str()).map(|s| s.to_string());
    let member = data.get("member").unwrap_or(data);
    let wallet = get_str(member, "wallet_address")
        .or_else(|| get_str(member, "solana_wallet"))
        .or_else(|| get_str(data, "wallet_address"));
    let product = get_str(data, "product_id").or_else(|| get_str(data, "plan_id"));
    let amount = data
        .get("payment")
        .and_then(|p| p.get("amount"))
        .and_then(|a| a.as_f64())
        .map(to_cents)
        // Flat `data.amount` / `data.final_amount` follow the configured unit.
        .or_else(|| {
            data.get("final_amount")
                .or_else(|| data.get("amount"))
                .and_then(|a| a.as_f64())
                .map(to_cents)
        });
    let payment_id = get_str(data, "payment_id")
        .or_else(|| get_str(data, "id"))
        .or_else(|| {
            data.get("payment")
                .and_then(|p| p.get("id"))
                .and_then(|x| x.as_str())
                .map(|s| s.to_string())
        });
    PaidEvent {
        wallet,
        product,
        amount_cents: amount,
        payment_id,
    }
}

/// Whop amounts arrive as decimal currency units (19.9 == $19.90); normalise
/// to cents without float-precision drift.
fn dollars_to_cents(d: f64) -> i64 {
    (d * 100.0).round() as i64
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WebhookOutcome {
    Recorded,
    Duplicate,
    Ignored,
    Rejected,
}

impl WebhookOutcome {
    pub fn as_str(self) -> &'static str {
        match self {
            WebhookOutcome::Recorded => "recorded",
            WebhookOutcome::Duplicate => "duplicate",
            WebhookOutcome::Ignored => "ignored",
            WebhookOutcome::Rejected => "rejected",
        }
    }
}

/// Keep every verified delivery verbatim for reconciliation and replay.
pub async fn log_webhook(
    db: &PgPool,
    event_type: &str,
    paid: &PaidEvent,
    outcome: WebhookOutcome,
    raw: &serde_json::Value,
) -> Result<(), sqlx::Error> {
    let credited = match outcome {
        WebhookOutcome::Recorded => paid.amount_cents.map(|c| c / 100),
        _ => None,
    };
    sqlx::query(
        r#"
        INSERT INTO whop_webhook_log
            (event_type, payment_id, wallet, product, amount_cents, credited_rp, outcome, raw)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        "#,
    )
    .bind(event_type)
    .bind(&paid.payment_id)
    .bind(&paid.wallet)
    .bind(&paid.product)
    .bind(paid.amount_cents)
    .bind(credited)
    .bind(outcome.as_str())
    .bind(raw)
    .execute(db)
    .await?;
    Ok(())
}

/// Server-side membership check against the Whop REST API.
pub async fn has_active_membership(api_key: &str, wallet_or_user: &str) -> anyhow::Result<bool> {
    let client = reqwest::Client::new();
    let url = format!(
        "https://api.whop.com/api/v2/memberships?user={}",
        urlencode(wallet_or_user)
    );
    let resp = client
        .get(&url)
        .header("Authorization", format!("Bearer {api_key}"))
        .send()
        .await?
        .error_for_status()?;
    let body: serde_json::Value = resp.json().await?;
    Ok(body
        .get("data")
        .and_then(|d| d.as_array())
        .map(|arr| {
            arr.iter().any(|m| {
                m.get("valid").and_then(|v| v.as_bool()).unwrap_or(false)
                    || m.get("status").and_then(|s| s.as_str()) == Some("active")
            })
        })
        .unwrap_or(false))
}

fn urlencode(s: &str) -> String {
    urlencoding_encode(s)
}

// Minimal percent-encoding to avoid pulling a full URL crate for one param.
fn urlencoding_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signature_roundtrip() {
        let secret = "whsec_test_123";
        let body = br#"{"type":"payment.succeeded"}"#;
        let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(body);
        let sig = hex::encode(mac.finalize().into_bytes());
        assert!(verify_webhook_signature(secret, body, &sig));
        assert!(verify_webhook_signature(
            secret,
            body,
            &format!("t=123,v1={sig}")
        ));
        assert!(!verify_webhook_signature(secret, body, "deadbeef"));
        assert!(!verify_webhook_signature("wrong", body, &sig));
    }

    #[test]
    fn parses_flat_and_nested_paid_events() {
        let ev: WhopWebhookEvent =
            serde_json::from_str(r#"{"type":"membership.went_valid","data":{"member":{"wallet_address":"9xQeWvG816bUx9EPjHmaT23yvVM2ZWbrrpZb9PusVFin"},"amount":1990}}"#).unwrap();
        let paid = parse_paid_event(&ev);
        assert_eq!(
            paid.wallet.as_deref(),
            Some("9xQeWvG816bUx9EPjHmaT23yvVM2ZWbrrpZb9PusVFin")
        );
        assert_eq!(paid.amount_cents, Some(199000));
    }

    #[test]
    fn parses_nested_payment_id() {
        let ev: WhopWebhookEvent = serde_json::from_str(
            r#"{"type":"payment.succeeded","data":{"id":"pay_abc","member":{"wallet_address":"9xQeWvG816bUx9EPjHmaT23yvVM2ZWbrrpZb9PusVFin"},"amount":1}}"#,
        )
        .unwrap();
        let paid = parse_paid_event(&ev);
        assert_eq!(paid.payment_id.as_deref(), Some("pay_abc"));
    }

    #[test]
    fn cents_unit_does_not_multiply() {
        let ev: WhopWebhookEvent = serde_json::from_str(
            r#"{"type":"payment.succeeded","data":{"wallet_address":"9xQeWvG816bUx9EPjHmaT23yvVM2ZWbrrpZb9PusVFin","amount":1990}}"#,
        )
        .unwrap();
        assert_eq!(
            parse_paid_event_with_unit(&ev, MoneyUnit::Cents).amount_cents,
            Some(1990)
        );
        assert_eq!(
            parse_paid_event_with_unit(&ev, MoneyUnit::Dollars).amount_cents,
            Some(199000)
        );
    }

    #[test]
    fn final_amount_is_preferred_over_amount() {
        let ev: WhopWebhookEvent = serde_json::from_str(
            r#"{"type":"payment.succeeded","data":{"final_amount":12.5,"amount":99}}"#,
        )
        .unwrap();
        assert_eq!(parse_paid_event(&ev).amount_cents, Some(1250));
    }

    #[test]
    fn signature_compare_is_case_insensitive_hex() {
        let secret = "s";
        let body = b"x";
        let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(body);
        let sig = hex::encode(mac.finalize().into_bytes()).to_uppercase();
        assert!(verify_webhook_signature(secret, body, &sig));
    }

    #[test]
    fn urlencode_basic() {
        assert_eq!(urlencode("abc123"), "abc123");
        assert_eq!(urlencode("a b/c"), "a%20b%2Fc");
    }
}
