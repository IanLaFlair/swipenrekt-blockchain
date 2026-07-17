use anchor_lang::prelude::*;
use anchor_spl::token_interface::{
    transfer_checked, Mint, TokenAccount, TokenInterface, TransferChecked,
};

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

    #[account(
        mut,
        seeds = [VAULT_SEED, market.key().as_ref()],
        bump = market.vault_bump,
    )]
    pub vault: InterfaceAccount<'info, TokenAccount>,

    #[account(address = market.mint)]
    pub mint: InterfaceAccount<'info, Mint>,

    #[account(
        mut,
        constraint = user_token_account.mint == market.mint,
        constraint = user_token_account.owner == user.key(),
    )]
    pub user_token_account: InterfaceAccount<'info, TokenAccount>,

    #[account(mut)]
    pub user: Signer<'info>,

    pub token_program: Interface<'info, TokenInterface>,
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

    // Transfer from vault → user. The vault's token::authority is the MARKET
    // PDA, so we sign the CPI with the market's seeds.
    let market_key = market.key();
    let fixture_le = market.fixture_id.to_le_bytes();
    let stat_le = market.stat_key.to_le_bytes();
    let win_le = market.window_start.to_le_bytes();
    let market_signer: &[&[u8]] = &[
        MARKET_SEED,
        &fixture_le,
        &stat_le,
        &win_le,
        &[market.bump],
    ];

    transfer_checked(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            TransferChecked {
                from: ctx.accounts.vault.to_account_info(),
                mint: ctx.accounts.mint.to_account_info(),
                to: ctx.accounts.user_token_account.to_account_info(),
                authority: ctx.accounts.market.to_account_info(),
            },
            &[market_signer],
        ),
        payout,
        ctx.accounts.mint.decimals,
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
