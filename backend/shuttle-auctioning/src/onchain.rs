//! On-chain transaction preparation helpers.
//! Backend builds the Anchor instruction + unsigned tx (with recent blockhash from RPC).
//! Client (Leptos + Phantom) receives base64 tx, signs it, and broadcasts.
//! This enables real web3 flows for project registration and log_paid_rp
//! without putting heavy Anchor client code in the WASM bundle.

use anyhow::{anyhow, Result};
use base64::{engine::general_purpose, Engine as _};
use serde::{Deserialize, Serialize};
use solana_sdk::{
    hash::Hash,
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    system_program,
    transaction::Transaction,
};
use std::str::FromStr;

use crate::config::AppConfig;

/// Default/placeholder; overridden by cfg.program_id at runtime (see config.rs + Secrets.toml)
pub const PROGRAM_ID_STR: &str = "AuCT1oN1Ng111111111111111111111111111111111";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrepareRegisterRequest {
    pub wallet: String,
    pub handle: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrepareRegisterResponse {
    pub tx_base64: String,
    pub project_pda: String,
    pub program_id: String,
    pub note: String,
}

/// Anchor discriminator for `register_project` (sha256("global:register_project")[..8])
const REGISTER_DISCRIMINATOR: [u8; 8] = [0x82, 0x96, 0x79, 0xd8, 0xb7, 0xe1, 0xf3, 0xc0];

/// Anchor discriminator for `log_paid_rp` (sha256("global:log_paid_rp")[..8])
const LOG_PAID_DISCRIMINATOR: [u8; 8] = [0x53, 0x74, 0x1e, 0xd9, 0xdb, 0x1b, 0x1f, 0x89];

/// Anchor discriminator for `open_race` (sha256("global:open_race")[..8])
const OPEN_RACE_DISCRIMINATOR: [u8; 8] = [0xe2, 0x8b, 0xbf, 0xf7, 0xe9, 0xc7, 0xe6, 0x03];

/// Anchor discriminator for `settle_race` (sha256("global:settle_race")[..8])
const SETTLE_RACE_DISCRIMINATOR: [u8; 8] = [0xac, 0x20, 0x48, 0xd4, 0x9b, 0x21, 0xa1, 0xed];

pub fn build_register_project_ix(owner: Pubkey, handle: &str, program_id: Pubkey) -> Result<Instruction> {
    if handle.len() > 32 || handle.is_empty() {
        anyhow::bail!("handle must be 1-32 bytes");
    }
    let (project_pda, _bump) = Pubkey::find_program_address(
        &[b"project", owner.as_ref()],
        &program_id,
    );

    let mut data = REGISTER_DISCRIMINATOR.to_vec();

    // Borsh String: little-endian u32 length prefix + bytes
    let hbytes = handle.as_bytes();
    data.extend_from_slice(&(hbytes.len() as u32).to_le_bytes());
    data.extend_from_slice(hbytes);

    let accounts = vec![
        AccountMeta::new(project_pda, false),
        AccountMeta::new(owner, true),
        AccountMeta::new_readonly(system_program::id(), false),
    ];

    Ok(Instruction::new_with_bytes(program_id, &data, accounts))
}

pub fn build_open_race_ix(payer: Pubkey, project_pda: Pubkey, current_race_nonce: u64, program_id: Pubkey) -> Result<Instruction> {
    let nonce_bytes = current_race_nonce.to_le_bytes();
    let (race_pda, _bump) = Pubkey::find_program_address(
        &[b"race", project_pda.as_ref(), &nonce_bytes],
        &program_id,
    );

    let data = OPEN_RACE_DISCRIMINATOR.to_vec(); // no args

    let accounts = vec![
        AccountMeta::new(project_pda, false),
        AccountMeta::new(race_pda, false),
        AccountMeta::new(payer, true),
        AccountMeta::new_readonly(system_program::id(), false),
    ];

    Ok(Instruction::new_with_bytes(program_id, &data, accounts))
}

fn serialize_race_results(results: &[RaceResultInput]) -> Result<Vec<u8>> {
    let mut data = Vec::new();
    data.extend_from_slice(&(results.len() as u32).to_le_bytes());
    for r in results {
        let entrant = Pubkey::from_str(&r.entrant)?;
        data.extend_from_slice(entrant.as_ref());
        data.extend_from_slice(&r.score.to_le_bytes());
        data.extend_from_slice(&r.rank.to_le_bytes());
    }
    Ok(data)
}

pub fn build_settle_race_ix(settler: Pubkey, project_pda: Pubkey, race_id: u64, results: &[RaceResultInput], program_id: Pubkey) -> Result<Instruction> {
    let race_id_bytes = race_id.to_le_bytes();
    let (race_pda, _bump) = Pubkey::find_program_address(
        &[b"race", project_pda.as_ref(), &race_id_bytes],
        &program_id,
    );

    let config_pda = Pubkey::find_program_address(&[b"config"], &program_id).0;

    let mut data = SETTLE_RACE_DISCRIMINATOR.to_vec();
    let results_bytes = serialize_race_results(results)?;
    data.extend_from_slice(&results_bytes);

    let accounts = vec![
        AccountMeta::new_readonly(config_pda, false),
        AccountMeta::new(race_pda, false),
        AccountMeta::new_readonly(settler, true),
    ];

    Ok(Instruction::new_with_bytes(program_id, &data, accounts))
}

pub fn build_log_paid_rp_ix(
    owner: Pubkey,
    rp_amount: u64,
    lamports_paid: u64,
    memo: &str,
    current_receipt_count: u64,
    program_id: Pubkey,
) -> Result<Instruction> {
    if memo.len() > 64 {
        anyhow::bail!("memo too long (max 64)");
    }
    if lamports_paid == 0 {
        anyhow::bail!("lamports_paid must be > 0");
    }
    let (project_pda, _bump) = Pubkey::find_program_address(
        &[b"project", owner.as_ref()],
        &program_id,
    );

    let payer = owner; // for now, assume the wallet is the project owner (or authority)

    // TODO: load real fee_vault from on-chain config or config. For demo use a placeholder.
    // In real, query the Config.fee_vault or hardcode after init.
    let fee_vault = Pubkey::from_str("FeeVau1t111111111111111111111111111111111").unwrap_or_else(|_| Pubkey::default());

    let config_pda = Pubkey::find_program_address(&[b"config"], &program_id).0;

    let seq_bytes = current_receipt_count.to_le_bytes();
    let (receipt_pda, _bump) = Pubkey::find_program_address(
        &[b"receipt", project_pda.as_ref(), &seq_bytes],
        &program_id,
    );

    let mut data = LOG_PAID_DISCRIMINATOR.to_vec();
    data.extend_from_slice(&rp_amount.to_le_bytes());
    data.extend_from_slice(&lamports_paid.to_le_bytes());

    // Borsh String for memo
    let mbytes = memo.as_bytes();
    data.extend_from_slice(&(mbytes.len() as u32).to_le_bytes());
    data.extend_from_slice(mbytes);

    let accounts = vec![
        AccountMeta::new(project_pda, false),
        AccountMeta::new(payer, true),
        AccountMeta::new(fee_vault, false),
        AccountMeta::new_readonly(config_pda, false),
        AccountMeta::new(receipt_pda, false),
        AccountMeta::new_readonly(system_program::id(), false),
    ];

    Ok(Instruction::new_with_bytes(program_id, &data, accounts))
}


async fn fetch_latest_blockhash(rpc_url: &str) -> Result<String> {
    let client = reqwest::Client::new();
    let payload = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "getLatestBlockhash",
        "params": [ { "commitment": "confirmed" } ]
    });

    let resp: serde_json::Value = client
        .post(rpc_url)
        .json(&payload)
        .send()
        .await?
        .json()
        .await?;

    resp["result"]["value"]["blockhash"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow!("invalid blockhash response from RPC"))
}

