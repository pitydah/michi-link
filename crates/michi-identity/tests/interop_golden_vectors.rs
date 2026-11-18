use ed25519_dalek::SigningKey;
use michi_identity::{
    aircat::{canonical_aircat_grant_bytes_v1, verify_aircat_grant_v1, AirCatGrantClaims},
    capabilities::{CanonicalServerCapabilities, AIRCAT_REQUIRED_CAPABILITIES, IDENTITY_SCHEME_V1},
    encode_base64url,
    home::{
        device_auth_challenge_payload, home_enrollment_pop_payload, server_auth_confirm_payload,
        sign_device_auth_challenge, sign_home_enrollment_pop, sign_server_auth_confirm,
        verify_device_auth_challenge, verify_home_enrollment_pop, verify_membership,
        verify_server_auth_session, HomeRootAuthority,
    },
    DeviceAuthSessionResponse, MichiId, Role,
};
use serde_json::json;
use std::path::PathBuf;

fn build_deterministic_vectors() -> serde_json::Value {
    let root_sk = SigningKey::from_bytes(&[0x11u8; 32]);
    let root_vk = root_sk.verifying_key();
    let root = HomeRootAuthority::from_signing_key(root_sk);
    let home_id = root.home_id();
    let root_pk = root.public_key_base64url();

    let server_sk = SigningKey::from_bytes(&[0x22u8; 32]);
    let server_vk = server_sk.verifying_key();
    let server_michi_id = MichiId::from_public_key(&server_vk).to_base64url();
    let server_pk = encode_base64url(&server_vk.to_bytes());

    let client_sk = SigningKey::from_bytes(&[0x33u8; 32]);
    let client_vk = client_sk.verifying_key();
    let client_michi_id = MichiId::from_public_key(&client_vk).to_base64url();
    let client_pk = encode_base64url(&client_vk.to_bytes());

    let enroll_challenge_id = "550e8400-e29b-41d4-a716-446655440099";
    let enroll_nonce = "VFfZjzw8JeAM7-RFiTSrMA";
    let enroll_pop_bytes = home_enrollment_pop_payload(
        &home_id,
        &server_michi_id,
        &client_michi_id,
        &client_pk,
        enroll_challenge_id,
        enroll_nonce,
    );
    let enroll_pop_sig = sign_home_enrollment_pop(
        &client_sk,
        &home_id,
        &server_michi_id,
        &client_michi_id,
        &client_pk,
        enroll_challenge_id,
        enroll_nonce,
    );

    let issued_at = "2026-11-18T12:00:00Z";
    let server_membership = root.issue_membership(
        &home_id,
        &server_michi_id,
        &server_pk,
        "server",
        vec![Role::MusicServer, Role::LibraryHost, Role::PlaybackHost],
        issued_at,
        1,
    );
    let client_membership = root.issue_membership(
        &home_id,
        &client_michi_id,
        &client_pk,
        "mobile",
        vec![Role::MobilePlayer, Role::RemoteController, Role::SyncClient],
        issued_at,
        2,
    );

    let auth_challenge_id = "550e8400-e29b-41d4-a716-446655440111";
    let auth_nonce = "WnBfQ2hhbGxlbmdlTm9uY2VfMTIzNDU2";
    let auth_client_payload = device_auth_challenge_payload(
        &home_id,
        &server_michi_id,
        &client_michi_id,
        auth_challenge_id,
        auth_nonce,
    );
    let auth_client_sig = sign_device_auth_challenge(
        &client_sk,
        &home_id,
        &server_michi_id,
        &client_michi_id,
        auth_challenge_id,
        auth_nonce,
    );

    let session_token = "michi_home_deterministic_golden_token_v1";
    let server_confirm_payload = server_auth_confirm_payload(
        &home_id,
        &server_michi_id,
        &client_michi_id,
        auth_challenge_id,
        session_token,
    );
    let server_confirm_sig = sign_server_auth_confirm(
        &server_sk,
        &home_id,
        &server_michi_id,
        &client_michi_id,
        auth_challenge_id,
        session_token,
    );

    let claims = AirCatGrantClaims {
        version: 1,
        transfer_id: "550e8400-e29b-41d4-a716-446655440333".into(),
        home_id: home_id.clone(),
        authority_michi_id: home_id.clone(),
        source_michi_id: server_michi_id.clone(),
        destination_michi_id: client_michi_id.clone(),
        source_library_id: "lib-micro-default".into(),
        destination_library_id: "lib-mobile-default".into(),
        selection_scope: "ALBUMS".into(),
        scopes: vec!["content:read".into(), "content:transfer".into()],
        manifest_sha256: "a".repeat(64),
        issued_at_millis: 1_763_467_200_000,
        expires_at_millis: 1_763_467_800_000,
        nonce_base64_url: "WnBfQ2hhbGxlbmdlTm9uY2VfMTIzNDU2".into(),
    };
    let grant_canonical_bytes = canonical_aircat_grant_bytes_v1(&claims).unwrap();
    let signed_grant = root
        .issue_aircat_grant_v1(&claims, 1_763_467_200_000)
        .unwrap();

    let caps_ready = CanonicalServerCapabilities::from_runtime(
        true, true, true, true, true, true, true, true, true, true, true, true, true,
    );
    let caps_no_tls = CanonicalServerCapabilities::from_runtime(
        true, true, true, true, true, true, true, true, true, true, true, true, false,
    );

    // Self-verify every vector before emitting
    assert!(verify_home_enrollment_pop(
        &client_pk,
        &enroll_pop_sig,
        &home_id,
        &server_michi_id,
        &client_michi_id,
        enroll_challenge_id,
        enroll_nonce,
    )
    .unwrap());
    verify_membership(&server_membership, &root_pk, &home_id, &[]).unwrap();
    verify_membership(&client_membership, &root_pk, &home_id, &[]).unwrap();
    assert!(verify_device_auth_challenge(
        &client_pk,
        &auth_client_sig,
        &home_id,
        &server_michi_id,
        &client_michi_id,
        auth_challenge_id,
        auth_nonce,
    )
    .unwrap());
    let session_resp = DeviceAuthSessionResponse {
        session_token: session_token.into(),
        token_type: "Bearer".into(),
        expires_in: 3600,
        server_michi_id: server_michi_id.clone(),
        server_membership: server_membership.clone(),
        server_signature: server_confirm_sig.clone(),
    };
    verify_server_auth_session(
        &session_resp,
        &root_pk,
        &home_id,
        &client_michi_id,
        auth_challenge_id,
        &[],
    )
    .unwrap();
    verify_aircat_grant_v1(
        &signed_grant,
        &root_vk,
        &client_michi_id,
        1_763_467_300_000,
        false,
    )
    .unwrap();

    json!({
        "schema": "michi.interop.golden_vectors.r1",
        "identity_scheme": IDENTITY_SCHEME_V1,
        "home_root": {
            "seed_hex": "11".repeat(32),
            "home_id": home_id,
            "public_key": root_pk
        },
        "server": {
            "seed_hex": "22".repeat(32),
            "michi_id": server_michi_id,
            "public_key": server_pk,
            "membership": server_membership
        },
        "mobile_client": {
            "seed_hex": "33".repeat(32),
            "michi_id": client_michi_id,
            "public_key": client_pk,
            "membership": client_membership
        },
        "home_enrollment_pop": {
            "challenge_id": enroll_challenge_id,
            "challenge_nonce": enroll_nonce,
            "canonical_payload_utf8": String::from_utf8(enroll_pop_bytes).unwrap(),
            "client_signature": enroll_pop_sig
        },
        "home_mutual_auth": {
            "challenge_id": auth_challenge_id,
            "challenge_nonce": auth_nonce,
            "client_payload_utf8": String::from_utf8(auth_client_payload).unwrap(),
            "client_signature": auth_client_sig,
            "session_token": session_token,
            "server_confirm_payload_utf8": String::from_utf8(server_confirm_payload).unwrap(),
            "server_signature": server_confirm_sig
        },
        "aircat_grant_v1": {
            "canonical_payload_base64url": encode_base64url(&grant_canonical_bytes),
            "signed_grant": signed_grant
        },
        "capabilities": {
            "aircat_required": AIRCAT_REQUIRED_CAPABILITIES,
            "features_with_tls": caps_ready.to_features_map(),
            "features_without_tls": caps_no_tls.to_features_map()
        }
    })
}

#[test]
fn test_micro_mobile_interop_golden_vectors_match_checked_in_fixture() {
    let generated = build_deterministic_vectors();
    let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/vectors/interop/micro-mobile-interop-vectors.json");

    if std::env::var("MICHI_UPDATE_INTEROP_VECTORS").as_deref() == Ok("1") {
        if let Some(parent) = fixture_path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        let pretty = format!("{}\n", serde_json::to_string_pretty(&generated).unwrap());
        std::fs::write(&fixture_path, pretty).unwrap();
    }

    let on_disk_raw = std::fs::read_to_string(&fixture_path).expect(
        "interop golden vector fixture missing; run with MICHI_UPDATE_INTEROP_VECTORS=1 once",
    );
    let on_disk: serde_json::Value = serde_json::from_str(&on_disk_raw).unwrap();
    assert_eq!(on_disk, generated, "interop golden vectors drifted");
}
