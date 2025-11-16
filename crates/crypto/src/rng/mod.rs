#![forbid(unsafe_code)]
#![deny(missing_docs)]

//! RNG module exposed by the `bq-crypto` crate.

mod hkdf;
mod rng_impl;

pub use hkdf::hkdf_expand;
pub use rng_impl::{RandomSource, RngError, RngService};
