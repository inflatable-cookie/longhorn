# Update And Licence Headless Proof (Card 159)

## Status

The packaged proof application is **deprioritized** (operator decision,
2026-08-08): it is more surface to maintain than the evidence it buys is
worth right now. The machine-bound claims — macOS bundle replacement and
relaunch (tauri#11392), interlock against a genuinely open transfer session,
credentials surviving a restart through a platform backend, an RFC 8252
sign-in through the system browser, and non-writable classification through
the real plugin — are recorded as **unmet** in the harness output, never as
passed.

- **Headless harness: complete and kept.** `rust/harness` proves every pure
  claim — update decision and rollout evaluation, the restart interlock gate
  against each deferral cause, licence signature verification and tamper
  rejection, both activation sources, usability windows and the clock guard,
  and credential slot round-trips — with no packaged application and no new
  dependencies. Run it with:

  ```sh
  cargo run -p longhorn-update-licence-proof
  ```

- **Concrete installer: landed.** `PluginUpdaterInstaller` in
  `longhorn-tauri-update` implements the `UpdateInstaller` port over
  `tauri-plugin-updater` 2.10.1 (Tauri graph unchanged at 2.11.5); see
  Card 153 for the recorded plugin limits.

Resume Card 159 when a consumer needs the packaged evidence: the app shell,
the keyring-backed `CredentialStore` composition, and the stub servers are
the remaining code.
