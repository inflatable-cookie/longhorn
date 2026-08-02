# Soundcheck Storage, Config, And Window Cutover

Date: 2026-08-02
Roadmap: g01.016
Card: 115
State: complete; Card 116 ready

## Result

Soundcheck now uses Longhorn for its stable-name storage layout, application
settings persistence, display/placement state, and protected primary-window
host. The cutover preserves `com.inflatablecookie.soundcheck` as canonical
identity, `Soundcheck` as the explicit storage leaf, and the existing
product-root `library.db` placement.

Legacy settings import is backup-first, digest-receipted, conflict-refusing,
locator-last, source-retaining, and cleanup-forbidden. Application settings
and window placement no longer share one document. SQLite participates through
its native online backup API without moving schema or recovery authority out
of soundcheck-library.

The `main` Tauri window is hidden until protected `window:primary` restoration
and renderer readiness both complete. Display fitting, minimum size, event
capture, debounce, and bounded close flush now use the shared host. The old
settings-owned geometry worker was removed.

## Generic Gap

The established Soundcheck product root contains substantial unrelated data.
Same-layout profile adoption previously walked that data even though no file
authority changed. Longhorn now omits unknown source inventory when source and
target layout digests match. A regression keeps retained product data outside
the transition preview.

## Exact Evidence

- prior Longhorn admission: `821eea31f00f779cedcb8dcdb6cfc882dd651115`
- Longhorn cutover support: `ab9cb31a70611a0714b02296016a22f0ae58a615`
- prior Soundcheck: `f4544e621b8ba3f4558c6b068db1cb36d6ef161f`
- Soundcheck cutover: `c2351a9f7f8de3a5a16ca633f4172ddb10f4665e`
- Soundcheck lock SHA-256:
  `ca50d24dba1355320820f90b7d2a5d798e5b058b7ab17ceeab2b93a0b984a178`
- fixture: `fixtures/migration/soundcheck-card115/storage-config-window-cutover-v1.json`
- verifier: `effigy proof:soundcheck-card115`

## Validation

Soundcheck passed renderer build and 28 renderer tests. New focused Rust tests
passed: three storage-profile, three legacy migration, three application
settings, and one window-domain case. The Rust suite passed 223 tests before
the pre-existing unrelated
`soundcheck-sync daw_discovery::tests::detects_each_installed_major_version_for_supported_daws`
failure: its fixture expects `RecoveryCapable`, while current donor behavior is
`WriteCapable`. The touched source does not include that subsystem.

Longhorn `longhorn-config --all-features` passes: 38 unit, 61 domain-store,
and 20 storage-layout tests. Full native GUI fresh/restart and rollback proof
is intentionally reserved for Card 119; Card 115 used isolated roots and did
not mutate the operator's live Soundcheck data.

## Next

Execute Card 116. Replace the bespoke central settings controller with one
shared modal registry/shell, keeping product pages and soundcheck-library
backup semantics downstream.
