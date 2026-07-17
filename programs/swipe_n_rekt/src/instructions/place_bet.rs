use anchor_lang::prelude::*;
use anchor_spl::token_interface::{
    transfer_checked, Mint, TokenAccount, TokenInterface, TransferChecked,
};

use crate::constants::*;
use crate::errors::SwipeError;
use crate::events::BetPlaced;
use crate::state::{Market, MarketStatus, Position, RewardPool};

#[derive(Accounts)]
pub struct PlaceBet<'info> {
    #[account(
        mut,
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
        init_if_needed,
        payer = user,
        space = 8 + Position::INIT_SPACE,
        seeds = [POSITION_SEED, market.key().as_ref(), user.key().as_ref()],
        bump
    )]
    pub position: Account<'info, Position>,

    #[account(
        mut,
        seeds = [VAULT_SEED, market.key().as_ref()],
        bump = market.vault_bump,
    )]
    pub vault: InterfaceAccount<'info, TokenAccount>,

    #[account(
        mut,
        seeds = [REWARD_POOL_SEED],
        bump = reward_pool.bump,
    )]
    pub reward_pool: Account<'info, RewardPool>,

    #[account(
        mut,
        seeds = [REWARD_VAULT_SEED],
        bump = reward_pool.vault_bump,
    )]
    pub reward_vault: InterfaceAccount<'info, TokenAccount>,

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
    pub system_program: Program<'info, System>,
}

pub fn handler(ctx: Context<PlaceBet>, side: u8, amount: u64, price: u32) -> Result<()> {
    require!(amount > 0, SwipeError::ZeroAmount);
    require!(side == SIDE_NO || side == SIDE_YES, SwipeError::InvalidSide);

    let now = Clock::get()?.unix_timestamp;
    {
        let market = &ctx.accounts.market;
        require!(market.status == MarketStatus::Open, SwipeError::MarketNotOpen);
        require!(now >= market.window_start, SwipeError::WindowNotOpen);
        require!(now < market.window_end, SwipeError::WindowClosed);
    }

    // 1) fee (2%) → reward vault
    let fee = amount
        .checked_mul(FEE_BPS)
        .ok_or(SwipeError::Overflow)?
        .checked_div(BPS_DENOMINATOR)
        .ok_or(SwipeError::Overflow)?;
    let net = amount.checked_sub(fee).ok_or(SwipeError::Overflow)?;
    let decimals = ctx.accounts.mint.decimals;

    if fee > 0 {
        transfer_checked(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                TransferChecked {
                    from: ctx.accounts.user_token_account.to_account_info(),
                    mint: ctx.accounts.mint.to_account_info(),
                    to: ctx.accounts.reward_vault.to_account_info(),
                    authority: ctx.accounts.user.to_account_info(),
                },
            ),
            fee,
            decimals,
        )?;
    }

    // 2) net stake → market vault
    transfer_checked(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            TransferChecked {
                from: ctx.accounts.user_token_account.to_account_info(),
                mint: ctx.accounts.mint.to_account_info(),
                to: ctx.accounts.vault.to_account_info(),
                authority: ctx.accounts.user.to_account_info(),
            },
        ),
        net,
        decimals,
    )?;

    // 3) reward pool accounting
    let reward_pool = &mut ctx.accounts.reward_pool;
    reward_pool.total_collected = reward_pool
        .total_collected
        .checked_add(fee)
        .ok_or(SwipeError::Overflow)?;

    // 4) market side totals
    let market = &mut ctx.accounts.market;
    if side == SIDE_YES {
        market.total_yes = market.total_yes.checked_add(net).ok_or(SwipeError::Overflow)?;
    } else {
        market.total_no = market.total_no.checked_add(net).ok_or(SwipeError::Overflow)?;
    }

    // 5) position (accumulate; disallow switching sides)
    let position = &mut ctx.accounts.position;
    if position.amount == 0 {
        position.user = ctx.accounts.user.key();
        position.market = market.key();
        position.side = side;
        position.price = price;
        position.claimed = false;
        position.bump = ctx.bumps.position;
    } else {
        require!(position.side == side, SwipeError::InvalidSide);
    }
    position.amount = position.amount.checked_add(net).ok_or(SwipeError::Overflow)?;

    emit!(BetPlaced {
        market: market.key(),
        user: ctx.accounts.user.key(),
        side,
        amount: net,
        fee,
        total_yes: market.total_yes,
        total_no: market.total_no,
    });
    Ok(())
}
