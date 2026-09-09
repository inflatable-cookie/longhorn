# Flattened-Task Migration Manifest

Status: archived record — not executable. Frozen from the pre-migration
inventory and the exact deletion set (`git diff 89896870 f3067f2a
--name-only --diff-filter=D`: 261 paths), committed as the reviewable
preservation record for the migration that removed them. Git history is the
full-fidelity archive.

## Classification

- g01: safely closed. Generation README, 20 task files, 137 card records plus
  the batch-cards index — all complete. Deferred items already rehomed:
  publication to g02.014, candidate receipt to g02.008, scale/platform
  evidence to the g02 candidate runway.
- g02: active. 103 card records absorbed into owning tasks; no new IDs except
  g02.037 (Card 177 had no owning task).
- Unresolved generations: none.

## Preservation

- Unique authority: g01 rules restate contracts 001-017; no live-only rule
  found. Destinations in `archive/g01.md`.
- Open commitments, all reachable: Card 149 blocked in g02.008; Card 166
  ready in g02.014; Card 192 in progress in g02.020; Card 218 needs recompile
  in g02.026; Card 177 in progress in g02.037; stage 3 deferred in g02.017;
  stage 2 external in g02.019.
- Evidence: batch logs in `docs/logs/`; donor pins and receipts cited in
  `archive/g01.md`; per-card absorbed records in each g02 task file.

## Removed paths and destinations

