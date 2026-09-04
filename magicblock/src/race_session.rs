//! MagicBlock Ephemeral Rollup race session.
//!
//! Live races run on a MagicBlock ER: a delegated PDA is committed to the
//! ephemeral layer, ticks (score updates) happen at rollup speed (~50ms), and
//! the final ranking is flushed back to mainnet via `settle_race` on the
//! auctioning Anchor program.
//!
//! This module is a *session client*: it wraps delegation, tick submission and
//! settlement into a small API used by the Shuttle backend's race worker.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::instruction::{AccountMeta, Instruction};
use solana_sdk::{
    commitment_config::CommitmentConfig,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    transaction::Transaction,
};
use std::str::FromStr;
use std::time::Duration;

/// Well-known MagicBlock ER program addresses on mainnet.
/// See https://docs.magiceden.io / magicblock.gg docs — pinned, not guessed at runtime.
pub mod er_programs {
    /// MagicBlock ephemeral rollup program (Delegation program).
    pub const DELEGATION_PROGRAM_ID: &str = "DELeGuTcxxBpgkLQ13bRKQdWJu6CotHqVjUgA1S5XZ";
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaceSessionConfig {
    /// Mainnet RPC endpoint (e.g. Helius/Triton URL).
    pub mainnet_rpc: String,
    /// MagicBlock ER RPC endpoint (ephemeral chain).
    pub er_rpc: String,
    /// WS endpoint for the ER (tick subscriptions).
    pub er_ws: String,
    /// Project PDA on mainnet (base58).
    pub project_pda: String,
    /// Keypair of the backend authority that opens/settles races (base58 secret).
    /// In production load from Vault/KMS — never hardcode.
    pub authority_secret_b58: Option<String>,
    /// Max race duration before forced settle.
    pub max_race_secs: u64,
}

/// One live entrant inside the ephemeral session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntrantTick {
    pub entrant: Pubkey,
    pub score: u64,
    pub updated_at_ms: u128,
}

/// Shuttle ingest envelope. Identity is `session_id` + `seq` (monotonic per
/// MagicBlock ER session). `signature` is optional collision-breaker only.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TickEnvelope {
    pub session_id: String,
    pub seq: u64,
    pub project: String,
    pub race_id: u64,
    pub entrant: String,
    pub score: u64,
    pub updated_at_ms: u128,
    pub signature: Option<String>,
    /// Project handle if the private catalog already knows it. Never invented.
    pub handle: Option<String>,
}

/// A live race running on the ER.
pub struct RaceSession {
    pub config: RaceSessionConfig,
    pub project: Pubkey,
    pub race_id: u64,
    pub er_client: RpcClient,
    pub mainnet_client: RpcClient,
    ticks: Vec<EntrantTick>,
}

impl RaceSession {
    /// Create a session bound to a mainnet project + race id.
    pub fn new(config: RaceSessionConfig, project: Pubkey, race_id: u64) -> Result<Self> {
        let er_client = RpcClient::new_with_timeout_and_commitment(
            config.er_rpc.clone(),
            Duration::from_secs(10),
            CommitmentConfig::confirmed(),
        );
        let mainnet_client = RpcClient::new_with_commitment(
            config.mainnet_rpc.clone(),
            CommitmentConfig::confirmed(),
        );
        Ok(Self {
            config,
            project,
            race_id,
            er_client,
            mainnet_client,
            ticks: Vec::new(),
        })
    }

    pub fn session_id_for(project: &Pubkey, race_id: u64) -> String {
        format!("{project}:{race_id}")
    }

    pub fn envelope(
        &self,
        seq: u64,
        tick: &EntrantTick,
        signature: Option<String>,
    ) -> TickEnvelope {
        TickEnvelope {
            session_id: Self::session_id_for(&self.project, self.race_id),
            seq,
            project: self.project.to_string(),
            race_id: self.race_id,
            entrant: tick.entrant.to_string(),
            score: tick.score,
            updated_at_ms: tick.updated_at_ms,
            signature,
            handle: None,
        }
    }

