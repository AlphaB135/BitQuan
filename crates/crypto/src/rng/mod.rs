#![forbid(unsafe_code)]
#![deny(missing_docs)]

//! RNG module exposed by the `bq-crypto` crate.

mod hkdf;
mod rng;

pub use hkdf::hkdf_expand;
pub use rng::{RandomSource, RngError, RngService};
