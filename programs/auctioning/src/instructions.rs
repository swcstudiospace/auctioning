use crate::state::*;
use anchor_lang::prelude::*;
use anchor_lang::system_program;

/// Initialize global config. One-time.
pub fn initialize(ctx: Context<Initialize>, fee_bps: u16) -> Result<()> {
    require!(fee_bps <= 5000, AuctioningError::FeeOutOfRange);
    let config = &mut ctx.accounts.config;
    config.authority = ctx.accounts.authority.key();
    config.fee_vault = ctx.accounts.fee_vault.key();
    config.fee_bps = fee_bps;
    config.bump = ctx.bumps.config;
    Ok(())
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(
        init,
        payer = authority,
        space = Config::SPACE,
        seeds = [Config::SEED.as_bytes()],
        bump
    )]
    pub config: Account<'info, Config>,
    /// CHECK: plain wallet receiving protocol fees.
    pub fee_vault: UncheckedAccount<'info>,
    #[account(mut)]
    pub authority: Signer<'info>,
    pub system_program: Program<'info, system_program::System>,
}

/// Register a project (business). Owner pays rent; PDA derived from owner key.
pub fn register_project(ctx: Context<RegisterProject>, handle: String) -> Result<()> {
    require!(handle.len() <= 32, AuctioningError::HandleTooLong);
    let project = &mut ctx.accounts.project;
    project.owner = ctx.accounts.owner.key();
    project.total_rp = 0;
    project.total_lamports_spent = 0;
    project.race_nonce = 0;
    project.receipt_count = 0;
    project.handle = handle;
    project.bump = ctx.bumps.project;
    Ok(())
}

#[derive(Accounts)]
pub struct RegisterProject<'info> {
    #[account(
        init,
        payer = owner,
        space = Project::SPACE,
        seeds = [Project::SEED.as_bytes(), owner.key().as_ref()],
        bump
    )]
    pub project: Account<'info, Project>,
    #[account(mut)]
    pub owner: Signer<'info>,
    pub system_program: Program<'info, system_program::System>,
}

/// Log a paid RP purchase: transfer `lamports_paid` from payer to fee vault,
/// bump public project totals, write an immutable receipt at PDA
/// ["receipt", project, seq]. Free/promo RP NEVER goes through this instruction.
///
/// NOTE on receipt PDA derivation: the receipt seed uses the receipt counter
/// value BEFORE this instruction runs. The client derives the expected PDA from
/// a pre-flight getAccountInfo of Project.receipt_count — standard Anchor
/// "counter PDA" pattern.
pub fn log_paid_rp(
    ctx: Context<LogPaidRp>,
    rp_amount: u64,
    lamports_paid: u64,
    memo: String,
) -> Result<()> {
    require!(lamports_paid > 0, AuctioningError::ZeroPayment);
    require!(memo.len() <= 64, AuctioningError::MemoTooLong);
    require_keys_eq!(ctx.accounts.fee_vault.key(), ctx.accounts.config.fee_vault);

    // CPI: payer -> fee vault. (anchor-lang 1.1.x: CpiContext::new takes the
    // program id directly.)
    system_program::transfer(
        CpiContext::new(
            ctx.accounts.system_program.key(),
            system_program::Transfer {
                from: ctx.accounts.payer.to_account_info(),
                to: ctx.accounts.fee_vault.to_account_info(),
            },
        ),
        lamports_paid,
    )?;

    // seq is read before the counter bump so it matches the PDA the client derived.
    let seq = ctx.accounts.project.receipt_count;

    let project = &mut ctx.accounts.project;
    project.total_rp = project
        .total_rp
        .checked_add(rp_amount)
        .ok_or(AuctioningError::Overflow)?;
    project.total_lamports_spent = project
        .total_lamports_spent
        .checked_add(lamports_paid)
        .ok_or(AuctioningError::Overflow)?;
    project.receipt_count = project
        .receipt_count
        .checked_add(1)
        .ok_or(AuctioningError::Overflow)?;

    let receipt = &mut ctx.accounts.receipt;
    receipt.project = project.key();
    receipt.payer = ctx.accounts.payer.key();
    receipt.rp_amount = rp_amount;
    receipt.lamports_paid = lamports_paid;
    receipt.slot = Clock::get()?.slot;
    receipt.seq = seq;
    receipt.memo = memo;

    emit!(RpLogged {
        project: project.key(),
        payer: ctx.accounts.payer.key(),
        rp_amount,
        lamports_paid,
    });
    Ok(())
}

