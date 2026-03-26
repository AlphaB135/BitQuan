//! Wallet session management with timeout, concurrent protection,
//! and brute-force mitigation via exponential backoff.

use std::sync::Mutex;
use std::time::{Duration, Instant};

use thiserror::Error;

use super::{
    keystore::{Keystore, KeystoreError},
    secure_types::SecurePrivateKey,
    secure_types::SecureString,
};

/// Session-level errors.
#[derive(Debug, Error)]
pub enum SessionError {
    /// Password was incorrect.
    #[error("invalid password")]
    InvalidPassword,
    /// Session has expired (timeout reached).
    #[error("session expired")]
    SessionExpired,
    /// Session was explicitly locked.
    #[error("session is locked")]
    SessionLocked,
    /// Too many failed attempts; caller must wait.
    #[error("too many attempts ({attempts}); wait {wait_secs}s")]
    TooManyAttempts {
        /// Consecutive failed unlock attempts.
        attempts: u32,
        /// Seconds the caller must wait before retrying.
        wait_secs: u64,
    },
    /// Underlying keystore error.
    #[error(transparent)]
    Keystore(#[from] KeystoreError),
}

/// Base delay (seconds) before exponential ramp-up.
const BACKOFF_BASE_SECS: u64 = 1;
/// Delay starts at attempt N (1-indexed).
const BACKOFF_START_ATTEMPT: u32 = 4;
/// Maximum backoff delay (seconds).
const BACKOFF_MAX_SECS: u64 = 30;
/// Failed attempts that trigger a hard lockout.
const LOCKOUT_THRESHOLD: u32 = 20;
/// Duration of a hard lockout (seconds).
const LOCKOUT_DURATION_SECS: u64 = 60;

/// Default session timeout (seconds).
const DEFAULT_TIMEOUT_SECS: u64 = 60;

/// Manages an unlocked wallet session with automatic timeout,
/// concurrent-attempt protection, and progressive brute-force
/// mitigation.
pub struct WalletSession {
    keystore: Keystore,
    unlocked_key: Mutex<Option<CachedKey>>,
    timeout: Duration,
    unlock_mutex: Mutex<()>,
    failed_attempts: Mutex<u32>,
    last_failed_at: Mutex<Option<Instant>>,
}

struct CachedKey {
    key: SecurePrivateKey,
    unlocked_at: Instant,
}

impl CachedKey {
    fn new(key: SecurePrivateKey) -> Self {
        Self {
            key,
            unlocked_at: Instant::now(),
        }
    }

    fn is_expired(&self, timeout: Duration) -> bool {
        self.unlocked_at.elapsed() > timeout
    }
}

impl WalletSession {
    /// Creates a new session wrapping the given keystore.
    ///
    /// The session starts locked. Call [`unlock`](Self::unlock) to
    /// decrypt the private key into memory.
    pub fn new(keystore: Keystore) -> Self {
        Self::with_timeout(keystore, Duration::from_secs(DEFAULT_TIMEOUT_SECS))
    }

    /// Creates a session with a custom timeout.
    pub fn with_timeout(keystore: Keystore, timeout: Duration) -> Self {
        Self {
            keystore,
            unlocked_key: Mutex::new(None),
            timeout,
            unlock_mutex: Mutex::new(()),
            failed_attempts: Mutex::new(0),
            last_failed_at: Mutex::new(None),
        }
    }

    /// Attempts to unlock the wallet with the given password.
    ///
    /// On success the decrypted key is cached in memory for
    /// `timeout` seconds. The failed-attempt counter is reset.
    ///
    /// On failure an exponential backoff delay is enforced and
    /// the caller receives [`SessionError::TooManyAttempts`] if
    /// they must wait before trying again.
    pub fn unlock(&self, password: &SecureString) -> Result<(), SessionError> {
        // Prevent concurrent unlock attempts (anti parallel brute-force).
        let _guard = self
            .unlock_mutex
            .lock()
            .map_err(|_| SessionError::SessionLocked)?;

        // Enforce backoff delay from previous failure.
        if let Some(wait) = self.remaining_backoff() {
            return Err(SessionError::TooManyAttempts {
                attempts: *self
                    .failed_attempts
                    .lock()
                    .map_err(|_| SessionError::SessionLocked)?,
                wait_secs: wait.as_secs(),
            });
        }

        match self.keystore.unlock(password) {
            Ok(key) => {
                // Reset failure counter on success.
                *self
                    .failed_attempts
                    .lock()
                    .map_err(|_| SessionError::SessionLocked)? = 0;
                *self
                    .last_failed_at
                    .lock()
                    .map_err(|_| SessionError::SessionLocked)? = None;

                let mut slot = self
                    .unlocked_key
                    .lock()
                    .map_err(|_| SessionError::SessionLocked)?;
                *slot = Some(CachedKey::new(key));
                Ok(())
            }
            Err(KeystoreError::InvalidPassword) => {
                self.record_failure();
                Err(SessionError::InvalidPassword)
            }
            Err(e) => Err(SessionError::Keystore(e)),
        }
    }

    /// Returns a copy of the cached private key if the session is
    /// active and has not timed out.
    ///
    /// Calling this method extends the session timeout (activity-based
    /// refresh), mirroring Bitcoin Core's behaviour where
    /// `walletpassphrase` overrides the previous timer.
    pub fn get_key(&self) -> Result<SecurePrivateKey, SessionError> {
        let mut slot = self
            .unlocked_key
            .lock()
            .map_err(|_| SessionError::SessionLocked)?;

        let cached = slot.as_mut().ok_or(SessionError::SessionLocked)?;

        if cached.is_expired(self.timeout) {
            *slot = None;
            return Err(SessionError::SessionExpired);
        }

        // Activity-based refresh: extend the unlock timestamp.
        cached.unlocked_at = Instant::now();
        Ok(cached.key.clone())
    }

