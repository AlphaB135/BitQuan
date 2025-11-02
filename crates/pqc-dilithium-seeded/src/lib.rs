#![allow(clippy::all)]

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
