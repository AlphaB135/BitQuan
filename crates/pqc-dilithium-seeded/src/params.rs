#[cfg(all(feature = "mode2", not(feature = "mode5")))]
mod mode_2;
#[cfg(feature = "mode5")]
mod mode_5;
// Only use mode3 as EXPLICIT opt-in, never as fallback
#[cfg(all(feature = "mode3", not(feature = "mode5")))]
mod mode_3;

#[cfg(all(not(feature = "mode5"), feature = "mode2"))]
use mode_2 as active_mode;
#[cfg(all(
  feature = "mode3",
  not(feature = "mode5"),
  not(feature = "mode2")
))]
use mode_3 as active_mode;
#[cfg(feature = "mode5")]
use mode_5 as active_mode;

use active_mode as mode_params;
pub use active_mode::*;

pub const SEEDBYTES: usize = 32;
pub const CRHBYTES: usize = 64;
pub const N: usize = 256;
pub const Q: usize = 8380417;
pub const D: usize = 13;
pub const ROOT_OF_UNITY: usize = 1753;

pub const POLYT1_PACKEDBYTES: usize = 320;
pub const POLYT0_PACKEDBYTES: usize = 416;
pub const POLYVECH_PACKEDBYTES: usize = mode_params::OMEGA + mode_params::K;

pub const POLYZ_PACKEDBYTES: usize =
  if cfg!(all(feature = "mode2", not(feature = "mode5"))) { 576 } else { 640 };
pub const POLYW1_PACKEDBYTES: usize =
  if cfg!(all(feature = "mode2", not(feature = "mode5"))) { 192 } else { 128 };

pub const POLYETA_PACKEDBYTES: usize =
  if cfg!(not(any(feature = "mode2", feature = "mode5"))) {
    128
  } else {
    96
  };

// Concise types to avoid cast cluttering
pub const Q_I32: i32 = Q as i32;
pub const N_U32: u32 = N as u32;
pub const L_U16: u16 = mode_params::L as u16;
pub const BETA_I32: i32 = mode_params::BETA as i32;
pub const GAMMA1_I32: i32 = mode_params::GAMMA1 as i32;
pub const GAMMA2_I32: i32 = mode_params::GAMMA2 as i32;
pub const OMEGA_U8: u8 = mode_params::OMEGA as u8;
pub const ETA_I32: i32 = mode_params::ETA as i32;
pub const GAMMA1_SUB_BETA: i32 =
  (mode_params::GAMMA1 - mode_params::BETA) as i32;

pub const PUBLICKEYBYTES: usize =
  SEEDBYTES + mode_params::K * POLYT1_PACKEDBYTES;
pub const SECRETKEYBYTES: usize = 3 * SEEDBYTES
  + mode_params::L * POLYETA_PACKEDBYTES
  + mode_params::K * POLYETA_PACKEDBYTES
  + mode_params::K * POLYT0_PACKEDBYTES;
pub const SIGNBYTES: usize =
  SEEDBYTES + mode_params::L * POLYZ_PACKEDBYTES + POLYVECH_PACKEDBYTES;

pub const RANDOMIZED_SIGNING: bool = cfg!(feature = "random_signing");
