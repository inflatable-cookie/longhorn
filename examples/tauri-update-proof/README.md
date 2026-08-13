# Tauri Update Proof

Scaffold, not yet a proof. **Deliberately not a workspace member**, so an
incomplete host cannot fail `effigy qa`. Add it to `Cargo.toml`'s `members`
when `src/main.rs` lands.

## What it is for

Card 159's last two claims, and the only two left in g02.009. Both need a
packaged application that genuinely runs, quits and comes back — every other
claim on the card was reachable by a binary that inspects a filesystem, and
those are done.

1. **Relaunch, and the tauri#11392 finding under Longhorn's close handling.**
   `packaged-update-proof` records relaunch as "unmet by design — relaunch is
   the host's". That division is correct and is not this finding: the question
   is whether `prevent_close` interferes, and a host that never relaunches
   cannot answer it.
2. **The restart interlock against a genuinely open transfer session**, rather
   than the `BusyProbe` the packaged proof uses today. The *ordering* — gate
   after the transfer, before the install — is already proved. What is not
   proved is that a real session reports itself.

## What is here

- `Cargo.toml`, `build.rs`, `tauri.conf.json`, `capabilities/update-proof.json`
- `frontend/index.html`: three operator controls — open a transfer session,
  close it, attempt an install — and an evidence pane.

## What is missing, and the one thing that is not obvious

`src/main.rs`. The composition it needs is mostly settled:

- `TransferCoordinator::session_count()` is the real signal claim 2 wants, and
  `transfer_session_probe(|| coordinator.session_count())` wires it straight
  into `UpdateGate`. No test double anywhere in that path.
- `UpdateController` composes as it does in `packaged-update-proof`: a
  `StaticJsonSource`, an `ArtifactFetch`, an `ArtifactKey`, and
  `NativeInstaller`.

The awkward part is **opening a session that is genuinely open**.
`TransferCoordinator::publish_lease` takes a `MonotonicClock` and a
`LeasePublication` of seven parts — window id, client id, client epoch, lease
generation, lifetime, window bounds, drop zones. That is a real composition
rather than a call, and it is the reason this file exists instead of a working
host: writing it badly would produce a session that satisfies the type system
and not the claim, which is exactly the failure claim 2 is meant to close.

## Convention

Follows `examples/tauri-windowing-proof`. Evidence is recorded, not gated —
these are claims CI cannot make.
