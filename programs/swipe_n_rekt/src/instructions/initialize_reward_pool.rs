use anchor_lang::prelude::*;
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};

use crate::constants::*;
use crate::state::RewardPool;

/// One-time setup of the global reward pool + its token vault.
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

    /// Reward pool vault (PDA-owned token account).
    #[account(
        init,
        payer = authority,
        seeds = [REWARD_VAULT_SEED],
        bump,
        token::mint = mint,
        token::authority = reward_pool,
    )]
    pub reward_vault: InterfaceAccount<'info, TokenAccount>,

    pub mint: InterfaceAccount<'info, Mint>,

    #[account(mut)]
    pub authority: Signer<'info>,

    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
}

pub fn handler(ctx: Context<InitializeRewardPool>) -> Result<()> {
    let pool = &mut ctx.accounts.reward_pool;
    pool.total_collected = 0;
    pool.total_distributed = 0;
    pool.current_period = 0;
    pool.authority = ctx.accounts.authority.key();
    pool.mint = ctx.accounts.mint.key();
    pool.bump = ctx.bumps.reward_pool;
    pool.vault_bump = ctx.bumps.reward_vault;
    Ok(())
}
