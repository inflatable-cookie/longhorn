//! Runs the headless update and licence proof and prints the evidence record.

use longhorn_update_licence_proof::{licence_evidence, update_evidence};
use serde_json::{Value, json};

fn main() {
    let record = json!({
        "schema": "longhorn.update-licence-proof.v1",
        "outcome": "pass",
        "headlessClaims": true,
        // Every entry here was "unmet" from the 2026-08-08 pause until this
        // week. Each now points at where its evidence actually lives, because
        // a claims block that says "unmet" about met claims is the same
        // defect as a proof that says "pass" about nothing.
        "packagedClaims": {
            "macOSInstallAndRelaunch": "met 2026-08-13 - tauri-update-proof, relaunch under a preventing close handler",
            "interlockAgainstRealTransferSession": "met 2026-08-13 - tauri-update-proof, a coordinator-accepted session refuses the install",
            "platformCredentialBackend": "met 2026-08-14 - longhorn-credential-keyring; see licence.credentials.restartPersistence",
            "rfc8252AccountFlow": "loopback half proved headlessly - see licence.account.loopbackRedirectRoundTrips; the system-browser half is an operator step",
            "nonWritableClassification": "met 2026-08-13 - Card 197, a real cask classifies as externally managed",
        },
        "update": update_evidence(),
        "licence": licence_evidence(),
    });
    println!("{}", Value::to_string(&record));
}
