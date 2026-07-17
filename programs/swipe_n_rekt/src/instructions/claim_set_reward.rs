use anchor_lang::prelude::*;
use anchor_spl::token_interface::{
    transfer_checked, Mint, TokenAccount, TokenInterface, TransferChecked,
};

use crate::constants::*;
use crate::errors::SwipeError;
use crate::events::SetRewardClaimed;
use crate::state::RewardPool;

// ============================================================================
// claim_set_reward — pay a user who completed a country set a % share of the
// live reward pool. Eligibility is attested by the pool authority (backend)
// co-signing this instruction. The payout is a percentage of the *current*
// vault balance (never a fixed number), so the pool can never go negative.
// ============================================================================

#[derive(Accounts)]
pub struct ClaimSetReward<'info> {
    #[account(
        mut,
        seeds = [REWARD_POOL_SEED],
        bump = reward_pool.bump,
        has_one = authority @ SwipeError::Unauthorized,
        has_one = mint @ SwipeError::CardSupplyMismatch,
    )]
    pub reward_pool: Account<'info, RewardPool>,

    #[account(
        mut,
        seeds = [REWARD_VAULT_SEED],
        bump = reward_pool.vault_bump,
    )]
    pub reward_vault: InterfaceAccount<'info, TokenAccount>,

    pub mint: InterfaceAccount<'info, Mint>,

    #[account(
        mut,
        constraint = user_token_account.mint == reward_pool.mint,
        constraint = user_token_account.owner == user.key(),
    )]
    pub user_token_account: InterfaceAccount<'info, TokenAccount>,

    /// Recipient of the reward.
    pub user: SystemAccount<'info>,

    /// Backend keeper that attests set completion by co-signing.
    pub authority: Signer<'info>,

    pub token_program: Interface<'info, TokenInterface>,
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

    let vault_balance = ctx.accounts.reward_vault.amount;
    require!(vault_balance > 0, SwipeError::InsufficientRewardPool);

    let amount: u64 = (vault_balance as u128)
        .checked_mul(distribution_bps as u128)
        .ok_or(SwipeError::Overflow)?
        .checked_div(BPS_DENOMINATOR as u128)
        .ok_or(SwipeError::Overflow)? as u64;
    require!(amount > 0, SwipeError::InsufficientRewardPool);

    // Transfer from reward vault (authority = reward_pool PDA) → user.
    let signer: &[&[u8]] = &[REWARD_POOL_SEED, &[ctx.accounts.reward_pool.bump]];

    transfer_checked(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            TransferChecked {
                from: ctx.accounts.reward_vault.to_account_info(),
                mint: ctx.accounts.mint.to_account_info(),
                to: ctx.accounts.user_token_account.to_account_info(),
                authority: ctx.accounts.reward_pool.to_account_info(),
            },
            &[signer],
        ),
        amount,
        ctx.accounts.mint.decimals,
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