pub async fn prepare_register_project(
    cfg: &AppConfig,
    req: PrepareRegisterRequest,
) -> Result<PrepareRegisterResponse> {
    let owner = Pubkey::from_str(&req.wallet)?;
    let program_id = Pubkey::from_str(&cfg.program_id)?;
    let ix = build_register_project_ix(owner, &req.handle, program_id)?;

    let blockhash_str = fetch_latest_blockhash(&cfg.mainnet_rpc).await?;
    let recent_blockhash = Hash::from_str(&blockhash_str)?;

    let mut tx = Transaction::new_with_payer(&[ix], Some(&owner));
    tx.message.recent_blockhash = recent_blockhash;

    let tx_bytes = bincode::serialize(&tx)?;
    let tx_base64 = general_purpose::STANDARD.encode(tx_bytes);

    let (pda, _) = Pubkey::find_program_address(
        &[b"project", owner.as_ref()],
        &program_id,
    );

    Ok(PrepareRegisterResponse {
        tx_base64,
        project_pda: pda.to_string(),
        program_id: cfg.program_id.clone(),
        note: "Sign this transaction with Phantom to create your on-chain project account. Free RP and allocations remain private.".into(),
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrepareLogPaidRequest {
    pub wallet: String,
    pub rp_amount: u64,
    pub lamports_paid: u64,
    pub memo: String,
    pub current_receipt_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrepareLogPaidResponse {
    pub tx_base64: String,
    pub project_pda: String,
    pub receipt_pda: String,
    pub program_id: String,
    pub note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrepareOpenRaceRequest {
    pub wallet: String,
    pub project_pda: String,
    pub current_race_nonce: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrepareOpenRaceResponse {
    pub tx_base64: String,
    pub project_pda: String,
    pub race_pda: String,
    pub program_id: String,
    pub note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaceResultInput {
    pub entrant: String, // base58 pubkey
    pub score: u64,
    pub rank: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrepareSettleRaceRequest {
    pub wallet: String,
    pub project_pda: String,
    pub race_id: u64,
    pub results: Vec<RaceResultInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrepareSettleRaceResponse {
    pub tx_base64: String,
    pub project_pda: String,
    pub race_pda: String,
    pub program_id: String,
    pub note: String,
}

pub async fn prepare_log_paid_rp(
    cfg: &AppConfig,
    req: PrepareLogPaidRequest,
) -> Result<PrepareLogPaidResponse> {
    let owner = Pubkey::from_str(&req.wallet)?;
    let program_id = Pubkey::from_str(&cfg.program_id)?;
    let ix = build_log_paid_rp_ix(
        owner,
        req.rp_amount,
        req.lamports_paid,
        &req.memo,
        req.current_receipt_count,
        program_id,
    )?;

    let blockhash_str = fetch_latest_blockhash(&cfg.mainnet_rpc).await?;
    let recent_blockhash = Hash::from_str(&blockhash_str)?;

    let mut tx = Transaction::new_with_payer(&[ix], Some(&owner));
    tx.message.recent_blockhash = recent_blockhash;

    let tx_bytes = bincode::serialize(&tx)?;
    let tx_base64 = general_purpose::STANDARD.encode(tx_bytes);

    let program_id = Pubkey::from_str(&cfg.program_id)?;
    let (project_pda, _) = Pubkey::find_program_address(
        &[b"project", owner.as_ref()],
        &program_id,
    );

    let seq_bytes = req.current_receipt_count.to_le_bytes();
    let (receipt_pda, _) = Pubkey::find_program_address(
        &[b"receipt", project_pda.as_ref(), &seq_bytes],
        &program_id,
    );

    Ok(PrepareLogPaidResponse {
        tx_base64,
        project_pda: project_pda.to_string(),
        receipt_pda: receipt_pda.to_string(),
        program_id: cfg.program_id.clone(),
        note: "Sign this transaction with Phantom. This will transfer lamports and log immutable paid RP receipt on-chain.".into(),
    })
}

pub async fn prepare_open_race(
    cfg: &AppConfig,
    req: PrepareOpenRaceRequest,
) -> Result<PrepareOpenRaceResponse> {
    let payer = Pubkey::from_str(&req.wallet)?;
    let project_pda = Pubkey::from_str(&req.project_pda)?;
    let program_id = Pubkey::from_str(&cfg.program_id)?;

    let ix = build_open_race_ix(payer, project_pda, req.current_race_nonce, program_id)?;

    let blockhash_str = fetch_latest_blockhash(&cfg.mainnet_rpc).await?;
    let recent_blockhash = Hash::from_str(&blockhash_str)?;

    let mut tx = Transaction::new_with_payer(&[ix], Some(&payer));
    tx.message.recent_blockhash = recent_blockhash;

    let tx_bytes = bincode::serialize(&tx)?;
    let tx_base64 = general_purpose::STANDARD.encode(tx_bytes);

    let program_id = Pubkey::from_str(&cfg.program_id)?;
    let nonce_bytes = req.current_race_nonce.to_le_bytes();
    let (race_pda, _) = Pubkey::find_program_address(
        &[b"race", project_pda.as_ref(), &nonce_bytes],
        &program_id,
    );

    Ok(PrepareOpenRaceResponse {
        tx_base64,
        project_pda: project_pda.to_string(),
        race_pda: race_pda.to_string(),
        program_id: cfg.program_id.clone(),
        note: "Sign this transaction with Phantom to open a new race on the L2 rollup. The race PDA is derived from current nonce.".into(),
    })
}
pub async fn prepare_settle_race(
    cfg: &AppConfig,
    req: PrepareSettleRaceRequest,
) -> Result<PrepareSettleRaceResponse> {
    let settler = Pubkey::from_str(&req.wallet)?;
    let project_pda = Pubkey::from_str(&req.project_pda)?;
    let program_id = Pubkey::from_str(&cfg.program_id)?;

    let ix = build_settle_race_ix(settler, project_pda, req.race_id, &req.results, program_id)?;

    let blockhash_str = fetch_latest_blockhash(&cfg.mainnet_rpc).await?;
    let recent_blockhash = Hash::from_str(&blockhash_str)?;

    let mut tx = Transaction::new_with_payer(&[ix], Some(&settler));
    tx.message.recent_blockhash = recent_blockhash;

    let tx_bytes = bincode::serialize(&tx)?;
    let tx_base64 = general_purpose::STANDARD.encode(tx_bytes);

    let program_id = Pubkey::from_str(&cfg.program_id)?;
    let race_id_bytes = req.race_id.to_le_bytes();
    let (race_pda, _) = Pubkey::find_program_address(
        &[b"race", project_pda.as_ref(), &race_id_bytes],
        &program_id,
    );

    Ok(PrepareSettleRaceResponse {
        tx_base64,
        project_pda: project_pda.to_string(),
        race_pda: race_pda.to_string(),
        program_id: cfg.program_id.clone(),
        note: "Sign this transaction with Phantom to settle the race on-chain (L2 results committed). Only race authority or protocol can settle.".into(),
    })
}
