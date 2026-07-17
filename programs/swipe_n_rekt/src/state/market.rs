use anchor_lang::prelude::*;

/// Comparison operator, mirrors TxOracle `Comparison`.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub enum Comparison {
    GreaterThan,
    LessThan,
    EqualTo,
}

impl Comparison {
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(Comparison::GreaterThan),
            1 => Some(Comparison::LessThan),
            2 => Some(Comparison::EqualTo),
            _ => None,
        }
    }
    pub fn to_u8(self) -> u8 {
        match self {
            Comparison::GreaterThan => 0,
            Comparison::LessThan => 1,
            Comparison::EqualTo => 2,
        }
    }
}

#[derive(AnchorSerialize, AnchorDeserialize, InitSpace, Clone, Copy, PartialEq, Eq, Debug)]
pub enum MarketStatus {
    Open,
    Closed,
    Settled,
}

/// One swipe card == one market. Value is escrowed in native SOL (lamports).
#[account]
#[derive(InitSpace)]
pub struct Market {
    /// Fixture this market resolves against (TxODDS fixture id).
    pub fixture_id: i64,
    /// Stat type (goals / corners / cards ...). Maps to `ScoreStat.key`.
    pub stat_key: u32,
    /// Period (full game / 1st half / 2nd half ...). Maps to `ScoreStat.period`.
    pub period: i32,
    /// TraderPredicate threshold.
    pub threshold: i32,
    /// TraderPredicate comparison (see `Comparison`).
    pub comparison: u8,
    pub window_start: i64,
    pub window_end: i64,
    /// Total lamports staked on YES.
    pub total_yes: u64,
    /// Total lamports staked on NO.
    pub total_no: u64,
    pub status: MarketStatus,
    /// Winning side once settled: 0 = NO, 1 = YES.
    pub winning_side: Option<u8>,
    /// Authority allowed to settle this market (backend keeper).
    pub authority: Pubkey,
    pub bump: u8,
    pub vault_bump: u8,
}

impl Market {
    /// Total pot across both sides.
    pub fn total_pot(&self) -> u64 {
        self.total_yes.saturating_add(self.total_no)
    }
}
