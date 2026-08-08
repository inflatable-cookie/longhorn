# Svelte Settings Session And Poodle Shell

Date: 2026-07-29
Card: g01.008 / 045
Status: complete

## Outcome

Added optional Svelte session state and one public-Poodle settings shell without
changing registry, configuration, or product authority.

## Session

`@inflatable-cookie/longhorn-settings/svelte` provides per-instance:

- listener-first registry and lazy scope lifetime
- deterministic route, search, and structural deep-link state
- consumer renderer-key resolution before reveal
- staged drafts plus immediate mutation status
- apply, cancel, reset confirmation, conflict, recovery, and activation state
- dirty page-switch and close guards
- exact listener and late-registration teardown

One page-level Apply accepts one dirty failure-atomic unit. Multiple dirty
units remain explicit instead of implying cross-domain atomicity. Conflict
accepts fresh authority and preserves the local draft.

## Shell

`@inflatable-cookie/longhorn-settings/poodle` exposes one controller-driven shell with modal,
independent-window, and routed-panel hosts. It uses public `@inflatable-cookie/poodle-svelte`
components for dialog, navigation, search, page headers, status, actions, and
surface presentation.

Consumer snippets retain product form state, copy, validation, reset scope,
and opaque intent codecs. Structural anchor navigation focuses a bound shell
target without DOM search.

## Evidence

- eleven session tests cover isolation, guards, conflict, immediate and scope
  failure, activation, missing renderers, reconnect, ambiguity, and late work
- eight mounted shell tests cover all host forms, dirty close, search/focus,
  conflict, failed save, reset confirmation, and guarded reveal
- one SSR import test covers both optional entry points without browser globals
- thirteen framework-neutral protocol/client/package tests remain green
- TypeScript and Svelte checks
- package dry run and public-Poodle source audit
- exact Card 038 preview verification
- full Effigy QA

All pass. The god-file scan remains at the 59-finding baseline with no new
settings file.

## Limits

- consumer snippets own field validation and product wording
- one built-in page Apply cannot claim atomicity across multiple units
- reset entry selection remains consumer-owned
- shared storage, backup, restore, and recovery pages remain Cards 046-047
