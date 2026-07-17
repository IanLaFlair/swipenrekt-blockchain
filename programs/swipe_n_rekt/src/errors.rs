use anchor_lang::prelude::*;

#[error_code]
pub enum SwipeError {
    #[msg("Market is not open for betting")]
    MarketNotOpen,
    #[msg("Betting window has closed")]
    WindowClosed,
    #[msg("Betting window has not opened yet")]
    WindowNotOpen,
    #[msg("Invalid betting window (start must be < end)")]
    InvalidWindow,
    #[msg("Bet amount must be greater than zero")]
    ZeroAmount,
    #[msg("Invalid side (must be 0 = NO or 1 = YES)")]
    InvalidSide,
    #[msg("Invalid comparison operator")]
    InvalidComparison,
    #[msg("Market is not yet settled")]
    MarketNotSettled,
    #[msg("Market is already settled")]
    MarketAlreadySettled,
    #[msg("Position is on the losing side")]
    LosingSide,
    #[msg("Payout already claimed for this position")]
    AlreadyClaimed,
    #[msg("No winning stake to distribute against")]
    NoWinningStake,
    #[msg("Numeric overflow")]
    Overflow,
    #[msg("Provided TxOracle program does not match the expected program id")]
    InvalidOracleProgram,
    #[msg("Oracle predicate did not evaluate as expected")]
    OracleValidationFailed,
    #[msg("Card supply cap reached for this player card")]
    SupplyCapReached,
    #[msg("Invalid rarity tier")]
    InvalidRarity,
    #[msg("Catalog id out of range (1..=1248)")]
    InvalidCatalogId,
    #[msg("Card supply metadata mismatch")]
    CardSupplyMismatch,
    #[msg("Reward pool has insufficient funds for this distribution")]
    InsufficientRewardPool,
    #[msg("Distribution basis points must be between 1 and 10000")]
    InvalidDistributionBps,
    #[msg("Unauthorized: signer is not the program authority")]
    Unauthorized,
}
