use anchor_lang::prelude::*;

/// Global reward pool that accrues protocol fees and pays out set-completion
/// rewards. Value is held in native SOL (lamports) in the reward vault PDA.
#[account]
#[derive(InitSpace)]
pub struct RewardPool {
    /// Lifetime fees collected (accounting only; live balance lives in the vault).
    pub total_collected: u64,
    /// Total ever distributed out of the pool.
    pub total_distributed: u64,
    pub current_period: u32,
    /// Authority allowed to authorize set-reward distributions (backend).
    pub authority: Pubkey,
    pub bump: u8,
    pub vault_bump: u8,
}