    /// Immediately zeroizes the cached key and locks the session.
    pub fn lock(&self) {
        if let Ok(mut slot) = self.unlocked_key.lock() {
            *slot = None;
        }
    }

    /// Returns `true` if the session is currently unlocked and
    /// has not timed out.
    pub fn is_unlocked(&self) -> bool {
        self.unlocked_key
            .lock()
            .ok()
            .and_then(|slot| slot.as_ref().map(|c| !c.is_expired(self.timeout)))
            .unwrap_or(false)
    }

    /// Returns the current count of consecutive failed attempts.
    pub fn failed_attempts(&self) -> u32 {
        self.failed_attempts.lock().map(|a| *a).unwrap_or(0)
    }

    // -- private helpers -------------------------------------------

    fn record_failure(&self) {
        if let (Ok(mut attempts), Ok(mut last)) =
            (self.failed_attempts.lock(), self.last_failed_at.lock())
        {
            *attempts += 1;
            *last = Some(Instant::now());
        }
    }

    fn remaining_backoff(&self) -> Option<Duration> {
        let attempts = self.failed_attempts.lock().ok()?;
        let last_guard = self.last_failed_at.lock().ok()?;
        let last = (*last_guard)?;

        if *attempts < BACKOFF_START_ATTEMPT {
            return None;
        }

        // Hard lockout after threshold.
        if *attempts >= LOCKOUT_THRESHOLD {
            let elapsed = last.elapsed();
            let lockout = Duration::from_secs(LOCKOUT_DURATION_SECS);
            if elapsed < lockout {
                return Some(lockout - elapsed);
            }
            // Lockout expired; reset counter on next attempt.
            return None;
        }

        // Exponential backoff: base * 2^(attempts - start).
        let exponent = *attempts - BACKOFF_START_ATTEMPT;
        let delay_secs = BACKOFF_BASE_SECS
            .saturating_mul(1 << exponent.min(30))
            .min(BACKOFF_MAX_SECS);

        let elapsed = last.elapsed();
        let delay = Duration::from_secs(delay_secs);
        if elapsed < delay {
            return Some(delay - elapsed);
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_keystore() -> Keystore {
        let private = SecurePrivateKey::new(vec![1, 2, 3, 4]);
        let password = SecureString::new("test-password".into());
        Keystore::new(&private, &password, "bq1test".into()).expect("keystore creation failed")
    }

    #[test]
    fn unlock_and_get_key() {
        let ks = make_keystore();
        let session = WalletSession::new(ks);
        let pw = SecureString::new("test-password".into());

        assert!(!session.is_unlocked());
        session.unlock(&pw).expect("unlock failed");
        assert!(session.is_unlocked());

        let key = session.get_key().expect("get_key failed");
        assert_eq!(key.as_slice(), &[1, 2, 3, 4]);
    }

    #[test]
    fn wrong_password_fails() {
        let ks = make_keystore();
        let session = WalletSession::new(ks);
        let wrong = SecureString::new("wrong".into());

        let err = session.unlock(&wrong).unwrap_err();
        assert!(matches!(err, SessionError::InvalidPassword));
        assert_eq!(session.failed_attempts(), 1);
    }

    #[test]
    fn lock_clears_key() {
        let ks = make_keystore();
        let session = WalletSession::new(ks);
        let pw = SecureString::new("test-password".into());

        session.unlock(&pw).expect("unlock failed");
        assert!(session.is_unlocked());

        session.lock();
        assert!(!session.is_unlocked());

        let err = session.get_key().unwrap_err();
        assert!(matches!(err, SessionError::SessionLocked));
    }

    #[test]
    fn success_resets_failed_attempts() {
        let ks = make_keystore();
        let session = WalletSession::new(ks);
        let wrong = SecureString::new("wrong".into());
        let correct = SecureString::new("test-password".into());

        for _ in 0..3 {
            let _ = session.unlock(&wrong);
        }
        assert_eq!(session.failed_attempts(), 3);

        session.unlock(&correct).expect("unlock should succeed");
        assert_eq!(session.failed_attempts(), 0);
    }

    #[test]
    fn session_timeout() {
        let ks = make_keystore();
        let session = WalletSession::with_timeout(ks, Duration::from_millis(50));
        let pw = SecureString::new("test-password".into());

        session.unlock(&pw).expect("unlock failed");
        assert!(session.is_unlocked());

        std::thread::sleep(Duration::from_millis(80));
        assert!(!session.is_unlocked());

        let err = session.get_key().unwrap_err();
        assert!(matches!(err, SessionError::SessionExpired));
    }

    #[test]
    fn backoff_enforced_after_threshold() {
        let ks = make_keystore();
        let session = WalletSession::new(ks);
        let wrong = SecureString::new("wrong".into());

        // First BACKOFF_START_ATTEMPT attempts: no delay.
        for _ in 0..BACKOFF_START_ATTEMPT {
            let _ = session.unlock(&wrong);
        }

        // Next attempt should trigger backoff.
        let err = session.unlock(&wrong).unwrap_err();
        assert!(matches!(err, SessionError::TooManyAttempts { .. }));
    }
}
