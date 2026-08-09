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
its findings recorded, its quiescence contract landed, and
`longhorn-update` carrying the probes and the install gate (they lived in a
`longhorn-tauri-update` crate until 2026-08-09, which had no Tauri code).
Only the concrete `tauri-plugin-updater` installer remains, behind an
injected port, and it lands with Card 159.
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

[Card 159](g02/batch-cards/159-update-and-licence-packaged-proof.md)
completed its headless batch and then stopped on an operator decision
(2026-08-08): the packaged proof application is deprioritized, and Longhorn
does not implement an installer — Tauri's updater plugin installs. The
card's landed value is the authorize-only interlock
(`UpdateGate::authorize` in `longhorn-update`, pure, no tauri
dependency) plus the headless regression harness. The machine-bound claims
stay recorded as unmet. The card resumes when a consumer needs the packaged
evidence.

[Card 161](g02/batch-cards/161-host-tier-positioning-and-leak-fixes.md) is
complete: zero pure crates depend on a host adapter, and the webview-shaped
geometry in core is host-neutral.

[Card 162](g02/batch-cards/162-native-update-execution.md) and
[Card 163](g02/batch-cards/163-gpui-host-adapter.md) are ready and
independent of each other. Research memo 021 repositions Longhorn as a
framework with two first-class, permanent hosts — Tauri and GPUI. The
separation mostly exists already: 12.5% of the Rust is Tauri-bound, with one
pure-to-host dependency edge and one webview-shaped concept in core. Card
161 is documentation plus a type move and unblocks the rest.

[Card 160](g02/batch-cards/160-ipc-validation-derived-from-authority.md)
opens g02.011 and is ready. Its inventory found nine of thirteen packages
never validate keys at the IPC boundary, and 29 hardcoded bounds with no
link to the 55 Rust constants that define them. Step 2 alone — emitting the
bound constants — is separable and closes the only finding with a live drift
mechanism.

[Card 164](g02/batch-cards/164-typescript-package-consolidation.md) is
complete and closes g02.013: eighteen TypeScript packages are three, grouped
by peer requirement. 61 entry points became 62 and nothing that resolved
stopped resolving.

All five consumers are migrated and green — nucleus, loophole, soundcheck,
jetstream, and figmatic, which had been missing from the inventory because it
aliased Longhorn by path and is now normalised onto `file:` dependencies like
the rest. Card 149's receipt still needs regenerating against the new counts
once it is unblocked.

[Card 165](g02/batch-cards/165-artifact-proof-selection-model.md) is complete
and closes g02.013. `proof:artifacts` is green across all twelve proofs, from
one before Card 164. The Poodle evidence pin now derives from the root
manifest, and `forbidden` was split into install-absence for packages and
import-absence for subpaths — the old check could only pass vacuously once a
domain became a subpath. Contract 012's Acceptance list is restated to match.

[Card 166](g02/batch-cards/166-first-publication.md) opens g02.014 and is the
live next step. The `@inflatable-cookie` scope was claimed on 2026-08-09, which
satisfies contract 012's working-names gate, so publication is no longer
deferred. The ordering is forced: Poodle publishes, Longhorn repoints off
`file:` packs, Longhorn's CI TypeScript lane runs for the first time, Longhorn
publishes, v0.1.0 is tagged, and twenty-four consumer manifests move to
versions. One decision has to be taken before the repoint — the artifact proofs
derive their pin from the `file:` pack reference that the repoint removes.

Rust is not part of it. Every crate sets `publish = false` and consumers take
them by git tag, so the Rust half of v0.1.0 needs only the tag.

Also outstanding: Card 149's receipt freeze remains operator-held on
consumer manifest quiescence, and the
[g02 candidate runway](g02/README.md#candidate-runway) still tiers the
deferred options. Package-manager publication remains deferred.

Next live pointer: the g02 planning checkpoint — characterize the next
shared gap from consumer evidence, or extend the runway.
