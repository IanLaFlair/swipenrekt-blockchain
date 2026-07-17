//! Minimal, self-contained bindings for the TxLINE TxOracle program's
//! `validate_stat` instruction, reconstructed from the plan / `idl/txoracle.json`.
//!
//! We invoke it via a raw `invoke` (manual instruction data) so we do NOT need
//! the TxOracle crate as a build dependency. If/when you vendor the real
//! `txoracle` crate you can swap `cpi::validate_stat` for the generated CPI.
//!
//! The Anchor instruction discriminator is `sha256("global:validate_stat")[..8]`.

pub mod types;
pub mod cpi;

pub use types::*;
