# Loophole Storage And Domain Transition

Date: 2026-08-02
Time: 01:25:29 Europe/London
Card: 105
Roadmap: g01.015

## Result

Loophole now selects Longhorn `shared-product-root-v1` through one
consumer-owned `echo-storage-profile` crate:

- canonical application id: `com.inflatablecookie.loophole`
- stable storage leaf: `Loophole`
- fixed locator: canonical-id native config root under `.longhorn`
- portable bypass: explicit `LOOPHOLE_USERDATA`

Aura, embedded Pulse, local brokered Pulse, Spark, and Echo profile config use
that selection. Remote Pulse remains server/project authority and does not
turn server data into client config.

## Transition

Aura startup inspects recovery before service mutation. Missing locators use
one of two deterministic sources:

- established `Loophole` root: adopt in place, including typed config-to-state
  reclassification
- old Tauri canonical-id root: import only when the established root contains
  no recognized durable domain

Custom adapters capture JSON files and the profile tree, validate payloads,
stage targets, refuse conflicting occupied targets, verify semantic evidence,
and let Longhorn commit the locator last. The consumer receipt binds identity,
profile, layout digest, transition digest, retained roots, unknown files, and
explicit exclusions. Source cleanup is not automatic.

Soundcheck owns the only admitted live SQLite database. Project documents,
Pulse persistence, autosaves, journals, media, and server data remain outside
the app-profile transition.

## Domains

| Class | Domains |
| --- | --- |
| user config | active profile, app settings, profiles/workspaces, recent projects, plugin presets, hosting policy, plugin preferences, renderer preferences |
| machine state | windowing, machine settings, audio/MIDI bindings, plugin-editor reopen state |
| excluded cache | plugin library projection |
| excluded incomplete state | recording recovery |
| external | Soundcheck database, Pulse projects/journals/media, remote server data |

The renderer imports five legacy WebView keys once. Production mounting waits
for the registered Longhorn domain. Later reads and writes use the host only.
Legacy WebView values remain retained but inactive.

## Longhorn Correction

Same-layout locator adoption previously reported false overlaps when the
derived workspace root nested beneath state. Identical source/target layout
digests now bypass cross-layout root conflicts. A regression test covers the
Loophole shared-product shape.

## Validation

- Longhorn storage layout suite: 20 passed
- Aura native suite: 204 passed
- Aura renderer suite: 1,031 passed
- Aura Svelte check: zero diagnostics
- Echo, Pulse, and Spark Effigy builds: passed
- consumer migration proofs: established-root adoption and old-Tauri import

## Next Task

Execute Card 106. Replace Loophole display observation, window planning, and
native Tauri application while preserving product policy.
