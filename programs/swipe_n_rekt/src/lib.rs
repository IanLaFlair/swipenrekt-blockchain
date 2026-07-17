#![allow(unexpected_cfgs)]
//! Swipe n Rekt — on-chain program.
//!
//! On-chain = MONEY & PROOF. Everything else lives in the NestJS backend.
//!  - USDC escrow per market (swipe card)
//!  - Trustless settlement via CPI into TxLINE TxOracle `validate_stat`
//!  - Proportional payouts to winners
//!  - Reward pool (2% fee collection + % distributions)
//!  - Per-player cNFT card supply caps

use anchor_lang::prelude::*;

pub mod constants;
pub mod errors;
pub mod events;
pub mod instructions;
pub mod state;
pub mod txoracle;

use instructions::*;
use txoracle::types::ValidateStatArgs;

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

#[program]
pub mod swipe_n_rekt {
    use super::*;

    /// One-time: create the global reward pool + its vault.
    pub fn initialize_reward_pool(ctx: Context<InitializeRewardPool>) -> Result<()> {
        instructions::initialize_reward_pool::handler(ctx)
    }

    /// Create a new market (one swipe card). Called by the backend authority.
    #[allow(clippy::too_many_arguments)]
    pub fn initialize_market(
        ctx: Context<InitializeMarket>,
        fixture_id: i64,
        stat_key: u32,
        period: i32,
        threshold: i32,
        comparison: u8,
        window_start: i64,
        window_end: i64,
    ) -> Result<()> {
        instructions::initialize_market::handler(
            ctx,
            fixture_id,
            stat_key,
            period,
            threshold,
            comparison,
            window_start,
            window_end,
        )
    }

    /// Place a bet. Fee → reward pool, net stake → market vault.
    pub fn place_bet(ctx: Context<PlaceBet>, side: u8, amount: u64, price: u32) -> Result<()> {
        instructions::place_bet::handler(ctx, side, amount, price)
    }

    /// Settle a market trustlessly via CPI into TxOracle `validate_stat`.
    pub fn settle_market(
        ctx: Context<SettleMarket>,
        claimed_winning_side: u8,
        oracle_args: ValidateStatArgs,
    ) -> Result<()> {
        instructions::settle_market::handler(ctx, claimed_winning_side, oracle_args)
    }

    /// Settle a market by authority decree (week-1 integration / EqualTo-NO fallback).
    pub fn settle_market_mock(ctx: Context<SettleMarketMock>, winning_side: u8) -> Result<()> {
        instructions::settle_market::handler_mock(ctx, winning_side)
    }

    /// Winner claims their proportional share of the pot.
    pub fn claim_payout(ctx: Context<ClaimPayout>) -> Result<()> {
        instructions::claim_payout::handler(ctx)
    }

    /// Mint a player card (enforces the rarity supply cap).
    pub fn mint_card(ctx: Context<MintCard>, catalog_id: u32, rarity: u8) -> Result<()> {
        instructions::mint_card::handler(ctx, catalog_id, rarity)
    }

    /// Distribute a % of the reward pool to a user who completed a country set.
    pub fn claim_set_reward(
        ctx: Context<ClaimSetReward>,
        country: u8,
        period: u32,
        distribution_bps: u16,
    ) -> Result<()> {
        instructions::claim_set_reward::handler(ctx, country, period, distribution_bps)
    }
}
