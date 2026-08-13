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

Claim 2, and the mechanism for it, with no double anywhere in the path. The
gate reads `TransferCoordinator::session_count()` through
`transfer_session_probe`, and the session it counts is one the coordinator
accepted through its own validation.

Run it and use the three controls: open a session, close it, attempt an
install. Two unit tests make the same claims without the window, so a
regression is caught by `cargo test` rather than only by a human opening the
app.

**A lease is not a session, and that distinction cost a build.** The first
version published a lease — client epoch bound, drop zone, lifetime, all
accepted — and `session_count()` stayed at zero. A lease advertises where a
transfer *could* land; a session is a transfer actually in flight, and the
interlock is about work in flight. `create_session` with a real
`TransferSourceAuthority` is what opens one.

That is exactly the failure this claim exists to close: a session that
satisfies the type system and not the coordinator would leave the gate
approving and the proof green. The assertion `opened == 1` is what caught it,
and it is why the tests assert the count rather than only the refusal.

## What is missing

**Claim 1: relaunch, and the tauri#11392 finding.** `attempt_install` reports
the gate's answer and records `relaunchClaim` as not yet exercised.
`packaged-update-proof` performs the replacement; what is still owed is a host
that quits and comes back under Longhorn's close handling, and an explicit
finding on whether `prevent_close` interferes.

The install path itself — `StaticJsonSource`, `ArtifactFetch`, `ArtifactKey`,
`NativeInstaller` — composes as it does in `packaged-update-proof` and is not
yet wired here, because relaunch is the claim and the install is only its
setup.

## Convention

Follows `examples/tauri-windowing-proof`. Evidence is recorded, not gated —
these are claims CI cannot make.
