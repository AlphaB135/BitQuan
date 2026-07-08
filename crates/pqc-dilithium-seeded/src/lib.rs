// CLIPPY ALL JUSTIFICATION:
//
// This crate is a Rust port of the NIST CRYSTALS-Dilithium reference implementation.
// The code is deliberately written to closely match the reference C implementation
// for security auditing and verification purposes.
//
// Deviations from the reference (including Rust-idiomatic changes) could:
// 1. Introduce subtle security bugs during translation
// 2. Make security audits more difficult by diverging from verified code
// 3. Create challenges for cross-verification against the reference
//
// Examples of non-idiomatic patterns that match the reference:
// - Manual bit rotation (matches reference's ROTL/ROTR macros)
// - Specific operator ordering (matches reference's expressions)
// - Loop patterns that index arrays (matches reference's pointer arithmetic)
//
// Therefore, all clippy lints are allowed for this crate. The code prioritizes
// security and verifiability over Rust idiomatic patterns.

#![allow(clippy::style, clippy::complexity, clippy::perf, clippy::pedantic)]
#![allow(
  clippy::needless_range_loop,
  clippy::many_single_char_names,
  clippy::too_many_arguments,
  clippy::cast_possible_truncation,
  clippy::cast_sign_loss
)]

#[cfg(feature = "aes")]
mod aes256ctr;
mod api;
mod fips202;
mod ntt;
mod packing;
mod params;
mod poly;
mod polyvec;
mod randombytes;
mod reduce;
mod rounding;
mod sign;
mod symmetric;
// Export params (including PUBLICKEYBYTES, SECRETKEYBYTES, etc.)
pub use params::*;

// Export API (Keypair, etc.)
pub use api::*;

// Export sign functions for deterministic key generation
pub use sign::{
  crypto_sign_keypair, crypto_sign_signature, crypto_sign_verify,
};

#[cfg(feature = "wasm")]
mod wasm;
