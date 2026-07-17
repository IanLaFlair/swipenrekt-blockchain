use anchor_lang::prelude::*;

use crate::constants::*;
use crate::state::RewardPool;

/// One-time setup of the global reward pool. Its vault is a native-SOL PDA
/// (system-owned, lamports only) created lazily by the first fee transfer, so
/// there is no token account to init here — we only record its bump.
#[derive(Accounts)]
pub struct InitializeRewardPool<'info> {
    #[account(
        init,
        payer = authority,
        space = 8 + RewardPool::INIT_SPACE,
        seeds = [REWARD_POOL_SEED],
        bump
    )]
    pub reward_pool: Account<'info, RewardPool>,

    #[account(mut)]
    pub authority: Signer<'info>,

    pub system_program: Program<'info, System>,
}

pub fn handler(ctx: Context<InitializeRewardPool>) -> Result<()> {
    let (_, vault_bump) =
        Pubkey::find_program_address(&[REWARD_VAULT_SEED], ctx.program_id);

    let pool = &mut ctx.accounts.reward_pool;
    pool.total_collected = 0;
    pool.total_distributed = 0;
    pool.current_period = 0;
    pool.authority = ctx.accounts.authority.key();
    pool.bump = ctx.bumps.reward_pool;
    pool.vault_bump = vault_bump;
    Ok(())
}
