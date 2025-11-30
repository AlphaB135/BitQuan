#![forbid(unsafe_code)]
#![deny(missing_docs)]

//! RNG module exposed by the `bq-crypto` crate.

mod blake3_kdf;
mod hkdf;
mod rng_impl;

pub use blake3_kdf::blake3_expand;
pub use hkdf::hkdf_expand;
pub use rng_impl::{RandomSource, RngError, RngService};