| Removed path | Destination |
| --- | --- |
| `docs/roadmaps/g01/001-foundation-contracts-and-package-topology.md` | archive/g01.md (g01.001 collapsed) |
| `docs/roadmaps/g01/002-configuration-backup-and-recovery.md` | archive/g01.md (g01.002 collapsed) |
| `docs/roadmaps/g01/003-display-geometry-and-window-planning.md` | archive/g01.md (g01.003 collapsed) |
| `docs/roadmaps/g01/004-tauri-window-host-and-lifecycle.md` | archive/g01.md (g01.004 collapsed) |
| `docs/roadmaps/g01/005-layout-container-region-and-panel-core.md` | archive/g01.md (g01.005 collapsed) |
| `docs/roadmaps/g01/006-optional-surfaces-and-cross-window-drag.md` | archive/g01.md (g01.006 collapsed) |
| `docs/roadmaps/g01/007-typescript-svelte-poodle-and-app-shell.md` | archive/g01.md (g01.007 collapsed) |
| `docs/roadmaps/g01/008-settings-registry-and-shell.md` | archive/g01.md (g01.008 collapsed) |
| `docs/roadmaps/g01/009-typed-bridge-and-optional-backend-topology.md` | archive/g01.md (g01.009 collapsed) |
| `docs/roadmaps/g01/010-command-registry-keymaps-and-palette.md` | archive/g01.md (g01.010 collapsed) |
| `docs/roadmaps/g01/011-history-kernel-and-branching-prototype.md` | archive/g01.md (g01.011 collapsed) |
| `docs/roadmaps/g01/012-async-operations-and-notifications.md` | archive/g01.md (g01.012 collapsed) |
| `docs/roadmaps/g01/013-native-content-islands-prototype.md` | archive/g01.md (g01.013 collapsed) |
| `docs/roadmaps/g01/014-nucleus-no-surface-migration.md` | archive/g01.md (g01.014 collapsed) |
| `docs/roadmaps/g01/015-loophole-full-hosting-migration.md` | archive/g01.md (g01.015 collapsed) |
| `docs/roadmaps/g01/016-secondary-consumers-and-greenfield-release.md` | archive/g01.md (g01.016 collapsed) |
| `docs/roadmaps/g01/017-optional-forkable-history-tree.md` | archive/g01.md (g01.017 collapsed) |
| `docs/roadmaps/g01/018-native-content-production-and-adoption-gate.md` | archive/g01.md (g01.018 collapsed) |
| `docs/roadmaps/g01/019-grouped-custom-adapter-restore.md` | archive/g01.md (g01.019 collapsed) |
| `docs/roadmaps/g01/020-child-view-navigation-control.md` | archive/g01.md (g01.020 collapsed) |
| `docs/roadmaps/g01/README.md` | archive/g01.md (generation roll-up) |
| `docs/roadmaps/g01/batch-cards/001-configuration-domain-store.md` | archive/g01.md (card 001 collapsed) |
| `docs/roadmaps/g01/batch-cards/002-coordinated-atomic-configuration-mutation.md` | archive/g01.md (card 002 collapsed) |
| `docs/roadmaps/g01/batch-cards/003-debounced-mutation-and-explicit-flush.md` | archive/g01.md (card 003 collapsed) |
| `docs/roadmaps/g01/batch-cards/004-backup-archive-and-restore-contract.md` | archive/g01.md (card 004 collapsed) |
| `docs/roadmaps/g01/batch-cards/005-backup-inventory-and-consistent-snapshot.md` | archive/g01.md (card 005 collapsed) |
| `docs/roadmaps/g01/batch-cards/006-backup-archive-publication-and-retention.md` | archive/g01.md (card 006 collapsed) |
| `docs/roadmaps/g01/batch-cards/007-restore-inspection-planning-and-staging.md` | archive/g01.md (card 007 collapsed) |
| `docs/roadmaps/g01/batch-cards/008-journaled-restore-and-crash-recovery.md` | archive/g01.md (card 008 collapsed) |
| `docs/roadmaps/g01/batch-cards/009-age-encrypted-backup-adapter.md` | archive/g01.md (card 009 collapsed) |
| `docs/roadmaps/g01/batch-cards/010-custom-backup-adapters-and-consumer-conformance.md` | archive/g01.md (card 010 collapsed) |
| `docs/roadmaps/g01/batch-cards/011-platform-storage-layout-profiles.md` | archive/g01.md (card 011 collapsed) |
| `docs/roadmaps/g01/batch-cards/012-storage-profile-transition-and-legacy-import.md` | archive/g01.md (card 012 collapsed) |
| `docs/roadmaps/g01/batch-cards/013-typed-coordinate-and-geometry-foundation.md` | archive/g01.md (card 013 collapsed) |
| `docs/roadmaps/g01/batch-cards/014-display-inventory-and-correlation.md` | archive/g01.md (card 014 collapsed) |
| `docs/roadmaps/g01/batch-cards/015-window-placement-resolution.md` | archive/g01.md (card 015 collapsed) |
| `docs/roadmaps/g01/batch-cards/016-live-window-diff-planning.md` | archive/g01.md (card 016 collapsed) |
| `docs/roadmaps/g01/batch-cards/017-tauri-display-and-live-window-observation.md` | archive/g01.md (card 017 collapsed) |
| `docs/roadmaps/g01/batch-cards/018-tauri-window-operation-execution.md` | archive/g01.md (card 018 collapsed) |
| `docs/roadmaps/g01/batch-cards/019-window-event-attribution-and-settling.md` | archive/g01.md (card 019 collapsed) |
| `docs/roadmaps/g01/batch-cards/020-tauri-window-capture-reveal-and-flush.md` | archive/g01.md (card 020 collapsed) |
| `docs/roadmaps/g01/batch-cards/021-tauri-window-host-composition-and-mock-proof.md` | archive/g01.md (card 021 collapsed) |
| `docs/roadmaps/g01/batch-cards/022-packaged-window-host-proof-and-closeout.md` | archive/g01.md (card 022 collapsed) |
| `docs/roadmaps/g01/batch-cards/023-layout-identity-policy-and-normalization.md` | archive/g01.md (card 023 collapsed) |
| `docs/roadmaps/g01/batch-cards/024-authoritative-layout-mutation-engine.md` | archive/g01.md (card 024 collapsed) |
| `docs/roadmaps/g01/batch-cards/025-registered-layout-persistence-and-coordination.md` | archive/g01.md (card 025 collapsed) |
| `docs/roadmaps/g01/batch-cards/026-generated-layout-typescript-protocol.md` | archive/g01.md (card 026 collapsed) |
| `docs/roadmaps/g01/batch-cards/027-two-shape-layout-conformance-and-closeout.md` | archive/g01.md (card 027 collapsed) |
| `docs/roadmaps/g01/batch-cards/028-surface-identity-topology-and-resolution.md` | archive/g01.md (card 028 collapsed) |
| `docs/roadmaps/g01/batch-cards/029-authoritative-surface-lifecycle-and-persistence.md` | archive/g01.md (card 029 collapsed) |
| `docs/roadmaps/g01/batch-cards/030-surface-window-host-composition-and-conformance.md` | archive/g01.md (card 030 collapsed) |
| `docs/roadmaps/g01/batch-cards/031-bounded-transfer-sessions-and-drop-zone-leases.md` | archive/g01.md (card 031 collapsed) |
| `docs/roadmaps/g01/batch-cards/032-authoritative-layout-panel-transfer.md` | archive/g01.md (card 032 collapsed) |
| `docs/roadmaps/g01/batch-cards/033-whole-surface-transfer-and-window-provisioning.md` | archive/g01.md (card 033 collapsed) |
| `docs/roadmaps/g01/batch-cards/034-generated-transfer-protocol-and-tauri-host.md` | archive/g01.md (card 034 collapsed) |
| `docs/roadmaps/g01/batch-cards/035-packaged-multi-window-transfer-proof-and-closeout.md` | archive/g01.md (card 035 collapsed) |
| `docs/roadmaps/g01/batch-cards/036-client-lifecycle-and-domain-free-tauri-transport.md` | archive/g01.md (card 036 collapsed) |
| `docs/roadmaps/g01/batch-cards/037-svelte-reactive-client-state.md` | archive/g01.md (card 037 collapsed) |
| `docs/roadmaps/g01/batch-cards/038-poodle-public-drag-seam-and-preview-artifact.md` | archive/g01.md (card 038 collapsed) |
| `docs/roadmaps/g01/batch-cards/039-poodle-layout-bindings.md` | archive/g01.md (card 039 collapsed) |
| `docs/roadmaps/g01/batch-cards/040-cross-window-drag-and-titlebar-actions.md` | archive/g01.md (card 040 collapsed) |
| `docs/roadmaps/g01/batch-cards/041-three-shape-app-shell-proof-and-closeout.md` | archive/g01.md (card 041 collapsed) |
| `docs/roadmaps/g01/batch-cards/042-settings-identity-registry-and-authority-protocol.md` | archive/g01.md (card 042 collapsed) |
| `docs/roadmaps/g01/batch-cards/043-config-backed-settings-apply-units.md` | archive/g01.md (card 043 collapsed) |
| `docs/roadmaps/g01/batch-cards/044-generated-settings-protocol-and-tauri-host.md` | archive/g01.md (card 044 collapsed) |
| `docs/roadmaps/g01/batch-cards/045-svelte-settings-session-and-poodle-shell.md` | archive/g01.md (card 045 collapsed) |
| `docs/roadmaps/g01/batch-cards/046-storage-profile-diagnostics-and-backup-pages.md` | archive/g01.md (card 046 collapsed) |
| `docs/roadmaps/g01/batch-cards/047-restore-conflict-and-recovery-settings-flow.md` | archive/g01.md (card 047 collapsed) |
| `docs/roadmaps/g01/batch-cards/048-settings-composition-proof-and-closeout.md` | archive/g01.md (card 048 collapsed) |
| `docs/roadmaps/g01/batch-cards/049-bridge-identity-negotiation-and-authority-protocol.md` | archive/g01.md (card 049 collapsed) |
| `docs/roadmaps/g01/batch-cards/050-typed-operations-streams-and-job-lifecycle.md` | archive/g01.md (card 050 collapsed) |
| `docs/roadmaps/g01/batch-cards/051-generated-bridge-client-and-direct-conformance.md` | archive/g01.md (card 051 collapsed) |
| `docs/roadmaps/g01/batch-cards/052-tauri-bridge-host-and-client-assembly.md` | archive/g01.md (card 052 collapsed) |
| `docs/roadmaps/g01/batch-cards/053-reconnect-retry-and-injected-supervision.md` | archive/g01.md (card 053 collapsed) |
| `docs/roadmaps/g01/batch-cards/054-five-shape-topology-conformance.md` | archive/g01.md (card 054 collapsed) |
| `docs/roadmaps/g01/batch-cards/055-bridge-topology-artifact-proof-and-closeout.md` | archive/g01.md (card 055 collapsed) |
| `docs/roadmaps/g01/batch-cards/056-command-registry-context-and-argument-foundation.md` | archive/g01.md (card 056 collapsed) |
| `docs/roadmaps/g01/batch-cards/057-fresh-availability-and-injected-execution-admission.md` | archive/g01.md (card 057 collapsed) |
| `docs/roadmaps/g01/batch-cards/058-deterministic-keyboard-and-keymap-resolution.md` | archive/g01.md (card 058 collapsed) |
| `docs/roadmaps/g01/batch-cards/059-config-backed-keymaps-and-generated-host-protocol.md` | archive/g01.md (card 059 collapsed) |
| `docs/roadmaps/g01/batch-cards/060-command-clients-svelte-sessions-and-poodle-projections.md` | archive/g01.md (card 060 collapsed) |
| `docs/roadmaps/g01/batch-cards/061-command-system-artifact-proof-and-closeout.md` | archive/g01.md (card 061 collapsed) |
| `docs/roadmaps/g01/batch-cards/062-lossless-history-donor-fixtures-and-typed-foundation.md` | archive/g01.md (card 062 collapsed) |
| `docs/roadmaps/g01/batch-cards/063-transactional-linear-navigation-and-failure-invariance.md` | archive/g01.md (card 063 collapsed) |
| `docs/roadmaps/g01/batch-cards/064-history-coalescing-grouping-retention-and-projections.md` | archive/g01.md (card 064 collapsed) |
| `docs/roadmaps/g01/batch-cards/065-history-persistence-and-committed-transition-stream.md` | archive/g01.md (card 065 collapsed) |
| `docs/roadmaps/g01/batch-cards/066-generated-history-client-tauri-svelte-and-poodle.md` | archive/g01.md (card 066 collapsed) |
| `docs/roadmaps/g01/batch-cards/067-linear-history-artifact-proof-and-checkpoint.md` | archive/g01.md (card 067 collapsed) |
| `docs/roadmaps/g01/batch-cards/068-private-forkable-history-tree-prototype.md` | archive/g01.md (card 068 collapsed) |
| `docs/roadmaps/g01/batch-cards/069-forkable-history-promotion-decision-and-closeout.md` | archive/g01.md (card 069 collapsed) |
| `docs/roadmaps/g01/batch-cards/070-fork-tree-identity-topology-and-branches.md` | archive/g01.md (card 070 collapsed) |
| `docs/roadmaps/g01/batch-cards/071-fork-tree-navigation-retention-and-checkpoints.md` | archive/g01.md (card 071 collapsed) |
| `docs/roadmaps/g01/batch-cards/072-dense-fork-tree-persistence-and-migration.md` | archive/g01.md (card 072 collapsed) |
| `docs/roadmaps/g01/batch-cards/073-fork-tree-clients-and-bounded-projections.md` | archive/g01.md (card 073 collapsed) |
| `docs/roadmaps/g01/batch-cards/074-fork-tree-artifact-proof-and-closeout.md` | archive/g01.md (card 074 collapsed) |
| `docs/roadmaps/g01/batch-cards/075-operation-donor-fixtures-identity-and-lifecycle-authority.md` | archive/g01.md (card 075 collapsed) |
| `docs/roadmaps/g01/batch-cards/076-operation-progress-cancellation-retention-and-teardown.md` | archive/g01.md (card 076 collapsed) |
| `docs/roadmaps/g01/batch-cards/077-generated-operation-protocol-and-tauri-bridge-composition.md` | archive/g01.md (card 077 collapsed) |
| `docs/roadmaps/g01/batch-cards/078-operation-svelte-session-and-poodle-projection.md` | archive/g01.md (card 078 collapsed) |
| `docs/roadmaps/g01/batch-cards/079-retained-notification-ledger-and-operation-observation.md` | archive/g01.md (card 079 collapsed) |
| `docs/roadmaps/g01/batch-cards/080-notification-clients-svelte-poodle-and-semantic-actions.md` | archive/g01.md (card 080 collapsed) |
| `docs/roadmaps/g01/batch-cards/081-operation-and-notification-artifact-proof-and-closeout.md` | archive/g01.md (card 081 collapsed) |
| `docs/roadmaps/g01/batch-cards/082-private-native-content-coordination-prototype.md` | archive/g01.md (card 082 collapsed) |
| `docs/roadmaps/g01/batch-cards/083-child-webview-mechanism-packaged-prototype.md` | archive/g01.md (card 083 collapsed) |
| `docs/roadmaps/g01/batch-cards/084-isolated-native-window-packaged-prototype.md` | archive/g01.md (card 084 collapsed) |
| `docs/roadmaps/g01/batch-cards/085-backing-surface-mechanism-packaged-prototype.md` | archive/g01.md (card 085 collapsed) |
| `docs/roadmaps/g01/batch-cards/086-native-content-promotion-decision-and-closeout.md` | archive/g01.md (card 086 collapsed) |
| `docs/roadmaps/g01/batch-cards/087-native-content-identity-state-and-planning.md` | archive/g01.md (card 087 collapsed) |
| `docs/roadmaps/g01/batch-cards/088-generated-native-content-client-and-host-protocol.md` | archive/g01.md (card 088 collapsed) |
| `docs/roadmaps/g01/batch-cards/089-tauri-child-view-production-adapter.md` | archive/g01.md (card 089 collapsed) |
| `docs/roadmaps/g01/batch-cards/090-isolated-window-production-adapter.md` | archive/g01.md (card 090 collapsed) |
| `docs/roadmaps/g01/batch-cards/091-backing-surface-production-adapter.md` | archive/g01.md (card 091 collapsed) |
| `docs/roadmaps/g01/batch-cards/092-svelte-native-content-session-and-layout-seam.md` | archive/g01.md (card 092 collapsed) |
| `docs/roadmaps/g01/batch-cards/093-native-content-artifact-proof-and-closeout.md` | archive/g01.md (card 093 collapsed) |
| `docs/roadmaps/g01/batch-cards/094-nucleus-behavior-authority-and-rollback-freeze.md` | archive/g01.md (card 094 collapsed) |
| `docs/roadmaps/g01/batch-cards/095-nucleus-private-artifact-and-cutover-admission.md` | archive/g01.md (card 095 collapsed) |
| `docs/roadmaps/g01/batch-cards/096-nucleus-storage-and-legacy-import.md` | archive/g01.md (card 096 collapsed) |
| `docs/roadmaps/g01/batch-cards/097-nucleus-protected-window-host-cutover.md` | archive/g01.md (card 097 collapsed) |
| `docs/roadmaps/g01/batch-cards/098-nucleus-project-layout-authority-cutover.md` | archive/g01.md (card 098 collapsed) |
| `docs/roadmaps/g01/batch-cards/099-nucleus-renderer-and-poodle-cutover.md` | archive/g01.md (card 099 collapsed) |
| `docs/roadmaps/g01/batch-cards/100-nucleus-native-browser-cutover.md` | archive/g01.md (card 100 collapsed) |
| `docs/roadmaps/g01/batch-cards/101-nucleus-migration-conformance-and-closeout.md` | archive/g01.md (card 101 collapsed) |
| `docs/roadmaps/g01/batch-cards/102-loophole-behavior-authority-and-rollback-freeze.md` | archive/g01.md (card 102 collapsed) |
| `docs/roadmaps/g01/batch-cards/103-loophole-storage-policy-and-profile-selection.md` | archive/g01.md (card 103 collapsed) |
| `docs/roadmaps/g01/batch-cards/104-loophole-baseline-and-private-artifact-admission.md` | archive/g01.md (card 104 collapsed) |
| `docs/roadmaps/g01/batch-cards/105-loophole-storage-and-domain-transition.md` | archive/g01.md (card 105 collapsed) |
| `docs/roadmaps/g01/batch-cards/106-loophole-display-and-window-host-cutover.md` | archive/g01.md (card 106 collapsed) |
| `docs/roadmaps/g01/batch-cards/107-loophole-registered-layout-authority-cutover.md` | archive/g01.md (card 107 collapsed) |
| `docs/roadmaps/g01/batch-cards/108-loophole-surface-lifecycle-and-hosting-cutover.md` | archive/g01.md (card 108 collapsed) |
| `docs/roadmaps/g01/batch-cards/109-loophole-renderer-poodle-and-transfer-cutover.md` | archive/g01.md (card 109 collapsed) |
| `docs/roadmaps/g01/batch-cards/110-loophole-settings-command-and-keyboard-cutover.md` | archive/g01.md (card 110 collapsed) |
| `docs/roadmaps/g01/batch-cards/111-loophole-linear-history-adoption.md` | archive/g01.md (card 111 collapsed) |
| `docs/roadmaps/g01/batch-cards/112-loophole-migration-conformance-and-closeout.md` | archive/g01.md (card 112 collapsed) |
| `docs/roadmaps/g01/batch-cards/113-secondary-consumer-behavior-authority-and-rollback-freeze.md` | archive/g01.md (card 113 collapsed) |
| `docs/roadmaps/g01/batch-cards/114-secondary-consumer-private-artifact-admission.md` | archive/g01.md (card 114 collapsed) |
| `docs/roadmaps/g01/batch-cards/115-soundcheck-storage-config-and-window-cutover.md` | archive/g01.md (card 115 collapsed) |
| `docs/roadmaps/g01/batch-cards/116-soundcheck-settings-backup-and-recovery-cutover.md` | archive/g01.md (card 116 collapsed) |
| `docs/roadmaps/g01/batch-cards/117-soundcheck-plugin-scan-operation-adoption.md` | archive/g01.md (card 117 collapsed) |
| `docs/roadmaps/g01/batch-cards/118-soundcheck-isolated-window-coordination-cutover.md` | archive/g01.md (card 118 collapsed) |
| `docs/roadmaps/g01/batch-cards/119-soundcheck-migration-conformance-and-closeout.md` | archive/g01.md (card 119 collapsed) |
| `docs/roadmaps/g01/batch-cards/120-split-shell-config-and-settings-cutover.md` | archive/g01.md (card 120 collapsed) |
| `docs/roadmaps/g01/batch-cards/121-split-shell-minimal-composition-conformance-and-closeout.md` | archive/g01.md (card 121 collapsed) |
| `docs/roadmaps/g01/batch-cards/122-jetstream-bridge-command-and-keyboard-cutover.md` | archive/g01.md (card 122 collapsed) |
| `docs/roadmaps/g01/batch-cards/123-jetstream-backing-surface-coordination-cutover.md` | archive/g01.md (card 123 collapsed) |
| `docs/roadmaps/g01/batch-cards/124-jetstream-migration-conformance-and-closeout.md` | archive/g01.md (card 124 collapsed) |
| `docs/roadmaps/g01/batch-cards/125-greenfield-composition-matrix.md` | archive/g01.md (card 125 collapsed) |
| `docs/roadmaps/g01/batch-cards/126-api-storage-composition-and-migration-guides.md` | archive/g01.md (card 126 collapsed) |
| `docs/roadmaps/g01/batch-cards/127-private-0-1-compatibility-candidate-and-closeout.md` | archive/g01.md (card 127 collapsed) |
| `docs/roadmaps/g01/batch-cards/128-grouped-adapter-restore-contract-and-protocol.md` | archive/g01.md (card 128 collapsed) |
| `docs/roadmaps/g01/batch-cards/129-grouped-adapter-restore-execution-and-journal.md` | archive/g01.md (card 129 collapsed) |
| `docs/roadmaps/g01/batch-cards/130-grouped-adapter-recovery-and-conformance.md` | archive/g01.md (card 130 collapsed) |
| `docs/roadmaps/g01/batch-cards/131-grouped-adapter-public-evidence-and-nucleus-handoff.md` | archive/g01.md (card 131 collapsed) |
| `docs/roadmaps/g01/batch-cards/132-child-view-navigation-contract-and-adapter-authority.md` | archive/g01.md (card 132 collapsed) |
| `docs/roadmaps/g01/batch-cards/133-tauri-child-view-navigation-execution-and-packaged-proof.md` | archive/g01.md (card 133 collapsed) |
| `docs/roadmaps/g01/batch-cards/134-child-view-navigation-artifact-closeout-and-consumer-handoffs.md` | archive/g01.md (card 134 collapsed) |
| `docs/roadmaps/g01/batch-cards/135-grouped-adapter-explicit-state-contract.md` | archive/g01.md (card 135 collapsed) |
| `docs/roadmaps/g01/batch-cards/136-grouped-adapter-absence-transaction-and-recovery.md` | archive/g01.md (card 136 collapsed) |
| `docs/roadmaps/g01/batch-cards/137-grouped-adapter-absence-conformance-and-nucleus-handoff.md` | archive/g01.md (card 137 collapsed) |
| `docs/roadmaps/g01/batch-cards/README.md` | superseded index; per-record destinations apply |
| `docs/roadmaps/g02/batch-cards/138-layout-ratio-serde-and-schema-caps.md` | g02/001-*.md (absorbed record; open scope inline where applicable) |
| `docs/roadmaps/g02/batch-cards/139-event-loop-flush-deferral-and-timer-wakes.md` | g02/002-*.md (absorbed record; open scope inline where applicable) |
| `docs/roadmaps/g02/batch-cards/140-retag-coherence-install-safety-and-race-closure.md` | g02/002-*.md (absorbed record; open scope inline where applicable) |
| `docs/roadmaps/g02/batch-cards/141-consumed-abort-truth-and-reconciliation-evidence.md` | g02/003-*.md (absorbed record; open scope inline where applicable) |
| `docs/roadmaps/g02/batch-cards/142-client-binding-races-and-ordered-events.md` | g02/003-*.md (absorbed record; open scope inline where applicable) |
| `docs/roadmaps/g02/batch-cards/143-async-storage-commands-and-lock-waiting.md` | g02/004-*.md (absorbed record; open scope inline where applicable) |
| `docs/roadmaps/g02/batch-cards/144-restore-self-heal-and-poison-consistency.md` | g02/004-*.md (absorbed record; open scope inline where applicable) |
| `docs/roadmaps/g02/batch-cards/145-diagnostics-seam-and-swallow-instrumentation.md` | g02/005-*.md (absorbed record; open scope inline where applicable) |
| `docs/roadmaps/g02/batch-cards/146-qa-selectors-and-package-hygiene.md` | g02/006-*.md (absorbed record; open scope inline where applicable) |
| `docs/roadmaps/g02/batch-cards/147-front-door-truth-sweep.md` | g02/006-*.md (absorbed record; open scope inline where applicable) |
| `docs/roadmaps/g02/batch-cards/148-dependency-refresh-sweep.md` | g02/007-*.md (absorbed record; open scope inline where applicable) |
| `docs/roadmaps/g02/batch-cards/149-distribution-candidate-v2.md` | g02/008-*.md (absorbed record; open scope inline where applicable) |
| `docs/roadmaps/g02/batch-cards/150-store-schema-stamping-and-forward-refusal.md` | g02/009-*.md (absorbed record; open scope inline where applicable) |
| `docs/roadmaps/g02/batch-cards/151-update-policy-channels-and-rollout.md` | g02/009-*.md (absorbed record; open scope inline where applicable) |
| `docs/roadmaps/g02/batch-cards/152-update-source-adapters.md` | g02/009-*.md (absorbed record; open scope inline where applicable) |
| `docs/roadmaps/g02/batch-cards/153-restart-interlock-and-tauri-install.md` | g02/009-*.md (absorbed record; open scope inline where applicable) |
| `docs/roadmaps/g02/batch-cards/154-update-client-surface.md` | g02/009-*.md (absorbed record; open scope inline where applicable) |
| `docs/roadmaps/g02/batch-cards/155-licence-model-and-entitlement-evaluation.md` | g02/010-*.md (absorbed record; open scope inline where applicable) |
| `docs/roadmaps/g02/batch-cards/156-activation-source-adapters.md` | g02/010-*.md (absorbed record; open scope inline where applicable) |
| `docs/roadmaps/g02/batch-cards/157-tauri-licence-host-and-secure-storage.md` | g02/010-*.md (absorbed record; open scope inline where applicable) |
| `docs/roadmaps/g02/batch-cards/158-licence-client-surface.md` | g02/010-*.md (absorbed record; open scope inline where applicable) |
| `docs/roadmaps/g02/batch-cards/159-update-and-licence-packaged-proof.md` | g02/009 + g02/010 (shared proof, recorded in both) |
| `docs/roadmaps/g02/batch-cards/160-ipc-validation-derived-from-authority.md` | g02/011-*.md (absorbed record; open scope inline where applicable) |
| `docs/roadmaps/g02/batch-cards/161-host-tier-positioning-and-leak-fixes.md` | g02/012-*.md (absorbed record; open scope inline where applicable) |
| `docs/roadmaps/g02/batch-cards/162-native-update-execution.md` | g02/012-*.md (absorbed record; open scope inline where applicable) |
| `docs/roadmaps/g02/batch-cards/163-gpui-host-adapter.md` | g02/012-*.md (absorbed record; open scope inline where applicable) |
| `docs/roadmaps/g02/batch-cards/164-typescript-package-consolidation.md` | g02/013-*.md (absorbed record; open scope inline where applicable) |
| `docs/roadmaps/g02/batch-cards/165-artifact-proof-selection-model.md` | g02/013-*.md (absorbed record; open scope inline where applicable) |
| `docs/roadmaps/g02/batch-cards/166-first-publication.md` | g02/014-*.md (absorbed record; open scope inline where applicable) |
| `docs/roadmaps/g02/batch-cards/167-publication-disclosure-readiness.md` | g02/014-*.md (absorbed record; open scope inline where applicable) |
| `docs/roadmaps/g02/batch-cards/168-installation-provenance.md` | g02/012-*.md (absorbed record; open scope inline where applicable) |
| `docs/roadmaps/g02/batch-cards/169-poodle-projection-tier.md` | g02/012-*.md (absorbed record; open scope inline where applicable) |
| `docs/roadmaps/g02/batch-cards/170-generated-projection-labels.md` | g02/012-*.md (absorbed record; open scope inline where applicable) |
| `docs/roadmaps/g02/batch-cards/171-cross-backend-projection-parity.md` | g02/012-*.md (absorbed record; open scope inline where applicable) |
| `docs/roadmaps/g02/batch-cards/172-gpui-build-cadence.md` | g02/015-*.md (absorbed record; open scope inline where applicable) |
| `docs/roadmaps/g02/batch-cards/173-gpui-composition-guide.md` | g02/015-*.md (absorbed record; open scope inline where applicable) |
| `docs/roadmaps/g02/batch-cards/174-gpui-worked-example.md` | g02/015-*.md (absorbed record; open scope inline where applicable) |
| `docs/roadmaps/g02/batch-cards/175-live-cross-window-drag.md` | g02/015-*.md (absorbed record; open scope inline where applicable) |
| `docs/roadmaps/g02/batch-cards/176-live-teardown-under-load.md` | g02/015-*.md (absorbed record; open scope inline where applicable) |
| `docs/roadmaps/g02/batch-cards/178-layout-container-provisioning.md` | contracts 002/014 (g02 README collapsed records) |
| `docs/roadmaps/g02/batch-cards/179-surfaces-absorb-containers.md` | contracts 002/014 (g02 README collapsed records) |
| `docs/roadmaps/g02/batch-cards/180-pre-release-simplification-audit.md` | contracts 012 (g02 README collapsed records) |
| `docs/roadmaps/g02/batch-cards/181-fork-history-workaround-removal.md` | g02/016-*.md (absorbed record; open scope inline where applicable) |
| `docs/roadmaps/g02/batch-cards/182-host-supplied-recorded-at.md` | g02/016-*.md (absorbed record; open scope inline where applicable) |
| `docs/roadmaps/g02/batch-cards/183-node-centric-fork-projection.md` | g02/016-*.md (absorbed record; open scope inline where applicable) |
| `docs/roadmaps/g02/batch-cards/184-checkout-continuation.md` | g02/016-*.md (absorbed record; open scope inline where applicable) |
| `docs/roadmaps/g02/batch-cards/185-delete-one-fork.md` | g02/017-*.md (absorbed record; open scope inline where applicable) |
| `docs/roadmaps/g02/batch-cards/186-retention-that-can-prune.md` | g02/017-*.md (absorbed record; open scope inline where applicable) |
| `docs/roadmaps/g02/batch-cards/187-generated-variant-field-maps.md` | g02/018-*.md (absorbed record; open scope inline where applicable) |
| `docs/roadmaps/g02/batch-cards/188-replace-inline-variant-keys.md` | g02/018-*.md (absorbed record; open scope inline where applicable) |
| `docs/roadmaps/g02/batch-cards/190-update-protocol-surface.md` | g02/009-*.md (absorbed record; open scope inline where applicable) |
| `docs/roadmaps/g02/batch-cards/191-name-the-origin.md` | g02/019-*.md (absorbed record; open scope inline where applicable) |
| `docs/roadmaps/g02/batch-cards/192-bind-the-settings-shell.md` | g02/020-*.md (absorbed record; open scope inline where applicable) |
| `docs/roadmaps/g02/batch-cards/193-licence-protocol-surface.md` | g02/010-*.md (absorbed record; open scope inline where applicable) |
| `docs/roadmaps/g02/batch-cards/194-strictness-for-the-lenient-eight.md` | g02/018-*.md (absorbed record; open scope inline where applicable) |
| `docs/roadmaps/g02/batch-cards/195-make-the-rule-a-check.md` | g02/018-*.md (absorbed record; open scope inline where applicable) |
| `docs/roadmaps/g02/batch-cards/196-longhorn-is-the-update-controller.md` | g02/009-*.md (absorbed record; open scope inline where applicable) |
| `docs/roadmaps/g02/batch-cards/197-cask-detection-is-backwards.md` | g02/009-*.md (absorbed record; open scope inline where applicable) |
| `docs/roadmaps/g02/batch-cards/198-the-proofs-nobody-can-run.md` | g02/009-*.md (absorbed record; open scope inline where applicable) |
| `docs/roadmaps/g02/batch-cards/199-which-machines-hold-a-seat.md` | g02/010-*.md (absorbed record; open scope inline where applicable) |
| `docs/roadmaps/g02/batch-cards/200-extraction-that-is-bounded.md` | g02/021-*.md (absorbed record; open scope inline where applicable) |
| `docs/roadmaps/g02/batch-cards/201-endpoint-authority-parsing.md` | g02/021-*.md (absorbed record; open scope inline where applicable) |
| `docs/roadmaps/g02/batch-cards/202-install-atomicity-and-recovery.md` | g02/021-*.md (absorbed record; open scope inline where applicable) |
| `docs/roadmaps/g02/batch-cards/203-escalation-contract-and-downgrade-binding.md` | g02/021-*.md (absorbed record; open scope inline where applicable) |
| `docs/roadmaps/g02/batch-cards/204-event-scoping-decision.md` | g02/022-*.md (absorbed record; open scope inline where applicable) |
| `docs/roadmaps/g02/batch-cards/205-session-teardown-and-entropy.md` | g02/022-*.md (absorbed record; open scope inline where applicable) |
| `docs/roadmaps/g02/batch-cards/206-example-hardening-and-acl-proof.md` | g02/022-*.md (absorbed record; open scope inline where applicable) |
| `docs/roadmaps/g02/batch-cards/207-activation-payload-safety.md` | g02/023-*.md (absorbed record; open scope inline where applicable) |
| `docs/roadmaps/g02/batch-cards/208-secret-hygiene-and-tamper-truth.md` | g02/023-*.md (absorbed record; open scope inline where applicable) |
| `docs/roadmaps/g02/batch-cards/209-pkce-generation-and-loopback-robustness.md` | g02/023-*.md (absorbed record; open scope inline where applicable) |
| `docs/roadmaps/g02/batch-cards/210-age-identity-persistence.md` | g02/023-*.md (absorbed record; open scope inline where applicable) |
| `docs/roadmaps/g02/batch-cards/211-port-the-layout-suites.md` | g02/024-*.md (absorbed record; open scope inline where applicable) |
| `docs/roadmaps/g02/batch-cards/212-licence-update-fixtures-and-seam-strings.md` | g02/024-*.md (absorbed record; open scope inline where applicable) |
| `docs/roadmaps/g02/batch-cards/213-fuzz-the-three-parsers.md` | g02/024-*.md (absorbed record; open scope inline where applicable) |
| `docs/roadmaps/g02/batch-cards/214-port-parity-and-keyring-coverage.md` | g02/024-*.md (absorbed record; open scope inline where applicable) |
| `docs/roadmaps/g02/batch-cards/215-post-179-architecture-sweep.md` | g02/025-*.md (absorbed record; open scope inline where applicable) |
| `docs/roadmaps/g02/batch-cards/216-front-door-truth-and-register-freshness.md` | g02/025-*.md (absorbed record; open scope inline where applicable) |
| `docs/roadmaps/g02/batch-cards/217-guide-repair.md` | g02/025-*.md (absorbed record; open scope inline where applicable) |
| `docs/roadmaps/g02/batch-cards/218-linked-poodle-exit-gate.md` | g02/026-*.md (absorbed record; open scope inline where applicable) |
| `docs/roadmaps/g02/batch-cards/219-gate-hygiene-and-single-sourcing.md` | g02/026-*.md (absorbed record; open scope inline where applicable) |
| `docs/roadmaps/g02/batch-cards/220-supply-chain-visibility.md` | g02/026-*.md (absorbed record; open scope inline where applicable) |
| `docs/roadmaps/g02/batch-cards/221-native-content-generation-hoist.md` | g02/027-*.md (absorbed record; open scope inline where applicable) |
| `docs/roadmaps/g02/batch-cards/222-shared-primitives-and-idiom-codification.md` | g02/027-*.md (absorbed record; open scope inline where applicable) |
| `docs/roadmaps/g02/batch-cards/223-dependency-sweep-and-measured-costs.md` | g02/027-*.md (absorbed record; open scope inline where applicable) |
| `docs/roadmaps/g02/batch-cards/224-credential-store-conditional-write.md` | g02/023-*.md (absorbed record; open scope inline where applicable) |
| `docs/roadmaps/g02/batch-cards/225-consumer-scoped-credential-slot-value.md` | g02/028-*.md (absorbed record; open scope inline where applicable) |
| `docs/roadmaps/g02/batch-cards/226-mixed-scale-desktop-mapping.md` | contract 009 (g02 README collapsed records) |
| `docs/roadmaps/g02/batch-cards/227-agent-control-spike.md` | g02/029-*.md (absorbed record; open scope inline where applicable) |
| `docs/roadmaps/g02/batch-cards/228-agent-control-core-crate.md` | g02/030-*.md (absorbed record; open scope inline where applicable) |
| `docs/roadmaps/g02/batch-cards/229-agent-control-stateless-server.md` | g02/030-*.md (absorbed record; open scope inline where applicable) |
| `docs/roadmaps/g02/batch-cards/230-agent-control-tauri-plugin.md` | g02/031-*.md (absorbed record; open scope inline where applicable) |
| `docs/roadmaps/g02/batch-cards/231-agent-control-capture.md` | g02/031-*.md (absorbed record; open scope inline where applicable) |
| `docs/roadmaps/g02/batch-cards/232-agent-control-webview-shim.md` | g02/032-*.md (absorbed record; open scope inline where applicable) |
| `docs/roadmaps/g02/batch-cards/233-agent-control-semantic-tools.md` | g02/032-*.md (absorbed record; open scope inline where applicable) |
| `docs/roadmaps/g02/batch-cards/234-agent-control-end-to-end-proof.md` | g02/032-*.md (absorbed record; open scope inline where applicable) |
| `docs/roadmaps/g02/batch-cards/235-agent-control-guide-and-skill.md` | g02/033-*.md (absorbed record; open scope inline where applicable) |
| `docs/roadmaps/g02/batch-cards/236-agent-control-skill-install-and-drift.md` | g02/033-*.md (absorbed record; open scope inline where applicable) |
| `docs/roadmaps/g02/batch-cards/237-agent-control-skill-dogfood-proof.md` | g02/033-*.md (absorbed record; open scope inline where applicable) |
| `docs/roadmaps/g02/batch-cards/238-agent-control-screenshot-composition.md` | g02/034-*.md (absorbed record; open scope inline where applicable) |
| `docs/roadmaps/g02/batch-cards/239-agent-control-webview-targeting.md` | g02/035-*.md (absorbed record; open scope inline where applicable) |
| `docs/roadmaps/g02/batch-cards/240-agent-control-child-drive-proof.md` | g02/035-*.md (absorbed record; open scope inline where applicable) |
| `docs/roadmaps/g02/batch-cards/README.md` | superseded index; per-record destinations apply |
