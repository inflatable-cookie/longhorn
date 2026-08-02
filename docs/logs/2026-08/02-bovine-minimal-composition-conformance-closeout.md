# Bovine Minimal Composition Conformance Closeout

Date: 2026-08-02
Roadmap: g01.016
Card: 121
State: complete; Card 122 ready

## Result

Bovine closes as Longhorn's minimal real consumer. It resolves four
TypeScript packages, six Rust crates, and five public Poodle packages. It has
no shared display/window/layout hierarchy and no Surface, transfer, bridge,
command, history, operation, notification, or native-content edge.

The macOS debug app ran twice against one confined acceptance root. Canonical
workspace root, ratio, `pathway:cat-fia` expansion and selection, and an
unknown product field survived import and restart. Legacy source and backup
are byte-identical. The receipt remains immutable while a valid versioned
target may evolve after import; corrupt evolution enters recovery instead of
being mistaken for a legitimate settings change.

The modal settings component now mounts one settings session per opening.
Close stops its listeners; reopen starts a fresh session and retains the
applied ratio. This fixed a real listener leak and a stopped-session remount
failure found during conformance.

## Evidence

- Bovine cutover: `6afc8da9f4ccc2989541312560eaeb4a154057d2`
- Bovine closeout: `8b3c19e8d1e96ce81d1dbbaf4769c9e72648dd28`
- previous authority: `b713644e08be384d20879b0ac31f7899589c8a9b`
- closeout fixture:
  `fixtures/migration/bovine-card121/conformance-closeout-v1.json`
- private artifact fixture:
  `fixtures/migration/bovine-card121/private-artifact-admission-v1.json`
- verifier: `effigy proof:bovine-card121`

Native GUI control could not attach deterministically because installed and
debug bundles share the same bundle identifier. The native launch/restart path
and actual settings component behavior are proved separately; the unavailable
click-through path is not claimed.

## Validation

Bovine `effigy qa` passes with 63 Rust tests, one renderer conformance test,
and zero Svelte errors or warnings. Production renderer build and debug macOS
bundle pass. The previous `b713644` workspace suite passes seven tests from a
temporary detached worktree, then that worktree is removed.

Private artifact proof passes outside workspace resolution with one Svelte
runtime. No package was published. No live app storage changed. Bovine's 19
unrelated docs/CHANGELOG paths and authored content remain untouched.

## Next

Execute Card 122. Cut Jetstream's editor-state projection, command discovery,
and keyboard resolution over while retaining engine and execution authority.