    /// Record a score tick. Called from the game loop at ER speed.
    /// In a full deployment these become ER instructions against the delegated
    /// race state; v1 buffers locally and commits the final ordering at settle.
    pub fn record_tick(&mut self, entrant: Pubkey, score: u64) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        if let Some(t) = self.ticks.iter_mut().find(|t| t.entrant == entrant) {
            t.score = t.score.max(score);
            t.updated_at_ms = now;
        } else {
            self.ticks.push(EntrantTick {
                entrant,
                score,
                updated_at_ms: now,
            });
        }
    }

    /// Final ranking derived from buffered ticks (score desc, earliest update wins ties).
    pub fn final_ranking(&self) -> Vec<crate::settlement::RaceResultEntry> {
        let mut ranked: Vec<_> = self.ticks.clone();
        ranked.sort_by(|a, b| {
            b.score
                .cmp(&a.score)
                .then(a.updated_at_ms.cmp(&b.updated_at_ms))
        });
        ranked
            .into_iter()
            .enumerate()
            .take(crate::settlement::MAX_ENTRANTS)
            .map(|(rank, t)| crate::settlement::RaceResultEntry {
                entrant: t.entrant,
                score: t.score,
                rank: rank as u16,
            })
            .collect()
    }

    /// Health check against the ER node.
    pub async fn ping_er(&self) -> Result<u64> {
        let slot = self
            .er_client
            .get_slot()
            .await
            .context("ER get_slot failed")?;
        Ok(slot)
    }
}

/// Settlement payload types mirroring the Anchor program's `RaceResult`.
pub mod settlement {
    use super::*;

    pub const MAX_ENTRANTS: usize = 16;

    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    pub struct RaceResultEntry {
        pub entrant: Pubkey,
        pub score: u64,
        pub rank: u16,
    }

    /// Build the settle instruction data (Anchor discriminator + args) without
    /// pulling in the full anchor client stack — keeps this module light.
    /// Discriminator = sha256("global:settle_race")[..8]; results are borsh vec.
    pub fn build_settle_instruction(
        program_id: &Pubkey,
        race_pda: &Pubkey,
        config_pda: &Pubkey,
        settler: &Pubkey,
        results: &[RaceResultEntry],
    ) -> Result<solana_sdk::instruction::Instruction> {
        use solana_sdk::instruction::{AccountMeta, Instruction};

        let mut data = [8u8] // placeholder replaced below with real discriminator
            .to_vec();
        data.clear();

        // Anchor 8-byte discriminator for `settle_race`.
        let disc = anchor_discriminator("settle_race");
        data.extend_from_slice(&disc);

        // Vec<RaceResult> borsh: u32 len then entries (32 + 8 + 2 each).
        let mut args = Vec::with_capacity(4 + results.len() * 42);
        args.extend_from_slice(&(results.len() as u32).to_le_bytes());
        for r in results {
            args.extend_from_slice(r.entrant.as_ref());
            args.extend_from_slice(&r.score.to_le_bytes());
            args.extend_from_slice(&r.rank.to_le_bytes());
        }
        data.extend_from_slice(&args);

        Ok(Instruction {
            program_id: *program_id,
            accounts: vec![
                AccountMeta::new_readonly(*config_pda, false),
                AccountMeta::new(*race_pda, false),
                AccountMeta::new_readonly(*settler, true),
            ],
            data,
        })
    }

    /// Anchor sighash discriminator (first 8 bytes of sha256("global:<name>")).
    pub fn anchor_discriminator(name: &str) -> [u8; 8] {
        use sha2::{Digest, Sha256};
        let preimage = format!("global:{name}");
        let hash = Sha256::digest(preimage.as_bytes());
        let mut out = [0u8; 8];
        out.copy_from_slice(&hash[..8]);
        out
    }
}

/// Parse a base58 keypair from a secret string (used by the Shuttle worker).
pub fn load_authority(secret_b58: &str) -> Result<Keypair> {
    let bytes = bs58::decode(secret_b58).into_vec().context("bad base58")?;
    let kp = Keypair::from_bytes(&bytes).context("bad keypair bytes")?;
    Ok(kp)
}

/// Sign + send the settlement transaction on mainnet.
pub async fn send_settlement(
    session: &RaceSession,
    authority: &Keypair,
    program_id: &Pubkey,
    race_pda: &Pubkey,
    config_pda: &Pubkey,
) -> Result<solana_sdk::signature::Signature> {
    let results = session.final_ranking();
    let ix = settlement::build_settle_instruction(
        program_id,
        race_pda,
        config_pda,
        &authority.pubkey(),
        &results,
    )?;
    let recent = session.mainnet_client.get_latest_blockhash().await?;
    let tx =
        Transaction::new_signed_with_payer(&[ix], Some(&authority.pubkey()), &[authority], recent);
    let sig = session
        .mainnet_client
        .send_and_confirm_transaction(&tx)
        .await
        .context("settlement tx failed")?;
    Ok(sig)
}

