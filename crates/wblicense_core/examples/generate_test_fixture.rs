use base64::Engine;
use ed25519_dalek::{Signer, SigningKey};
use std::collections::BTreeSet;
use wblicense_core::{
    EntitlementCapabilitiesDoc, EntitlementDocument, SignedEntitlement, ENTITLEMENT_SCHEMA_VERSION,
};
use wbcore::LicenseTier;

fn main() {
    let signing_key = SigningKey::from_bytes(&[7u8; 32]);
    let verifying_key = signing_key.verifying_key();
    let now: u64 = 1_700_000_000;

    let mut allowed_tool_ids = BTreeSet::new();
    allowed_tool_ids.insert("slope".to_string());
    allowed_tool_ids.insert("aspect".to_string());

    let payload = EntitlementDocument {
        schema_version: ENTITLEMENT_SCHEMA_VERSION,
        entitlement_id: "ent_fixture_001".to_string(),
        customer_id: "cust_fixture_001".to_string(),
        product: "whitebox_next_gen".to_string(),
        tier: LicenseTier::Pro,
        capabilities: EntitlementCapabilitiesDoc {
            max_tier: Some(LicenseTier::Pro),
            allowed_tool_ids,
        },
        not_before_unix: now.saturating_sub(60),
        expires_at_unix: now + 86_400,
        issued_at_unix: now.saturating_sub(60),
        issuer: "radiant-garden".to_string(),
    };

    let payload_bytes = serde_json::to_vec(&payload).unwrap();
    let sig = signing_key.sign(&payload_bytes);
    let signature_b64url = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(sig.to_bytes());
    let public_key_b64url = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(verifying_key.to_bytes());

    let signed = SignedEntitlement {
        alg: "EdDSA".to_string(),
        kid: "k1".to_string(),
        payload,
        signature_b64url,
    };

    println!("PUBLIC_KEY_B64URL={public_key_b64url}");
    println!("{}", serde_json::to_string_pretty(&signed).unwrap());
}
