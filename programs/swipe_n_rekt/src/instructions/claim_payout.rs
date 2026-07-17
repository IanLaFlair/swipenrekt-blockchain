use anchor_lang::prelude::*;
use anchor_lang::system_program::{transfer, Transfer};

use crate::constants::*;
use crate::errors::SwipeError;
use crate::events::PayoutClaimed;
use crate::state::{Market, MarketStatus, Position};

#[derive(Accounts)]
pub struct ClaimPayout<'info> {
    #[account(
        seeds = [
            MARKET_SEED,
            &market.fixture_id.to_le_bytes(),
            &market.stat_key.to_le_bytes(),
            &market.window_start.to_le_bytes(),
        ],
        bump = market.bump,
    )]
    pub market: Account<'info, Market>,

    #[account(
        mut,
        seeds = [POSITION_SEED, market.key().as_ref(), user.key().as_ref()],
        bump = position.bump,
        has_one = user @ SwipeError::Unauthorized,
        has_one = market @ SwipeError::Unauthorized,
    )]
    pub position: Account<'info, Position>,

    /// Native-SOL escrow vault (system-owned PDA holding lamports only).
    #[account(
        mut,
        seeds = [VAULT_SEED, market.key().as_ref()],
        bump = market.vault_bump,
    )]
    pub vault: SystemAccount<'info>,

    #[account(mut)]
    pub user: Signer<'info>,

    pub system_program: Program<'info, System>,
}

pub fn handler(ctx: Context<ClaimPayout>) -> Result<()> {
    let market = &ctx.accounts.market;
    require!(
        market.status == MarketStatus::Settled,
        SwipeError::MarketNotSettled
    );
    let winning_side = market.winning_side.ok_or(SwipeError::MarketNotSettled)?;

    let position = &ctx.accounts.position;
    require!(!position.claimed, SwipeError::AlreadyClaimed);
    require!(position.side == winning_side, SwipeError::LosingSide);

    // payout = position.amount / total_winning * total_pot
    let total_winning = if winning_side == SIDE_YES {
        market.total_yes
    } else {
        market.total_no
    };
    require!(total_winning > 0, SwipeError::NoWinningStake);

    let total_pot = market.total_pot();
    // u128 math to avoid overflow on the intermediate product.
    let payout: u64 = (position.amount as u128)
        .checked_mul(total_pot as u128)
        .ok_or(SwipeError::Overflow)?
        .checked_div(total_winning as u128)
        .ok_or(SwipeError::Overflow)? as u64;

    // Move lamports from the vault PDA → user. The vault is a system-owned PDA
    // with no data, so it can sign a System Program transfer with its own seeds.
    let market_key = market.key();
    let vault_signer: &[&[u8]] = &[VAULT_SEED, market_key.as_ref(), &[market.vault_bump]];

    transfer(
        CpiContext::new_with_signer(
            ctx.accounts.system_program.to_account_info(),
            Transfer {
                from: ctx.accounts.vault.to_account_info(),
                to: ctx.accounts.user.to_account_info(),
            },
            &[vault_signer],
        ),
        payout,
    )?;

    let position = &mut ctx.accounts.position;
    position.claimed = true;

    emit!(PayoutClaimed {
        user: ctx.accounts.user.key(),
        market: market_key,
        amount: payout,
    });
    Ok(())
}
