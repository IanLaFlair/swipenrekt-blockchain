use anchor_lang::prelude::*;
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};

use crate::constants::*;
use crate::errors::SwipeError;
use crate::events::MarketInitialized;
use crate::state::{Comparison, Market, MarketStatus};

#[derive(Accounts)]
#[instruction(fixture_id: i64, stat_key: u32, period: i32, threshold: i32, comparison: u8, window_start: i64)]
pub struct InitializeMarket<'info> {
    #[account(
        init,
        payer = authority,
        space = 8 + Market::INIT_SPACE,
        // PDA seed formula agreed with backend (plan §8):
        // [b"market", fixture_id, stat_key, window_start]
        seeds = [
            MARKET_SEED,
            &fixture_id.to_le_bytes(),
            &stat_key.to_le_bytes(),
            &window_start.to_le_bytes(),
        ],
        bump
    )]
    pub market: Account<'info, Market>,

    /// Escrow vault for this market (PDA-owned token account).
    #[account(
        init,
        payer = authority,
        seeds = [VAULT_SEED, market.key().as_ref()],
        bump,
        token::mint = mint,
        token::authority = market,
    )]
    pub vault: InterfaceAccount<'info, TokenAccount>,

    /// USDC mint (devnet).
    pub mint: InterfaceAccount<'info, Mint>,

    #[account(mut)]
    pub authority: Signer<'info>,

    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
}

#[allow(clippy::too_many_arguments)]
pub fn handler(
    ctx: Context<InitializeMarket>,
    fixture_id: i64,
    stat_key: u32,
    period: i32,
    threshold: i32,
    comparison: u8,
    window_start: i64,
    window_end: i64,
) -> Result<()> {
    require!(window_start < window_end, SwipeError::InvalidWindow);
    require!(
        Comparison::from_u8(comparison).is_some(),
        SwipeError::InvalidComparison
    );

    let market = &mut ctx.accounts.market;
    market.fixture_id = fixture_id;
    market.stat_key = stat_key;
    market.period = period;
    market.threshold = threshold;
    market.comparison = comparison;
    market.window_start = window_start;
    market.window_end = window_end;
    market.total_yes = 0;
    market.total_no = 0;
    market.status = MarketStatus::Open;
    market.winning_side = None;
    market.authority = ctx.accounts.authority.key();
    market.mint = ctx.accounts.mint.key();
    market.bump = ctx.bumps.market;
    market.vault_bump = ctx.bumps.vault;

    emit!(MarketInitialized {
        market: market.key(),
        fixture_id,
        stat_key,
        period,
        threshold,
        comparison,
        window_start,
        window_end,
    });
    Ok(())
}
