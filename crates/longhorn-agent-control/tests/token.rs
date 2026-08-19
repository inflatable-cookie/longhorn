//! Token fixtures (Card 228): generation strength, constant-time verify,
//! and the credential posture — the token never appears in `Debug` output.

use longhorn_agent_control::{DISCOVERY_SCHEMA_VERSION, DiscoveryFile, InstanceToken};

#[test]
fn token_never_appears_in_debug_output() {
    let token = InstanceToken::generate().unwrap();

    let debug = format!("{token:?}");
    assert!(!debug.contains(token.as_str()));

    // The redaction holds through the discovery file that carries it.
    let file = DiscoveryFile {
        schema_version: DISCOVERY_SCHEMA_VERSION,
        app_id: "dev.example.soundcheck".to_owned(),
        pid: 1234,
        port: 49152,
        token: token.clone(),
    };
    let debug = format!("{file:?}");
    assert!(!debug.contains(token.as_str()));
}

#[test]
fn discovery_file_round_trip_preserves_the_token() {
    let token = InstanceToken::generate().unwrap();
    let file = DiscoveryFile {
        schema_version: DISCOVERY_SCHEMA_VERSION,
        app_id: "dev.example.nucleus".to_owned(),
        pid: 4321,
        port: 49153,
        token: token.clone(),
    };

    let json = serde_json::to_string(&file).unwrap();
    // The file is how an agent learns the credential: plaintext by design.
    assert!(json.contains(token.as_str()));

    let parsed: DiscoveryFile = serde_json::from_str(&json).unwrap();
    assert!(parsed.token.verify(token.as_str()));
}

#[test]
fn tampered_tokens_fail_verification() {
    let token = InstanceToken::generate().unwrap();
    let flipped = token
        .as_str()
        .chars()
        .enumerate()
        .map(|(index, symbol)| {
            if index == 0 {
                if symbol == 'a' { 'b' } else { 'a' }
            } else {
                symbol
            }
        })
        .collect::<String>();
    assert!(!token.verify(&flipped));
    assert!(InstanceToken::new(flipped).is_ok());
}
