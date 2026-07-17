use anchor_lang::prelude::*;

use crate::constants::*;
use crate::errors::SwipeError;
use crate::events::MarketSettled;
use crate::state::{Comparison, Market, MarketStatus};
use crate::txoracle::cpi::validate_stat;
use crate::txoracle::types::{OracleComparison, TraderPredicate, ValidateStatArgs};

// ============================================================================
// settle_market — trustless settlement via CPI into TxOracle `validate_stat`.
// ============================================================================
//
// The keeper (backend) claims a `winning_side` and supplies the Merkle proofs it
// fetched from the TxLINE API. The CONTRACT (not the caller) reconstructs the
// predicate that must hold for that claim to be true, then CPIs `validate_stat`.
// If the oracle proof supports the predicate the CPI returns Ok and we record the
// claimed side; if it doesn't, the CPI errors and the whole tx reverts (nothing
// is settled), so a keeper cannot lie.
//
// Predicate reconstruction (integer stats):
//   YES claim:  the market's own (threshold, comparison).
//   NO  claim:  the logical negation, expressed as a single comparison:
//       stat >  T   ->  NO: stat <  T+1   (i.e. stat <= T)
//       stat <  T   ->  NO: stat >  T-1   (i.e. stat >= T)
//       stat == T   ->  not expressible as one comparison; use settle_market_mock
//                       or validate_stat_v2. Rejected here.

#[derive(Accounts)]
pub struct SettleMarket<'info> {
    #[account(
        mut,
        seeds = [
            MARKET_SEED,
            &market.fixture_id.to_le_bytes(),
            &market.stat_key.to_le_bytes(),
            &market.window_start.to_le_bytes(),
        ],
        bump = market.bump,
        has_one = authority @ SwipeError::Unauthorized,
    )]
    pub market: Account<'info, Market>,

    /// CHECK: TxOracle's `daily_scores_merkle_roots` account. Ownership/shape is
    /// validated by the TxOracle program during the CPI.
    pub daily_scores_merkle_roots: UncheckedAccount<'info>,

    /// CHECK: must be the TxOracle program; enforced by address constraint.
    #[account(address = TXORACLE_PROGRAM_ID @ SwipeError::InvalidOracleProgram)]
    pub txoracle_program: UncheckedAccount<'info>,

    pub authority: Signer<'info>,
}

pub fn handler(
    ctx: Context<SettleMarket>,
    claimed_winning_side: u8,
    mut oracle_args: ValidateStatArgs,
) -> Result<()> {
    require!(
        claimed_winning_side == SIDE_NO || claimed_winning_side == SIDE_YES,
        SwipeError::InvalidSide
    );

    let market = &ctx.accounts.market;
    require!(
        market.status != MarketStatus::Settled,
        SwipeError::MarketAlreadySettled
    );

    // The proven stat must be the exact stat/period this market is about.
    require!(
        oracle_args.stat_a.stat_to_prove.key == market.stat_key
            && oracle_args.stat_a.stat_to_prove.period == market.period,
        SwipeError::OracleValidationFailed
    );

    // Reconstruct the predicate the contract will actually verify.
    let market_cmp = Comparison::from_u8(market.comparison).ok_or(SwipeError::InvalidComparison)?;
    let predicate = build_predicate(market_cmp, market.threshold, claimed_winning_side)?;
    oracle_args.predicate = predicate;

    // CPI — reverts the tx if the proof does not support the predicate.
    validate_stat(
        &ctx.accounts.txoracle_program.to_account_info(),
        &ctx.accounts.daily_scores_merkle_roots.to_account_info(),
        &oracle_args,
    )?;

    let market = &mut ctx.accounts.market;
    market.status = MarketStatus::Settled;
    market.winning_side = Some(claimed_winning_side);

    emit!(MarketSettled {
        market: market.key(),
        winning_side: claimed_winning_side,
    });
    Ok(())
}

/// Build the TraderPredicate to verify for a given claimed side (integer negation).
fn build_predicate(
    market_cmp: Comparison,
    threshold: i32,
    claimed_side: u8,
) -> Result<TraderPredicate> {
    // For YES: prove the market condition directly.
    if claimed_side == SIDE_YES {
        return Ok(TraderPredicate {
            threshold,
            comparison: to_oracle_cmp(market_cmp),
        });
    }

    // For NO: prove the negation.
    match market_cmp {
        Comparison::GreaterThan => Ok(TraderPredicate {
            // not (stat > T)  <=>  stat < T+1
            threshold: threshold.checked_add(1).ok_or(SwipeError::Overflow)?,
            comparison: OracleComparison::LessThan,
        }),
        Comparison::LessThan => Ok(TraderPredicate {
            // not (stat < T)  <=>  stat > T-1
            threshold: threshold.checked_sub(1).ok_or(SwipeError::Overflow)?,
            comparison: OracleComparison::GreaterThan,
        }),
        // not (stat == T) can't be one comparison -> keeper/v2 required.
        Comparison::EqualTo => Err(SwipeError::OracleValidationFailed.into()),
    }
}

fn to_oracle_cmp(c: Comparison) -> OracleComparison {
    match c {
        Comparison::GreaterThan => OracleComparison::GreaterThan,
        Comparison::LessThan => OracleComparison::LessThan,
        Comparison::EqualTo => OracleComparison::EqualTo,
    }
}

// ============================================================================
// settle_market_mock — authority-signed settlement, NO oracle. For week-1
// backend integration and for markets the oracle can't express (EqualTo NO).
// ============================================================================

#[derive(Accounts)]
pub struct SettleMarketMock<'info> {
    #[account(
        mut,
        seeds = [
            MARKET_SEED,
            &market.fixture_id.to_le_bytes(),
            &market.stat_key.to_le_bytes(),
            &market.window_start.to_le_bytes(),
        ],
        bump = market.bump,
        has_one = authority @ SwipeError::Unauthorized,
    )]
    pub market: Account<'info, Market>,

    pub authority: Signer<'info>,
}

pub fn handler_mock(ctx: Context<SettleMarketMock>, winning_side: u8) -> Result<()> {
    require!(
        winning_side == SIDE_NO || winning_side == SIDE_YES,
        SwipeError::InvalidSide
    );
    let market = &mut ctx.accounts.market;
    require!(
        market.status != MarketStatus::Settled,
        SwipeError::MarketAlreadySettled
    );
    market.status = MarketStatus::Settled;
    market.winning_side = Some(winning_side);

    emit!(MarketSettled {
        market: market.key(),
        winning_side,
    });
    Ok(())
}
