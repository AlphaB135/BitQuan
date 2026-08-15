# Attack Report #003: RPC Authentication & RBAC Privilege Escalation

**Date**: 2026-08-15 10:57:00 UTC  
**Attack Type**: RPC & API / Authentication Bypass  
**Severity**: High  
**Status**: Blocked (Mitigated & Verified)  
**Target Component**: `crates/rpc/src/server.rs`, `crates/rpc/src/jwt/auth.rs`

---

## 1. Attack Objective & Vector Description

The objective is to access privileged RPC methods (`submittransaction`, `generatetoaddress`, `importprivkey`, `stop`) without providing valid credentials, by forging JWT tokens, or by using a long-lived Refresh Token in place of an Access Token.

### Attack Vectors Tested:
1. **Unauthenticated Request**: Invoking state-modifying endpoints without `Authorization` header.
2. **Forged Signature / Secret Tampering**: Signing a JWT token with an arbitrary or empty HMAC secret.
3. **Refresh Token Misuse**: Presenting a 7-day Refresh Token directly to the RPC gateway to bypass access token expiration.
4. **Role Escalation**: Using a `Readonly` role token to invoke `submittransaction` or `generate`.

---

## 2. Steps to Reproduce (PoC)

```bash
RPC_URL="http://127.0.0.1:19443"

# Vector A: Unauthenticated State Modification
curl -s -X POST "$RPC_URL" \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"submittransaction","params":["deadbeef"],"id":1}'

# Vector B: Tampered JWT Token
FAKE_JWT="eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiJhZG1pbiIsInJvbGUiOiJhZG1pbiIsImV4cCI6OTk5OTk5OTk5OX0.invalidsignaturehere"

curl -s -X POST "$RPC_URL" \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $FAKE_JWT" \
  -d '{"jsonrpc":"2.0","method":"generatetoaddress","params":[1, "addr"],"id":2}'
```

---

## 3. Observed Behavior & Red Team Findings

1. **Authentication Enforcement**:
   - The RPC handler extracts `Authorization: Bearer <token>`. Unauthenticated requests to non-whitelisted routes receive HTTP 401 Unauthorized or JSON-RPC error: `Unauthorized: Missing or invalid authentication token`.
2. **Cryptographic Signature Verification**:
   - `jwt.verify_token(token)` validates HMAC-SHA256 signature against the node's internal JWT secret. Forged signatures fail parsing immediately.
3. **Refresh Token Rejection**:
   - Refresh tokens contain `token_type: "refresh"`. The server explicitly rejects them on line 1329 with:
     ```text
     Refresh tokens cannot be used for RPC authentication
     ```
4. **Role-Based Access Control (RBAC)**:
   - `submittransaction` and `generate` are strictly bounded to `Miner` and `Admin` roles. `Readonly` tokens receive `Forbidden: method requires Miner or Admin privilege`.

---

## 4. Impact Assessment

- **Availability**: Maintained (Unauthorized DoS via `generate` blocked).
- **Integrity**: Protected (Privileged state mutations cannot be executed without authentication).
- **Confidentiality**: Protected (Wallet private data and node keys protected behind auth).

---

## 5. Defense Verification

- Automated test executed: `cargo test -p bitquan-rpc --test jwt_simple_test`
- Test Output:
  ```text
  running 12 tests
  test test_jwt_admin_role_check ... ok
  test test_jwt_auth_invalid_user ... ok
  test test_jwt_config_default ... ok
  test test_jwt_from_config ... ok
  test test_jwt_auth_add_user ... ok
  test test_jwt_auth_invalid_password ... ok
  test test_jwt_auth_creation ... ok
  test test_jwt_refresh_token ... ok
  test test_jwt_refresh_with_access_token_fails ... ok
  test test_jwt_token_verification_fails_with_wrong_secret ... ok
  test test_jwt_token_claims_structure ... ok
  test test_jwt_refresh_token_expiration ... ok
  test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.40s
  ```
- **Red Team Verdict**: Defense is ACTIVE and functioning as intended.
