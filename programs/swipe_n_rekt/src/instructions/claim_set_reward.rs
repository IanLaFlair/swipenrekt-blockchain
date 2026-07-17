use anchor_lang::prelude::*;
use anchor_lang::system_program::{transfer, Transfer};

use crate::constants::*;
use crate::errors::SwipeError;
use crate::events::SetRewardClaimed;
use crate::state::RewardPool;

// ============================================================================
// claim_set_reward — pay a user who completed a country set a % share of the
// live reward vault. Eligibility is attested by the pool authority (backend)
// co-signing. The payout is a percentage of the *current* vault balance (never
// a fixed number), so the pool can never go negative. Native SOL.
// ============================================================================

#[derive(Accounts)]
pub struct ClaimSetReward<'info> {
    #[account(
        mut,
        seeds = [REWARD_POOL_SEED],
        bump = reward_pool.bump,
        has_one = authority @ SwipeError::Unauthorized,
    )]
    pub reward_pool: Account<'info, RewardPool>,

    /// Reward vault (system-owned PDA holding lamports only).
    #[account(
        mut,
        seeds = [REWARD_VAULT_SEED],
        bump = reward_pool.vault_bump,
    )]
    pub reward_vault: SystemAccount<'info>,

    /// Recipient of the reward.
    #[account(mut)]
    pub user: SystemAccount<'info>,

    /// Backend keeper that attests set completion by co-signing.
    pub authority: Signer<'info>,

    pub system_program: Program<'info, System>,
}

pub fn handler(
    ctx: Context<ClaimSetReward>,
    country: u8,
    period: u32,
    distribution_bps: u16,
) -> Result<()> {
    require!(
        distribution_bps >= 1 && (distribution_bps as u64) <= BPS_DENOMINATOR,
        SwipeError::InvalidDistributionBps
    );

    let vault_balance = ctx.accounts.reward_vault.lamports();
    require!(vault_balance > 0, SwipeError::InsufficientRewardPool);

    let amount: u64 = (vault_balance as u128)
        .checked_mul(distribution_bps as u128)
        .ok_or(SwipeError::Overflow)?
        .checked_div(BPS_DENOMINATOR as u128)
        .ok_or(SwipeError::Overflow)? as u64;
    require!(amount > 0, SwipeError::InsufficientRewardPool);

    // Transfer from reward vault (system-owned PDA) → user, signed with seeds.
    let signer: &[&[u8]] = &[REWARD_VAULT_SEED, &[ctx.accounts.reward_pool.vault_bump]];

    transfer(
        CpiContext::new_with_signer(
            ctx.accounts.system_program.to_account_info(),
            Transfer {
                from: ctx.accounts.reward_vault.to_account_info(),
                to: ctx.accounts.user.to_account_info(),
            },
            &[signer],
        ),
        amount,
    )?;

    let pool = &mut ctx.accounts.reward_pool;
    pool.total_distributed = pool
        .total_distributed
        .checked_add(amount)
        .ok_or(SwipeError::Overflow)?;
    pool.current_period = period;

    emit!(SetRewardClaimed {
        user: ctx.accounts.user.key(),
        country,
        period,
        amount,
    });
    Ok(())
}
