use pqc_dilithium_seeded::*;

#[test]
fn sign_then_verify_valid() {
  if std::env::var_os("BITQUAN_SKIP_PQC_TESTS").is_some() {
    return;
  }

  let msg = b"Hello";
  let keys = Keypair::generate();
  let signature = keys.sign(msg);
  assert!(verify(&signature, msg, &keys.public).is_ok())
}

#[test]
fn sign_then_verify_invalid() {
  if std::env::var_os("BITQUAN_SKIP_PQC_TESTS").is_some() {
    return;
  }

  let msg = b"Hello";
  let keys = Keypair::generate();
  let mut signature = keys.sign(msg);
  signature[..4].copy_from_slice(&[255u8; 4]);
  assert!(verify(&signature, msg, &keys.public).is_err())
}
