# BitQuan SDK (Rust)

Scope:
- Address encode/decode (Bech32m, HRP bq/tbq).
- PSBT(PQC) build/sign (per BQIP-0003) and transaction builder (per BQIP-0001/0004).
- FFI-safe signer contexts with zeroization and limited lifetime.

Layout proposal:
- crate: `bq-sdk` (not yet added to workspace; bootstrap here first).
- modules: address/, psbt/, tx/, signer/.

Next steps:
- [ ] Define public API surface (traits/types) draft.
- [ ] Create minimal crate skeleton with CI (after API sign-off).
- [ ] Conformance tests against node JSON-RPC.