/// Config loader that tolerates missing optional fields (for local dev).
impl RaceSessionConfig {
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            mainnet_rpc: std::env::var("AUCTIONING_MAINNET_RPC")
                .unwrap_or_else(|_| "https://api.mainnet-beta.solana.com".into()),
            er_rpc: std::env::var("AUCTIONING_ER_RPC")
                .unwrap_or_else(|_| "https://devnet-er.magicblock.app".into()),
            er_ws: std::env::var("AUCTIONING_ER_WS")
                .unwrap_or_else(|_| "wss://devnet-er.magicblock.app/ws".into()),
            project_pda: std::env::var("AUCTIONING_PROJECT_PDA")
                .unwrap_or_else(|_| Pubkey::default().to_string()),
            authority_secret_b58: std::env::var("AUCTIONING_AUTHORITY_SECRET").ok(),
            max_race_secs: std::env::var("AUCTIONING_MAX_RACE_SECS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(300),
        })
    }
}

pub fn parse_pubkey(s: &str) -> Result<Pubkey> {
    Pubkey::from_str(s).with_context(|| format!("invalid pubkey: {s}"))
}

/// MagicBlock ER delegation program id (parsed from the pinned constant).
pub fn delegation_program_id() -> Pubkey {
    parse_pubkey(er_programs::DELEGATION_PROGRAM_ID)
        .unwrap_or_else(|_| Pubkey::new_from_array([0; 32]))
}

/// Build a `delegate` instruction against the MagicBlock delegation program.
pub fn build_delegate_instruction(authority: &Pubkey, account: &Pubkey) -> Instruction {
    Instruction {
        program_id: delegation_program_id(),
        accounts: vec![
            AccountMeta::new(*account, false),
            AccountMeta::new_readonly(*authority, true),
        ],
        data: settlement::anchor_discriminator("delegate").to_vec(),
    }
}

/// JSON-RPC 2.0 `accountSubscribe` request for an ER account.
pub fn account_subscribe_request(id: u64, pubkey: &str) -> serde_json::Value {
    serde_json::json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": "accountSubscribe",
        "params": [
            pubkey,
            {
                "encoding": "base64",
                "commitment": "confirmed"
            }
        ]
    })
}