#[derive(Accounts)]
pub struct LogPaidRp<'info> {
    #[account(mut)]
    pub project: Account<'info, Project>,
    #[account(
        mut,
        constraint = payer.key() == project.owner || payer.key() == config_authority(&config)
            @ AuctioningError::Unauthorized
    )]
    pub payer: Signer<'info>,
    /// CHECK: must equal config.fee_vault (checked in handler).
    #[account(mut)]
    pub fee_vault: UncheckedAccount<'info>,
    #[account(seeds = [Config::SEED.as_bytes()], bump)]
    pub config: Box<Account<'info, Config>>,
    #[account(
        init,
        payer = payer,
        space = RpReceiptSpace::SIZE,
        seeds = [
            RpReceipt::SEED.as_bytes(),
            project.key().as_ref(),
            &receipt_seq(&project.receipt_count)
        ],
        bump
    )]
    pub receipt: Account<'info, RpReceipt>,
    pub system_program: Program<'info, system_program::System>,
}

fn config_authority(config: &Account<Config>) -> Pubkey {
    config.authority
}

fn receipt_seq(count: &u64) -> [u8; 8] {
    count.to_le_bytes()
}

/// Const space mirror so the Accounts constraint stays version-stable.
pub struct RpReceiptSpace;
impl RpReceiptSpace {
    pub const SIZE: usize = 8 + 32 + 32 + 8 + 8 + 8 + 8 + (4 + 64);
}

#[event]
pub struct RpLogged {
    pub project: Pubkey,
    pub payer: Pubkey,
    pub rp_amount: u64,
    pub lamports_paid: u64,
}

/// Open a race on mainnet. Increments the project's race nonce.
///
/// NOTE: `project.race_nonce` is read BEFORE the bump inside the handler, so
/// it equals the id of the race being created. The Accounts constraint uses
/// that same pre-bump value for the PDA seed, matching the client derivation:
///   seeds = ["race", project.key(), race_id.to_le_bytes()]
pub fn open_race(ctx: Context<OpenRace>) -> Result<()> {
    let race_id = ctx.accounts.project.race_nonce;
    let project = &mut ctx.accounts.project;
    project.race_nonce = race_id.checked_add(1).ok_or(AuctioningError::Overflow)?;

    let race = &mut ctx.accounts.race;
    race.project = project.key();
    race.race_id = race_id;
    race.authority = ctx.accounts.payer.key();
    race.opened_at = Clock::get()?.unix_timestamp;
    race.settled_at = 0;
    race.results = Vec::new();
    race.status = Race::STATUS_OPEN;
    race.bump = ctx.bumps.race;

    emit!(RaceOpened {
        project: race.project,
        race_id
    });
    Ok(())
}

#[derive(Accounts)]
pub struct OpenRace<'info> {
    #[account(mut)]
    pub project: Account<'info, Project>,
    #[account(
        init,
        payer = payer,
        space = Race::SPACE,
        // Seeds use the PRE-bump nonce: Anchor runs `init` constraints before
        // the handler body bumps the counter, so project.race_nonce here still
        // equals the id of the race being created.
        seeds = [
            Race::SEED.as_bytes(),
            project.key().as_ref(),
            &project.race_nonce.to_le_bytes()
        ],
        bump
    )]
    pub race: Account<'info, Race>,
    #[account(mut)]
    pub payer: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[event]
pub struct RaceOpened {
    pub project: Pubkey,
    pub race_id: u64,
}

/// Settle a race: commit final results from the ephemeral rollup session.
/// Only the race authority or the protocol authority may settle.
pub fn settle_race(ctx: Context<SettleRace>, results: Vec<RaceResult>) -> Result<()> {
    require!(
        results.len() <= Race::MAX_RESULTS,
        AuctioningError::TooManyEntrants
    );
    let race = &mut ctx.accounts.race;
    require!(
        race.status == Race::STATUS_OPEN,
        AuctioningError::RaceAlreadySettled
    );

    let is_race_authority = ctx.accounts.settler.key() == race.authority;
    let is_protocol_authority = ctx.accounts.settler.key() == ctx.accounts.config.authority;
    require!(
        is_race_authority || is_protocol_authority,
        AuctioningError::Unauthorized
    );

    race.results = results;
    race.settled_at = Clock::get()?.unix_timestamp;
    race.status = Race::STATUS_SETTLED;

    emit!(RaceSettled {
        project: race.project,
        race_id: race.race_id
    });
    Ok(())
}

#[derive(Accounts)]
pub struct SettleRace<'info> {
    #[account(seeds = [Config::SEED.as_bytes()], bump)]
    pub config: Box<Account<'info, Config>>,
    #[account(
        mut,
        seeds = [
            Race::SEED.as_bytes(),
            race.project.as_ref(),
            &race.race_id.to_le_bytes()
        ],
        bump = race.bump
    )]
    pub race: Account<'info, Race>,
    pub settler: Signer<'info>,
}

#[event]
pub struct RaceSettled {
    pub project: Pubkey,
    pub race_id: u64,
}
