use anchor_lang::prelude::*;

declare_id!("AuCT1oN1Ng111111111111111111111111111111111"); // PLACEHOLDER - replace after first mainnet deploy

/// Global protocol config. PDA ["config"].
#[account]
pub struct Config {
    /// Protocol authority (controls treasury + fee settings).
    pub authority: Pubkey,
    /// Fee vault receiving protocol fees (SOL lamports).
    pub fee_vault: Pubkey,
    /// Basis points charged on paid RP purchases (e.g. 300 = 3%). Max 5000.
    pub fee_bps: u16,
    /// Bump seed.
    pub bump: u8,
    /// Reserved for future upgrades.
    pub reserved: [u8; 64],
}

impl Config {
    pub const SPACE: usize = 8 + 32 + 32 + 2 + 1 + 64;
    pub const SEED: &'static str = "config";
}

/// A business / project registered on-chain. PDA ["project", owner_pubkey].
#[account]
pub struct Project {
    /// Wallet that controls this project registration.
    pub owner: Pubkey,
    /// Total RP ever recorded publicly (paid only; free RP never touches chain).
    pub total_rp: u64,
    /// Lamports spent on RP all-time (public provenance of spend).
    pub total_lamports_spent: u64,
    /// Monotonically increasing race id counter for this project.
    pub race_nonce: u64,
    /// Number of immutable receipts written (drives receipt PDAs).
    pub receipt_count: u64,
    /// Arbitrary UTF-8 handle (max 32 bytes), e.g. "beanz-coffee-brisbane".

    pub handle: String,
    /// Bump seed.
    pub bump: u8,
}

impl Project {
    pub const SPACE: usize = 8 + 32 + 8 + 8 + 8 + 8 + (4 + 32) + 1;
    pub const SEED: &'static str = "project";
}

impl Default for Project {
    fn default() -> Self {
        Self {
            owner: Pubkey::default(),
            total_rp: 0,
            total_lamports_spent: 0,
            race_nonce: 0,
            receipt_count: 0,
            handle: String::new(),
            bump: 255,
        }
    }
}

/// A single paid RP purchase event. Immutable public receipt.
/// PDA ["receipt", project, seq] where seq == Project.receipt_count at insert.
#[account]
pub struct RpReceipt {
    /// Project this receipt belongs to.
    pub project: Pubkey,
    /// Payer wallet for this purchase.
    pub payer: Pubkey,
    /// Amount of RP credited (public amount).
    pub rp_amount: u64,
    /// Lamports actually transferred to the fee vault.
    pub lamports_paid: u64,
    /// Slot at which the purchase was confirmed.
    pub slot: u64,
    /// Sequence number within the project (monotonic).
    pub seq: u64,
    /// Free-form memo (max 64 bytes). Never contains PII.

    pub memo: String,
}

impl RpReceipt {
    pub const SEED: &'static str = "receipt";
}

/// One ranked entrant in a race's final results.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Default)]
pub struct RaceResult {
    /// Entrant pubkey (wallet or ephemeral rollup participant).
    pub entrant: Pubkey,
    /// Final score in this race.
    pub score: u64,
    /// Placement (0-based rank).
    pub rank: u16,
}

/// A race session. Opened on mainnet as the settlement anchor; live ticks happen
/// on a MagicBlock Ephemeral Rollup and are flushed back on settle.
/// PDA ["race", project, race_id].
#[account]
pub struct Race {
    pub project: Pubkey,
    /// Race number within the project (from Project.race_nonce).
    pub race_id: u64,
    /// Entity authorized to settle (backend signer or delegated ER authority).
    pub authority: Pubkey,
    /// Unix timestamp when the race opened on mainnet.
    pub opened_at: i64,
    /// Unix timestamp when the race settled. 0 while open.
    pub settled_at: i64,
    /// Final ranking payload committed at settle (max 16 entrants).

    pub results: Vec<RaceResult>,
    /// 0 = open, 1 = settled.
    pub status: u8,
    /// Bump seed.
    pub bump: u8,
}

impl Race {
    pub const MAX_RESULTS: usize = 16;
    pub const SPACE: usize = 8 + 32 + 8 + 32 + 8 + 8 + (4 + Self::MAX_RESULTS * 42) + 1 + 1;
    pub const SEED: &'static str = "race";
    pub const STATUS_OPEN: u8 = 0;
    pub const STATUS_SETTLED: u8 = 1;
}

/// Errors.
#[error_code]
pub enum AuctioningError {
    #[msg("Unauthorized")]
    Unauthorized,
    #[msg("Race is not open")]
    RaceNotOpen,
    #[msg("Race is already settled")]
    RaceAlreadySettled,
    #[msg("Results overflow: too many entrants")]
    TooManyEntrants,
    #[msg("Fee basis points out of range (max 5000)")]
    FeeOutOfRange,
    #[msg("Handle too long (max 32 bytes)")]
    HandleTooLong,
    #[msg("Memo too long (max 64 bytes)")]
    MemoTooLong,
    #[msg("Payment must be greater than zero")]
    ZeroPayment,
    #[msg("Arithmetic overflow")]
    Overflow,
}
