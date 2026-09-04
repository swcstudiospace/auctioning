use anchor_lang::prelude::*;

pub mod instructions;
pub mod state;

pub use instructions::*;
pub use state::*;

/// Public immutable ledger: project registry, paid RP receipts, race open/settle.
/// Free / promotional RP never enters this program.
#[program]
pub mod auctioning {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>, fee_bps: u16) -> Result<()> {
        crate::instructions::initialize(ctx, fee_bps)
    }

    /// Authority-only: fees, fee vault, pause flag, authority rotation.
    pub fn update_config(
        ctx: Context<UpdateConfig>,
        fee_bps: Option<u16>,
        paused: Option<bool>,
        new_authority: Option<Pubkey>,
    ) -> Result<()> {
        crate::instructions::update_config(ctx, fee_bps, paused, new_authority)
    }

    pub fn register_project(ctx: Context<RegisterProject>, handle: String) -> Result<()> {
        crate::instructions::register_project(ctx, handle)
    }

    /// Paid RP only. Free weekly / bonus / event-multiplier RP stays off-chain.
    pub fn log_paid_rp(
        ctx: Context<LogPaidRp>,
        rp_amount: u64,
        lamports_paid: u64,
        memo: String,
    ) -> Result<()> {
        crate::instructions::log_paid_rp(ctx, rp_amount, lamports_paid, memo)
    }

    pub fn open_race(ctx: Context<OpenRace>) -> Result<()> {
        crate::instructions::open_race(ctx)
    }

    pub fn settle_race(ctx: Context<SettleRace>, results: Vec<RaceResult>) -> Result<()> {
        crate::instructions::settle_race(ctx, results)
    }
}
