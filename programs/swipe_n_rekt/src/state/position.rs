use anchor_lang::prelude::*;

/// One user's stake in one market. Amounts accumulate if the user bets multiple
/// times on the same side; switching sides is rejected at the instruction level.
#[account]
#[derive(InitSpace)]
pub struct Position {
    pub user: Pubkey,
    pub market: Pubkey,
    /// 0 = NO, 1 = YES.
    pub side: u8,
    /// Net USDC staked (after fee) on `side`.
    pub amount: u64,
    /// Implied probability at entry, in basis points (informational / analytics).
    pub price: u32,
    pub claimed: bool,
    pub bump: u8,
}
