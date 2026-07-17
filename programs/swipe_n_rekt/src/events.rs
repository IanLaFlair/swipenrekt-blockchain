use anchor_lang::prelude::*;

#[event]
pub struct MarketInitialized {
    pub market: Pubkey,
    pub fixture_id: i64,
    pub stat_key: u32,
    pub period: i32,
    pub threshold: i32,
    pub comparison: u8,
    pub window_start: i64,
    pub window_end: i64,
}

#[event]
pub struct BetPlaced {
    pub market: Pubkey,
    pub user: Pubkey,
    pub side: u8,
    pub amount: u64,
    pub fee: u64,
    pub total_yes: u64,
    pub total_no: u64,
}

#[event]
pub struct MarketSettled {
    pub market: Pubkey,
    pub winning_side: u8,
}

#[event]
pub struct PayoutClaimed {
    pub user: Pubkey,
    pub market: Pubkey,
    pub amount: u64,
}

#[event]
pub struct CardMinted {
    pub user: Pubkey,
    pub catalog_id: u32,
    pub rarity: u8,
    pub minted: u32,
    pub mint: Pubkey,
}

#[event]
pub struct SetRewardClaimed {
    pub user: Pubkey,
    pub country: u8,
    pub period: u32,
    pub amount: u64,
}