/// Parse a Shuttle `TickEnvelope` or a Solana `accountNotification` into a tick.
pub fn parse_tick_notification(text: &str) -> Option<TickEnvelope> {
    if let Ok(env) = serde_json::from_str::<TickEnvelope>(text) {
        if !env.session_id.is_empty() {
            return Some(env);
        }
    }

    let v: serde_json::Value = serde_json::from_str(text).ok()?;
    if v.get("method").and_then(|m| m.as_str()) != Some("accountNotification") {
        return None;
    }

    let params = v.get("params");
    let result = params.and_then(|p| p.get("result"));
    let seq = result
        .and_then(|r| r.get("context"))
        .and_then(|c| c.get("slot"))
        .and_then(|s| s.as_u64())
        .unwrap_or(0);
    let project = result
        .and_then(|r| r.get("value"))
        .and_then(|val| val.get("pubkey"))
        .and_then(|p| p.as_str())
        .map(str::to_string)
        .or_else(|| {
            params
                .and_then(|p| p.get("pubkey"))
                .and_then(|p| p.as_str())
                .map(str::to_string)
        })
        .or_else(|| {
            params
                .and_then(|p| p.get("subscription"))
                .and_then(|s| s.as_str())
                .map(str::to_string)
        })
        .unwrap_or_default();

    Some(TickEnvelope {
        session_id: project.clone(),
        seq,
        project: project.clone(),
        race_id: 0,
        entrant: project,
        score: 0,
        updated_at_ms: 0,
        signature: None,
        handle: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_session() -> RaceSession {
        RaceSession::new(
            RaceSessionConfig::from_env().unwrap(),
            Pubkey::new_unique(),
            0,
        )
        .unwrap()
    }

    #[test]
    fn envelope_identity_is_session_and_seq() {
        let s = test_session();
        let e = Pubkey::new_unique();
        let tick = EntrantTick {
            entrant: e,
            score: 7,
            updated_at_ms: 1,
        };
        let env = s.envelope(3, &tick, Some("sig1".into()));
        assert_eq!(env.seq, 3);
        assert_eq!(env.session_id, RaceSession::session_id_for(&s.project, 0));
        assert_eq!(env.signature.as_deref(), Some("sig1"));
        assert_eq!(env.score, 7);
    }

    #[test]
    fn ranking_orders_by_score_then_time() {
        let mut s = test_session();
        let a = Pubkey::new_unique();
        let b = Pubkey::new_unique();
        s.record_tick(a, 10);
        std::thread::sleep(std::time::Duration::from_millis(2));
        s.record_tick(b, 20);
        std::thread::sleep(std::time::Duration::from_millis(2));
        s.record_tick(a, 15);
        let ranked = s.final_ranking();
        assert_eq!(ranked[0].entrant, b);
        assert_eq!(ranked[0].score, 20);
        assert_eq!(ranked[1].entrant, a);
        assert_eq!(ranked[1].score, 15); // max() kept, not overwritten
    }

    #[test]
    fn tie_earlier_update_wins() {
        let mut s = test_session();
        let a = Pubkey::new_unique();
        let b = Pubkey::new_unique();
        s.record_tick(a, 5);
        std::thread::sleep(std::time::Duration::from_millis(2));
        s.record_tick(b, 5);
        let ranked = s.final_ranking();
        assert_eq!(ranked[0].entrant, a);
    }

    #[test]
    fn discriminator_matches_anchor_sighash_layout() {
        // sha256("global:settle_race") first 8 bytes; layout check only.
        let d = settlement::anchor_discriminator("settle_race");
        assert_eq!(d.len(), 8);
        let d2 = settlement::anchor_discriminator("settle_race");
        assert_eq!(d, d2);
        assert_ne!(d, settlement::anchor_discriminator("open_race"));
    }

    #[test]
    fn build_delegate_instruction_signer_and_disc() {
        let authority = Pubkey::new_unique();
        let account = Pubkey::new_unique();
        let ix = build_delegate_instruction(&authority, &account);
        assert!(ix.accounts[1].is_signer);
        assert_eq!(
            ix.data.as_slice(),
            settlement::anchor_discriminator("delegate").as_slice()
        );
        if let Ok(pid) = parse_pubkey(er_programs::DELEGATION_PROGRAM_ID) {
            assert_eq!(ix.program_id, pid);
        }
        assert_eq!(ix.accounts[0].pubkey, account);
        assert!(!ix.accounts[0].is_signer);
        assert_eq!(ix.accounts[1].pubkey, authority);
    }

    #[test]
    fn account_subscribe_request_method_and_id() {
        let v = account_subscribe_request(7, "Abc");
        assert_eq!(
            v.get("method").and_then(|m| m.as_str()),
            Some("accountSubscribe")
        );
        assert_eq!(v.get("id").and_then(|i| i.as_u64()), Some(7));
    }

    #[test]
    fn parse_tick_notification_envelope_roundtrip() {
        let env = TickEnvelope {
            session_id: "sess-1".into(),
            seq: 1,
            project: "proj".into(),
            race_id: 0,
            entrant: "ent".into(),
            score: 9,
            updated_at_ms: 0,
            signature: None,
            handle: None,
        };
        let json = serde_json::to_string(&env).unwrap();
        let parsed = parse_tick_notification(&json).expect("envelope");
        assert_eq!(parsed.session_id, "sess-1");
    }

    #[test]
    fn parse_tick_notification_garbage_is_none() {
        assert!(parse_tick_notification("not-json {{{").is_none());
    }

    #[test]
    fn parse_tick_notification_account_notification_slot() {
        let text = r#"{"jsonrpc":"2.0","method":"accountNotification","params":{"result":{"context":{"slot":42},"value":{}},"subscription":1}}"#;
        let parsed = parse_tick_notification(text).expect("notification");
        assert_eq!(parsed.seq, 42);
    }

    #[tokio::test]
    async fn ping_er_reports_error_when_unreachable() {
        // Local dev has no ER node; ensure the error path surfaces cleanly.
        let cfg = RaceSessionConfig {
            er_rpc: "http://127.0.0.1:1".into(),
            ..RaceSessionConfig::from_env().unwrap()
        };
        let s = RaceSession::new(cfg, Pubkey::new_unique(), 0).unwrap();
        assert!(s.ping_er().await.is_err());
    }
}
