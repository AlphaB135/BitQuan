#![deny(missing_docs)]

//! Deterministic random bit generators and derivation helpers.

mod hkdf;
mod rng;

pub use rng::{RandomSource, RngError, RngService};
