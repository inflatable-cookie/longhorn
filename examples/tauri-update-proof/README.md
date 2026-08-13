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

## Claim 1: the relaunch harness

Built, and **only an operator can complete it** — it needs a packaged build, a
window, and a human to press the control.

The contributing factor tauri#11392 names is reproduced rather than imported:
the window event handler calls `api.prevent_close()` on every close request,
which is what Longhorn's windowing host does when its lifecycle receipt reports
a user close. Reproduced rather than composed because the mechanism is what the
issue is about, and pulling `longhorn-tauri-windowing` in would add a lifecycle
model to a question that does not need one.

`request_relaunch` uses `request_restart`, not `restart`. The first triggers
`ExitRequested` and `Exit` reliably; the second skips them when called on the
main thread, which would answer an easier question than the one asked — and
`ExitRequested` is the path a close handler could interfere with.

Evidence has to survive the process, so it is a marker file in the app data
directory: written immediately before the request, read and cleared on the next
start. Reaching the read at all is the finding. An in-memory flag cannot
measure the thing that destroys it.

### To complete it

```sh
cargo tauri build --config examples/tauri-update-proof/src-tauri/tauri.conf.json
```

Then:

1. Open the built application.
2. Press **Request relaunch**.
3. If the application comes back, the evidence pane reports
   `relaunchClaim: met` with the request it recovered. If it does not come
   back, that silence is the finding tauri#11392 predicts, and the marker is
   left on disk as proof the request was made.

Either outcome is a result. The card's stop condition already provides for the
second: if relaunch cannot be made reliable under Longhorn's close handling,
the finding is the deliverable and the interlock gains a documented
manual-relaunch path.

### Not wired

The install path — `StaticJsonSource`, `ArtifactFetch`, `ArtifactKey`,
`NativeInstaller` — composes as it does in `packaged-update-proof`. It is
deliberately absent: relaunch is the claim, the install is only its setup, and
`packaged-update-proof` already proves the replacement against a real bundle.

## Convention

Follows `examples/tauri-windowing-proof`. Evidence is recorded, not gated —
these are claims CI cannot make.

Its README documents `effigy proof-windowing-build`, which is not a task in
`effigy.toml`. This one gives the `cargo tauri` invocation directly rather than
inheriting a command that does not exist.
