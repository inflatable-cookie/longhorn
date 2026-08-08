//! Runs the headless update and licence proof and prints the evidence record.

use longhorn_update_licence_proof::{licence_evidence, update_evidence};
use serde_json::{Value, json};

fn main() {
    let record = json!({
        "schema": "longhorn.update-licence-proof.v1",
        "outcome": "pass",
        "headlessClaims": true,
        "packagedClaims": {
            "macOSInstallAndRelaunch": "unmet - packaged proof deprioritized",
            "interlockAgainstRealTransferSession": "unmet - packaged proof deprioritized",
            "platformCredentialBackend": "unmet - composition, not built",
            "rfc8252AccountFlow": "unmet - packaged proof deprioritized",
            "nonWritableClassification": "unmet - plugin has no typed error",
        },
        "update": update_evidence(),
        "licence": licence_evidence(),
    });
    println!("{}", Value::to_string(&record));
}
