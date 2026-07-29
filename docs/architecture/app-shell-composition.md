# App Shell Composition

Status: promoted
Owner: Tom
Updated: 2026-07-29
Contracts: `../contracts/011-cross-window-transfer.md`,
`../contracts/012-distribution-and-compatibility.md`,
`../contracts/013-svelte-and-poodle-adapter-lifecycle.md`

## Boundary

Longhorn supplies state, host integration, actions, and composition guidance.
Poodle supplies tokens, presentation context, controls, loading, status, and
layout visuals. Consumers own app frames, navigation, panel catalogues,
product labels, errors, and capability policy.

There is no shared `AppShell` component. A small document app and a full
multi-window workspace should not render or depend on the same frame.

## Bootstrap Order

Use this order per window:

1. import public Poodle token CSS and the selected theme artifact
2. create the public theme controller and presentation provider
3. install event listeners before requesting current authority
4. load and validate every required authoritative snapshot
5. accept snapshots into per-window Longhorn state
6. mount the selected consumer composition
7. wait for the Svelte render boundary
8. ask the host to reveal the window

Never reveal from a fabricated renderer default. A missing initial snapshot,
protocol mismatch, missing capability, or host error remains a visible state.

Theme selection and persistence are product settings. Proofs disable Poodle's
local persistence and select one public theme explicitly. Production apps
should route persisted selection through their registered configuration
domain.

## State Surfaces

| State | UI rule | Authority rule |
| --- | --- | --- |
| loading | public Poodle loading presentation | no app frame revealed yet |
| ready | consumer composition inside `UiPresentationProvider` | current checked snapshots only |
| reconnecting | visible Poodle pending callout; last snapshot may remain | projection cannot become write authority |
| unsupported | visible capability name and reason | no silent local fallback |
| failed | visible error and retry route owned by consumer | preserve typed host evidence |

Destruction stops connections, cancels armed sessions, releases leases, clears
optimism, and removes timers. Late listener registration still unlistens once.

## Composition Profiles

### Minimal

Bovine proves a small split composition. It uses the generic Svelte client
lifetime and public Poodle `SplitView`. It has no layout, Surface, or transfer
package.

### Surface-free workspace

Nucleus proves:

```text
window -> layout container -> region -> panel
```

It uses registered layout authority and public Longhorn/Poodle region
bindings. It has no Surface or Surface-transfer package. A transfer package
may be added later for direct-window panel movement without changing that
rule.

### Full workspace

Loophole proves:

```text
display -> window -> Surface -> layout container -> region -> panel
```

It composes Surface state, layout state, panel transfer, whole-Surface
transfer capability, public Poodle bindings, and guarded reveal. Product
studio policy and panel bodies remain consumer-owned.

## Capabilities

Declare the smallest host surface for the selected profile:

- minimal: core window behavior only
- Surface-free: event lifetime, window behavior, and registered layout
  commands supplied by the app host
- full: base transfer plus optional Surface transfer and event lifetime

Capability files are examples, not a universal manifest. Window labels and
product commands remain consumer inputs. A capability expected by the shell
must be diagnosed at startup and rendered when absent.

## Transfer And UI Authority

Poodle owns drag interaction and visual affordances. Longhorn owns prepared
session ids, strict payload parsing, leases, target resolution, and
authoritative commit. Payloads contain only protocol version and host-issued
session id.

Do not inspect Poodle classes, generated DOM ids, private MIME values, or
component source. Do not serialize panels, Surfaces, layout documents, host
bindings, windows, or product data into native drag payloads.

## Artifact Proof

[`examples/app-shell-proof`](../../examples/app-shell-proof/README.md) installs
each profile outside workspace resolution. The proof rejects source aliases,
unexpected optional packages, duplicate Svelte runtimes, broad capability
graphs, and changed Poodle artifact membership.

The proof covers mounted macOS-oriented Tauri compositions with mock host
transports. It makes no Windows or Linux packaged-runtime claim.
