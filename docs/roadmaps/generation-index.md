# Generation Index

## Mode

Dependency-led. New generations start only after a shared Longhorn gap is
characterized and promoted through the docs spine.

## Active Generation

[g02](g02/README.md) — workspace integrity remediation, now open past it.
Research memo 018 characterizes the post-g01 audit gap; Cards 138-147
delivered all six remediation milestones. Research memo 019 characterizes
the in-app update gap and compiles contract 018; g02.009 (Cards 150-154) is
the first non-remediation milestone. Research memo 020 compiles contract 019
over licensing as g02.010 (Cards 155-158).

## Rollover History

- [g01](g01/README.md) — complete, 20 milestones, Cards 001-137

## Next Task

[Card 150](g02/batch-cards/150-store-schema-stamping-and-forward-refusal.md)
is complete: every store's future-schema refusal is now proved
non-destructive, and one shared classification in `longhorn-core` lets the
update surface recognise a channel rejoin without matching per-store errors.

[Card 151](g02/batch-cards/151-update-policy-channels-and-rollout.md) is
complete: `longhorn-update` carries channel, manifest, rollout, and deferral
policy as a pure crate.

[Card 152](g02/batch-cards/152-update-source-adapters.md) is complete: the
`UpdateSource` trait plus static-JSON, GitHub-releases, and object-storage
adapters, with private GitHub documented as needing a consumer proxy.

[Card 153](g02/batch-cards/153-restart-interlock-and-tauri-install.md) has
its mechanism findings recorded and its quiescence contract landed. Its
remaining half — `longhorn-tauri-update` host wiring — needs a packaged
proof application, because install and relaunch cannot be exercised
headlessly and tauri#11392 puts the relaunch path specifically in doubt.
[Card 154](g02/batch-cards/154-update-client-surface.md) follows it.

[Card 155](g02/batch-cards/155-licence-model-and-entitlement-evaluation.md)
is complete: `longhorn-licence` carries the licence shape, trust basis,
entitlements, both windows, lease and grace.

[Card 156](g02/batch-cards/156-activation-source-adapters.md) is complete:
`ActivationSource`, the signed-file and token-redemption adapters, and the
licence-key format helpers.

[Card 157](g02/batch-cards/157-tauri-licence-host-and-secure-storage.md) has
its pure half complete — PKCE, callback validation, machine identity, and
the credential seam — and reached its documented stop condition on platform
storage, so the seam is injected and the decision is recorded.

[Card 159](g02/batch-cards/159-update-and-licence-packaged-proof.md) is
ready and is the next thing to run. Cards 153 and 157 both stopped at claims
that cannot be verified headlessly — macOS install and relaunch, a real
keychain, a real browser redirect — so the proof application they both need
is now its own card.

The v0.1.0 tag is blocked on a poodle release; g02.009 is independent of it.

Also outstanding: Card 149's receipt freeze remains operator-held on
consumer manifest quiescence, and the
[g02 candidate runway](g02/README.md#candidate-runway) still tiers the
deferred options. Package-manager publication remains deferred.
