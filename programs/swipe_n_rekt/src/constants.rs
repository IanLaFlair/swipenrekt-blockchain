use anchor_lang::prelude::*;

/// TxLINE TxOracle program id (verified from `txodds/tx-on-chain`).
/// On devnet this account is cloned from mainnet (see Anchor.toml `[[test.validator.clone]]`).
pub const TXORACLE_PROGRAM_ID: Pubkey = pubkey!("9ExbZjAapQww1vfcisDmrngPinHTEfpjYRWMunJgcKaA");

// ---- PDA seed prefixes -------------------------------------------------------
#[constant]
pub const MARKET_SEED: &[u8] = b"market";
#[constant]
pub const POSITION_SEED: &[u8] = b"position";
#[constant]
pub const VAULT_SEED: &[u8] = b"vault";
#[constant]
pub const REWARD_POOL_SEED: &[u8] = b"reward_pool";
#[constant]
pub const REWARD_VAULT_SEED: &[u8] = b"reward_vault";
#[constant]
pub const CARD_SUPPLY_SEED: &[u8] = b"card_supply";

// ---- Economics ---------------------------------------------------------------
/// Protocol fee taken on every bet, in basis points (2% = 200 bps).
#[constant]
pub const FEE_BPS: u64 = 200;
#[constant]
pub const BPS_DENOMINATOR: u64 = 10_000;

// ---- Bet sides ---------------------------------------------------------------
pub const SIDE_NO: u8 = 0;
pub const SIDE_YES: u8 = 1;

// ---- Card supply caps by rarity ---------------------------------------------
// Common(0)/Uncommon(1) = unlimited, Rare(2)=5_000, Epic(3)=1_000, Legendary(4)=300
pub const RARITY_COMMON: u8 = 0;
pub const RARITY_UNCOMMON: u8 = 1;
pub const RARITY_RARE: u8 = 2;
pub const RARITY_EPIC: u8 = 3;
pub const RARITY_LEGENDARY: u8 = 4;

/// Returns the mint cap for a rarity tier. `u32::MAX` == effectively unlimited.
pub fn cap_for_rarity(rarity: u8) -> Option<u32> {
    match rarity {
        RARITY_COMMON | RARITY_UNCOMMON => Some(u32::MAX),
        RARITY_RARE => Some(5_000),
        RARITY_EPIC => Some(1_000),
        RARITY_LEGENDARY => Some(300),
        _ => None,
    }
}

/// Valid catalog id range (player index from the CSV, 1..=1248).
pub const CATALOG_ID_MIN: u32 = 1;
pub const CATALOG_ID_MAX: u32 = 1248;
