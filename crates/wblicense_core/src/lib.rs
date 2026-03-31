use base64::Engine;
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use wbcore::{CapabilityProvider, LicenseTier, ToolId};

pub const ENTITLEMENT_SCHEMA_VERSION: u16 = 1;

fn default_schema_version() -> u16 {
    ENTITLEMENT_SCHEMA_VERSION
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EntitlementDocument {
    #[serde(default = "default_schema_version")]
    pub schema_version: u16,
    pub entitlement_id: String,
    pub customer_id: String,
    pub product: String,
    pub tier: LicenseTier,
    pub capabilities: EntitlementCapabilitiesDoc,
    pub not_before_unix: u64,
    pub expires_at_unix: u64,
    pub issued_at_unix: u64,
    pub issuer: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct EntitlementCapabilitiesDoc {
    pub max_tier: Option<LicenseTier>,
    pub allowed_tool_ids: BTreeSet<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SignedEntitlement {
    pub alg: String,
    pub kid: String,
    pub payload: EntitlementDocument,
    pub signature_b64url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct LocalLicenseState {
    pub active_entitlement: Option<SignedEntitlement>,
    pub last_verified_unix: Option<u64>,
    pub source: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct LeaseState {
    pub lease_id: Option<String>,
    pub acquired_at_unix: Option<u64>,
    pub expires_at_unix: Option<u64>,
    pub renewed_at_unix: Option<u64>,
}

#[derive(Debug, Clone, Default)]
pub struct VerificationKeyStore {
    keys: BTreeMap<String, VerifyingKey>,
}

impl VerificationKeyStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, kid: impl Into<String>, key: VerifyingKey) {
        self.keys.insert(kid.into(), key);
    }

    pub fn get(&self, kid: &str) -> Option<&VerifyingKey> {
        self.keys.get(kid)
    }

    pub fn insert_base64url_public_key(
        &mut self,
        kid: impl Into<String>,
        public_key_b64url: &str,
    ) -> Result<(), LicenseError> {
        let key_bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(public_key_b64url)
            .map_err(|_| LicenseError::InvalidSignatureEncoding)?;
        let key_array: [u8; 32] = key_bytes
            .try_into()
            .map_err(|_| LicenseError::InvalidSignatureEncoding)?;
        let key = VerifyingKey::from_bytes(&key_array).map_err(|_| LicenseError::InvalidSignature)?;
        self.insert(kid, key);
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct VerifiedEntitlement {
    pub signed: SignedEntitlement,
    pub verified_at_unix: u64,
}

#[derive(Debug, thiserror::Error)]
pub enum LicenseError {
    #[error("invalid signed entitlement json")]
    InvalidEnvelopeJson,
    #[error("unsupported signature algorithm: {0}")]
    UnsupportedAlgorithm(String),
    #[error("unsupported entitlement schema version: {0}")]
    UnsupportedSchemaVersion(u16),
    #[error("invalid entitlement: {0}")]
    InvalidEntitlement(String),
    #[error("unknown verification key id: {0}")]
    UnknownKeyId(String),
    #[error("invalid signature encoding")]
    InvalidSignatureEncoding,
    #[error("invalid signature")]
    InvalidSignature,
    #[error("entitlement not yet valid")]
    NotYetValid,
    #[error("entitlement expired")]
    Expired,
    #[error("license state i/o error: {0}")]
    LicenseStateIo(String),
    #[error("license state file is corrupt or unreadable")]
    LicenseStateCorrupt,
}

fn platform_config_dir() -> Option<PathBuf> {
    if cfg!(target_os = "macos") {
        std::env::var("HOME")
            .ok()
            .map(|h| PathBuf::from(h).join("Library").join("Application Support"))
    } else if cfg!(target_os = "windows") {
        std::env::var("APPDATA").ok().map(PathBuf::from)
    } else {
        // Linux, *BSD, etc. — honour XDG_CONFIG_HOME or fall back to ~/.config
        std::env::var("XDG_CONFIG_HOME")
            .ok()
            .map(PathBuf::from)
            .or_else(|| std::env::var("HOME").ok().map(|h| PathBuf::from(h).join(".config")))
    }
}

impl LocalLicenseState {
    /// Returns the default on-disk path for the license state file.
    ///
    /// - macOS:   `~/Library/Application Support/whitebox/{product_slug}/license_state.json`
    /// - Windows: `%APPDATA%\whitebox\{product_slug}\license_state.json`
    /// - Linux:   `$XDG_CONFIG_HOME/whitebox/{product_slug}/license_state.json`
    ///            (falls back to `~/.config/whitebox/{product_slug}/license_state.json`)
    ///
    /// Returns `None` if the platform config base directory cannot be resolved.
    pub fn default_path(product_slug: &str) -> Option<PathBuf> {
        platform_config_dir().map(|base| {
            base.join("whitebox")
                .join(product_slug)
                .join("license_state.json")
        })
    }

    /// Load state from `path`. Returns `Ok(Default)` if the file does not yet exist.
    ///
    /// Fails with [`LicenseError::LicenseStateCorrupt`] if the file exists but cannot
    /// be parsed, or [`LicenseError::LicenseStateIo`] for other I/O errors.
    pub fn load(path: &Path) -> Result<Self, LicenseError> {
        match std::fs::read_to_string(path) {
            Ok(contents) => {
                serde_json::from_str(&contents).map_err(|_| LicenseError::LicenseStateCorrupt)
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(Self::default()),
            Err(e) => Err(LicenseError::LicenseStateIo(e.to_string())),
        }
    }

    /// Serialize this state to `path`, creating parent directories as needed.
    pub fn save(&self, path: &Path) -> Result<(), LicenseError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| LicenseError::LicenseStateIo(e.to_string()))?;
        }
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| LicenseError::LicenseStateIo(e.to_string()))?;
        std::fs::write(path, json).map_err(|e| LicenseError::LicenseStateIo(e.to_string()))
    }

    /// Record a freshly-verified entitlement into this state.
    ///
    /// This does **not** save to disk; call [`save`](Self::save) afterwards if persistence
    /// is required.
    pub fn record_verification(&mut self, verified: &VerifiedEntitlement) {
        self.active_entitlement = Some(verified.signed.clone());
        self.last_verified_unix = Some(verified.verified_at_unix);
    }

    /// Returns `true` if the cached entitlement has not yet expired according to `now_unix`.
    ///
    /// Does **not** re-verify the cryptographic signature; use [`reload_verified`](Self::reload_verified)
    /// for full re-verification when coming back online.
    pub fn is_still_valid(&self, now_unix: u64) -> bool {
        match &self.active_entitlement {
            Some(e) => now_unix >= e.payload.not_before_unix && now_unix <= e.payload.expires_at_unix,
            None => false,
        }
    }

    /// Re-verify the cached signed entitlement against `key_store` at `now_unix`.
    ///
    /// Useful on startup to re-establish a `VerifiedEntitlement` from persisted state
    /// without a network round-trip.
    pub fn reload_verified(
        &self,
        key_store: &VerificationKeyStore,
        now_unix: u64,
    ) -> Result<VerifiedEntitlement, LicenseError> {
        let signed = self
            .active_entitlement
            .clone()
            .ok_or_else(|| LicenseError::InvalidEntitlement("no cached entitlement".to_string()))?;
        verify_signed_entitlement(signed, key_store, now_unix)
    }
}

fn validate_entitlement_payload(payload: &EntitlementDocument) -> Result<(), LicenseError> {
    if payload.schema_version != ENTITLEMENT_SCHEMA_VERSION {
        return Err(LicenseError::UnsupportedSchemaVersion(payload.schema_version));
    }

    if payload.entitlement_id.trim().is_empty() {
        return Err(LicenseError::InvalidEntitlement(
            "entitlement_id must be non-empty".to_string(),
        ));
    }
    if payload.customer_id.trim().is_empty() {
        return Err(LicenseError::InvalidEntitlement(
            "customer_id must be non-empty".to_string(),
        ));
    }
    if payload.product.trim().is_empty() {
        return Err(LicenseError::InvalidEntitlement(
            "product must be non-empty".to_string(),
        ));
    }
    if payload.issuer.trim().is_empty() {
        return Err(LicenseError::InvalidEntitlement(
            "issuer must be non-empty".to_string(),
        ));
    }
    if payload.not_before_unix > payload.expires_at_unix {
        return Err(LicenseError::InvalidEntitlement(
            "not_before_unix must be <= expires_at_unix".to_string(),
        ));
    }
    if payload.issued_at_unix > payload.expires_at_unix {
        return Err(LicenseError::InvalidEntitlement(
            "issued_at_unix must be <= expires_at_unix".to_string(),
        ));
    }

    Ok(())
}

pub fn parse_signed_entitlement_json(signed_json: &str) -> Result<SignedEntitlement, LicenseError> {
    serde_json::from_str(signed_json).map_err(|_| LicenseError::InvalidEnvelopeJson)
}

pub fn verify_signed_entitlement_json(
    signed_json: &str,
    key_store: &VerificationKeyStore,
    now_unix: u64,
) -> Result<VerifiedEntitlement, LicenseError> {
    let signed = parse_signed_entitlement_json(signed_json)?;
    verify_signed_entitlement(signed, key_store, now_unix)
}

pub fn verify_signed_entitlement(
    signed: SignedEntitlement,
    key_store: &VerificationKeyStore,
    now_unix: u64,
) -> Result<VerifiedEntitlement, LicenseError> {
    if signed.alg != "EdDSA" {
        return Err(LicenseError::UnsupportedAlgorithm(signed.alg));
    }

    let verifying_key = key_store
        .get(&signed.kid)
        .ok_or_else(|| LicenseError::UnknownKeyId(signed.kid.clone()))?;

    let payload_bytes = serde_json::to_vec(&signed.payload)
        .map_err(|_| LicenseError::InvalidSignature)?;

    let sig_bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(&signed.signature_b64url)
        .map_err(|_| LicenseError::InvalidSignatureEncoding)?;
    let signature = Signature::from_slice(&sig_bytes).map_err(|_| LicenseError::InvalidSignature)?;

    verifying_key
        .verify(&payload_bytes, &signature)
        .map_err(|_| LicenseError::InvalidSignature)?;

    validate_entitlement_payload(&signed.payload)?;

    if now_unix < signed.payload.not_before_unix {
        return Err(LicenseError::NotYetValid);
    }
    if now_unix > signed.payload.expires_at_unix {
        return Err(LicenseError::Expired);
    }

    Ok(VerifiedEntitlement {
        signed,
        verified_at_unix: now_unix,
    })
}

pub fn verify_signed_entitlement_now(
    signed: SignedEntitlement,
    key_store: &VerificationKeyStore,
) -> Result<VerifiedEntitlement, LicenseError> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    verify_signed_entitlement(signed, key_store, now)
}

#[derive(Debug, Clone)]
pub struct EntitlementCapabilities {
    pub max_tier: LicenseTier,
    pub allowed_tool_ids: BTreeSet<String>,
    pub expires_at_unix: u64,
    pub now_unix: u64,
}

impl EntitlementCapabilities {
    pub fn from_verified(verified: &VerifiedEntitlement, now_unix: u64) -> Self {
        let max_tier = verified
            .signed
            .payload
            .capabilities
            .max_tier
            .unwrap_or(verified.signed.payload.tier);
        Self {
            max_tier,
            allowed_tool_ids: verified.signed.payload.capabilities.allowed_tool_ids.clone(),
            expires_at_unix: verified.signed.payload.expires_at_unix,
            now_unix,
        }
    }

    pub fn is_expired(&self) -> bool {
        self.now_unix > self.expires_at_unix
    }
}

impl CapabilityProvider for EntitlementCapabilities {
    fn has_tool_access(&self, tool_id: ToolId, required_tier: LicenseTier) -> bool {
        if self.is_expired() {
            return false;
        }

        if required_tier > self.max_tier {
            return false;
        }

        if self.allowed_tool_ids.is_empty() {
            return true;
        }

        self.allowed_tool_ids.contains(tool_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::Engine;
    use ed25519_dalek::{Signer, SigningKey};

    const VALID_SIGNED_ENTITLEMENT_JSON: &str =
        include_str!("../tests/fixtures/valid_signed_entitlement.json");
    const TAMPERED_SIGNED_ENTITLEMENT_JSON: &str =
        include_str!("../tests/fixtures/tampered_signed_entitlement.json");
    const K1_PUBLIC_KEY_B64URL: &str = include_str!("../tests/fixtures/k1_public_key.b64url.txt");

    fn fixture_key_store() -> VerificationKeyStore {
        let mut keys = VerificationKeyStore::new();
        keys.insert_base64url_public_key("k1", K1_PUBLIC_KEY_B64URL.trim())
            .unwrap();
        keys
    }

    fn signed_fixture(now: u64) -> (SignedEntitlement, VerificationKeyStore) {
        let signing_key = SigningKey::from_bytes(&[7u8; 32]);
        let verifying_key = signing_key.verifying_key();

        let payload = EntitlementDocument {
            schema_version: ENTITLEMENT_SCHEMA_VERSION,
            entitlement_id: "ent_001".to_string(),
            customer_id: "cust_123".to_string(),
            product: "whitebox_next_gen".to_string(),
            tier: LicenseTier::Pro,
            capabilities: EntitlementCapabilitiesDoc {
                max_tier: Some(LicenseTier::Pro),
                allowed_tool_ids: BTreeSet::new(),
            },
            not_before_unix: now.saturating_sub(60),
            expires_at_unix: now + 3600,
            issued_at_unix: now.saturating_sub(60),
            issuer: "radiant-garden".to_string(),
        };

        let payload_bytes = serde_json::to_vec(&payload).unwrap();
        let sig = signing_key.sign(&payload_bytes);
        let signature_b64url = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(sig.to_bytes());

        let signed = SignedEntitlement {
            alg: "EdDSA".to_string(),
            kid: "k1".to_string(),
            payload,
            signature_b64url,
        };

        let mut keys = VerificationKeyStore::new();
        keys.insert("k1", verifying_key);
        (signed, keys)
    }

    #[test]
    fn verifies_valid_signature() {
        let now = 1_700_000_000;
        let (signed, keys) = signed_fixture(now);
        let verified = verify_signed_entitlement(signed, &keys, now).unwrap();
        assert_eq!(verified.signed.payload.customer_id, "cust_123");
    }

    #[test]
    fn rejects_tampered_payload() {
        let now = 1_700_000_000;
        let (mut signed, keys) = signed_fixture(now);
        signed.payload.customer_id = "attacker".to_string();
        let err = verify_signed_entitlement(signed, &keys, now).unwrap_err();
        assert!(matches!(err, LicenseError::InvalidSignature));
    }

    #[test]
    fn enforces_expiry() {
        let now = 1_700_000_000;
        let (signed, keys) = signed_fixture(now);
        let err = verify_signed_entitlement(signed, &keys, now + 4_000).unwrap_err();
        assert!(matches!(err, LicenseError::Expired));
    }

    #[test]
    fn rejects_unsupported_schema_version() {
        let now: u64 = 1_700_000_000;
        let signing_key = SigningKey::from_bytes(&[7u8; 32]);
        let verifying_key = signing_key.verifying_key();
        let payload = EntitlementDocument {
            schema_version: ENTITLEMENT_SCHEMA_VERSION + 1,
            entitlement_id: "ent_001".to_string(),
            customer_id: "cust_123".to_string(),
            product: "whitebox_next_gen".to_string(),
            tier: LicenseTier::Pro,
            capabilities: EntitlementCapabilitiesDoc {
                max_tier: Some(LicenseTier::Pro),
                allowed_tool_ids: BTreeSet::new(),
            },
            not_before_unix: now.saturating_sub(60),
            expires_at_unix: now + 3600,
            issued_at_unix: now.saturating_sub(60),
            issuer: "radiant-garden".to_string(),
        };
        let payload_bytes = serde_json::to_vec(&payload).unwrap();
        let sig = signing_key.sign(&payload_bytes);
        let signature_b64url = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(sig.to_bytes());

        let signed = SignedEntitlement {
            alg: "EdDSA".to_string(),
            kid: "k1".to_string(),
            payload,
            signature_b64url,
        };

        let mut keys = VerificationKeyStore::new();
        keys.insert("k1", verifying_key);

        let err = verify_signed_entitlement(signed, &keys, now).unwrap_err();
        assert!(matches!(err, LicenseError::UnsupportedSchemaVersion(_)));
    }

    #[test]
    fn verifies_json_envelope() {
        let now = 1_700_000_000;
        let verified = verify_signed_entitlement_json(VALID_SIGNED_ENTITLEMENT_JSON, &fixture_key_store(), now)
            .unwrap();
        assert_eq!(verified.signed.payload.product, "whitebox_next_gen");
    }

    #[test]
    fn rejects_tampered_fixture_envelope() {
        let now = 1_700_000_000;
        let err = verify_signed_entitlement_json(TAMPERED_SIGNED_ENTITLEMENT_JSON, &fixture_key_store(), now)
            .unwrap_err();
        assert!(matches!(err, LicenseError::InvalidSignature));
    }

    #[test]
    fn capability_provider_respects_tier() {
        let now = 1_700_000_000;
        let (signed, keys) = signed_fixture(now);
        let verified = verify_signed_entitlement(signed, &keys, now).unwrap();
        let caps = EntitlementCapabilities::from_verified(&verified, now);

        assert!(caps.has_tool_access("any_tool", LicenseTier::Open));
        assert!(caps.has_tool_access("any_tool", LicenseTier::Pro));
        assert!(!caps.has_tool_access("any_tool", LicenseTier::Enterprise));
    }

    #[test]
    fn local_license_state_roundtrip() {
        let now = 1_700_000_000u64;
        let (signed, keys) = signed_fixture(now);
        let verified = verify_signed_entitlement(signed, &keys, now).unwrap();

        let mut state = LocalLicenseState::default();
        state.record_verification(&verified);
        state.source = Some("test".to_string());

        let dir = std::env::temp_dir().join("wblicense_core_test_roundtrip");
        let path = dir.join("license_state.json");

        state.save(&path).unwrap();
        let loaded = LocalLicenseState::load(&path).unwrap();

        assert_eq!(loaded.source, Some("test".to_string()));
        assert_eq!(loaded.last_verified_unix, Some(now));
        assert!(loaded.active_entitlement.is_some());

        // Clean up
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn local_license_state_load_missing_returns_default() {
        let path = std::env::temp_dir().join("wblicense_core_nonexistent_state.json");
        // Ensure it doesn't exist
        let _ = std::fs::remove_file(&path);

        let state = LocalLicenseState::load(&path).unwrap();
        assert!(state.active_entitlement.is_none());
        assert!(state.last_verified_unix.is_none());
    }

    #[test]
    fn local_license_state_is_still_valid() {
        let now = 1_700_000_000u64;
        let (signed, keys) = signed_fixture(now);
        let verified = verify_signed_entitlement(signed, &keys, now).unwrap();

        let mut state = LocalLicenseState::default();
        state.record_verification(&verified);

        assert!(state.is_still_valid(now));
        assert!(state.is_still_valid(now + 100));
        // Exactly at expiry boundary (now + 3600) — still valid
        assert!(state.is_still_valid(now + 3600));
        // One second past expiry — invalid
        assert!(!state.is_still_valid(now + 3601));
        // Empty state — always invalid
        assert!(!LocalLicenseState::default().is_still_valid(now));
    }

    #[test]
    fn local_license_state_reload_verified() {
        let now = 1_700_000_000u64;
        let (signed, keys) = signed_fixture(now);
        let verified = verify_signed_entitlement(signed, &keys, now).unwrap();

        let mut state = LocalLicenseState::default();
        state.record_verification(&verified);

        let reloaded = state.reload_verified(&keys, now).unwrap();
        assert_eq!(reloaded.signed.payload.customer_id, "cust_123");
    }

    #[test]
    fn local_license_state_reload_verified_detects_expiry() {
        let now = 1_700_000_000u64;
        let (signed, keys) = signed_fixture(now);
        let verified = verify_signed_entitlement(signed, &keys, now).unwrap();

        let mut state = LocalLicenseState::default();
        state.record_verification(&verified);

        let err = state.reload_verified(&keys, now + 4_000).unwrap_err();
        assert!(matches!(err, LicenseError::Expired));
    }

    #[test]
    fn local_license_state_default_path_is_some() {
        let p = LocalLicenseState::default_path("whitebox_next_gen");
        assert!(p.is_some(), "expected a valid config path on this platform");
        let p = p.unwrap();
        assert!(p.ends_with("license_state.json"));
        assert!(p.to_string_lossy().contains("whitebox"));
        assert!(p.to_string_lossy().contains("whitebox_next_gen"));
    }
}
