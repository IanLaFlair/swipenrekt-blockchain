use anchor_lang::prelude::*;

/// Per-player-card supply counter. Enforces the rarity cap.
#[account]
#[derive(InitSpace)]
pub struct CardSupply {
    /// Player index from the CSV (1..=1248).
    pub catalog_id: u32,
    /// 0=common .. 4=legendary.
    pub rarity: u8,
    /// How many have been minted so far.
    pub minted: u32,
    /// Hard cap (u32::MAX == unlimited).
    pub cap: u32,
    pub bump: u8,
}
