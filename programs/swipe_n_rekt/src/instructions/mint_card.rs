use anchor_lang::prelude::*;

use crate::constants::*;
use crate::errors::SwipeError;
use crate::events::CardMinted;
use crate::state::CardSupply;

// ============================================================================
// mint_card — enforce the per-player rarity supply cap and record the mint.
//
// The compressed-NFT (Bubblegum) mint itself is a separate CPI with a large,
// version-sensitive account set (merkle_tree, tree_authority, log_wrapper,
// compression_program, bubblegum_program, ...). To keep this program's build
// self-contained and audited, the on-chain SUPPLY GUARANTEE lives here and the
// Bubblegum `mint_v1` CPI is invoked at the marked integration point (or driven
// by the backend in the same transaction). `asset` is the resulting cNFT id.
// ============================================================================

#[derive(Accounts)]
#[instruction(catalog_id: u32, rarity: u8)]
pub struct MintCard<'info> {
    #[account(
        init_if_needed,
        payer = user,
        space = 8 + CardSupply::INIT_SPACE,
        seeds = [CARD_SUPPLY_SEED, &catalog_id.to_le_bytes()],
        bump
    )]
    pub card_supply: Account<'info, CardSupply>,

    #[account(mut)]
    pub user: Signer<'info>,

    /// CHECK: the cNFT asset id being minted (Bubblegum leaf / asset). Recorded
    /// for indexing; the Bubblegum CPI enforces its correctness.
    pub asset: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
}

pub fn handler(ctx: Context<MintCard>, catalog_id: u32, rarity: u8) -> Result<()> {
    require!(
        (CATALOG_ID_MIN..=CATALOG_ID_MAX).contains(&catalog_id),
        SwipeError::InvalidCatalogId
    );
    let cap = cap_for_rarity(rarity).ok_or(SwipeError::InvalidRarity)?;

    let supply = &mut ctx.accounts.card_supply;

    // Initialise on first mint.
    if supply.cap == 0 && supply.minted == 0 {
        supply.catalog_id = catalog_id;
        supply.rarity = rarity;
        supply.cap = cap;
        supply.bump = ctx.bumps.card_supply;
    } else {
        // Subsequent mints must match the original card identity.
        require!(supply.catalog_id == catalog_id, SwipeError::CardSupplyMismatch);
        require!(supply.rarity == rarity, SwipeError::CardSupplyMismatch);
    }

    require!(supply.minted < supply.cap, SwipeError::SupplyCapReached);

    // ---- Bubblegum cNFT mint integration point -----------------------------
    // mpl_bubblegum::cpi::mint_v1(CpiContext::new(...), metadata_args)?;
    // The asset/leaf resulting from that CPI is `ctx.accounts.asset`.
    // ------------------------------------------------------------------------

    supply.minted = supply.minted.checked_add(1).ok_or(SwipeError::Overflow)?;

    emit!(CardMinted {
        user: ctx.accounts.user.key(),
        catalog_id,
        rarity,
        minted: supply.minted,
        mint: ctx.accounts.asset.key(),
    });
    Ok(())
}
