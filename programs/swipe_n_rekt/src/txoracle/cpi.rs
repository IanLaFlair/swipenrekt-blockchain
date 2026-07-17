use anchor_lang::prelude::*;
use anchor_lang::solana_program::instruction::{AccountMeta, Instruction};
use anchor_lang::solana_program::program::invoke;

use super::types::ValidateStatArgs;

/// Anchor instruction discriminator for `validate_stat`
/// = sha256("global:validate_stat")[..8].
pub const VALIDATE_STAT_DISCRIMINATOR: [u8; 8] = [107, 197, 232, 90, 191, 136, 105, 185];

/// CPI into TxOracle `validate_stat`.
///
/// TxOracle validates the supplied stat against the on-chain daily Merkle roots
/// and the predicate. If the predicate holds it returns `Ok`; otherwise the CPI
/// returns an `Err`, which is exactly the signal we use to decide the winning
/// side in `settle_market`.
///
/// Accounts (per plan / IDL):
///   0. `daily_scores_merkle_roots` — readonly, the oracle's roots account.
///
/// Returns `Ok(())` when the predicate evaluated TRUE, propagates the oracle's
/// error otherwise.
pub fn validate_stat<'info>(
    txoracle_program: &AccountInfo<'info>,
    daily_scores_merkle_roots: &AccountInfo<'info>,
    args: &ValidateStatArgs,
) -> Result<()> {
    let mut data = Vec::with_capacity(256);
    data.extend_from_slice(&VALIDATE_STAT_DISCRIMINATOR);
    args.serialize(&mut data)?;

    let ix = Instruction {
        program_id: *txoracle_program.key,
        accounts: vec![AccountMeta::new_readonly(
            *daily_scores_merkle_roots.key,
            false,
        )],
        data,
    };

    invoke(
        &ix,
        &[
            daily_scores_merkle_roots.clone(),
            txoracle_program.clone(),
        ],
    )?;

    Ok(())
}
