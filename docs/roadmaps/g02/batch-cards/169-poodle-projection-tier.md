# 169 Poodle Projection Tier

Status: in progress — first domain projected
Owner: Tom
Roadmap: g02.012 follow-up
Governing refs: contract 020; contract 013; research memo 021
Depends on: Card 163
Auto-start next card: no

## Objective

Give a GPUI application something to draw. `longhorn-gpui-windowing` opens and
places a window; nothing renders into it.

## Why this exists

Memo 021 named this and no card ever scheduled it:

> A GPUI host therefore needs a projection tier, not a parallel of
> `packages/*`.

Card 163 finished the host boundary and confirmed the memo's structural
claim: a GPUI application needs **one** host adapter where Tauri has eleven,
because nine of the Tauri crates are IPC handler assembly a GPUI application
never composes, and the rest are value boundaries that work unchanged.

What it does not have is the layer above. `longhorn-poodle-svelte` projects
Longhorn's domains — commands, config, history, notifications, operation,
settings — into Svelte components against Poodle. Its Rust sibling does not
exist, so today a GPUI product can place a window and must then hand-write
every projection itself, which is precisely the duplication a framework
exists to prevent.

## Structural questions — answered 2026-08-09, before code

Both questions this card said to settle first are settled, and one changed the
card's premise.

### The name is `longhorn-poodle`, not `longhorn-poodle-gpui`

Card 164 asserted the Rust sibling "will be `longhorn-poodle-gpui`". Measuring
Poodle says otherwise. `poodle-specs` — Poodle's shared Rust contract layer —
already has **two** renderers over it:

| Adapter | Lines | Maps specs to |
| --- | ---: | --- |
| `poodle-gpui` | 4,408 | GPUI elements |
| `poodle-jetstream` | 5,025 | Jetstream `UiTree` nodes |

Both describe themselves identically: "maps shared contract specs and resolved
styles to <renderer> nodes". So a Longhorn projection that emits **specs** is
renderer-neutral by construction, and works for Jetstream and anything after
it without change.

Naming it `-gpui` would bake one renderer into something that is not
renderer-specific, and would invite a second copy the first time Jetstream
needs the same projection. `longhorn-poodle` is the sibling of
`longhorn-poodle-svelte`, where the suffix names the *framework binding* — and
Card 164 freed exactly that name when it renamed the TypeScript package.

### The `gpui` dependency does not arise

This card feared the dependency it inherited from Card 163's compromise. It
does not apply. Poodle's own adapter is deliberately `gpui`-free — it emits
`GpuiStyle`, `GpuiColor`, `GpuiEdges`: GPUI-*shaped* plain data, with a
comment saying so outright — and only `packages/gpui/preview` builds real
elements.

So the chain is:

```text
Longhorn domains -> longhorn-poodle -> poodle-specs
                                          |-> poodle-gpui      -> gpui elements
                                          `-> poodle-jetstream -> UiTree nodes
```

`longhorn-poodle` depends on `poodle-specs` and nothing heavy. It is an
ordinary workspace crate, inside `effigy qa`, at no cost to any lane. The
"heavyweight host SDKs have no in-gate home" papercut is untouched by this
card — it remains a real problem for `prototypes/gpui-windowing` and is not
one this card has to solve.

This is the same shape twice over: `longhorn-gpui-windowing` takes no `gpui`
and pushes the binding to an excluded prototype; `poodle-gpui` takes no `gpui`
and pushes it to a preview. Two repositories reached it independently.

## Dependency mechanism — corrected 2026-08-09

This card was briefly recorded as blocked on the grounds that Longhorn had no
sanctioned way to depend on Poodle's Rust crates. That was wrong, and the
error was mine: I treated a cross-repo path dependency as disqualifying
without checking that Longhorn already uses one.

It does. `package.json` carries
`"@inflatable-cookie/poodle-core": "file:../poodle/.artifacts/…"`. A sibling
path reference is the established temporary shape on the npm side, and the
Rust side takes the same one:

```toml
poodle-specs = { version = "0.1.0", path = "../../../poodle/packages/contracts/components" }
```

**The long-term mechanism is git refs, not crates.io.** Poodle is untagged, so
path refs stand in until it carries a tag; at that point every consumer's path
ref swaps to a git ref together — npm and Cargo alike. Publishing to crates.io
was my recommendation and is not the plan.

### The CI consequence, stated

`ci.yml` says it "exists to prove a clean clone with no sibling checkouts, no
`[patch]` config, and no warm caches — the environment consumers of a tag are
in". A path ref contradicts that by construction, and so does the `file:` pin
that predates it: both jobs need a sibling Poodle to resolve.

So this crate does not make CI worse, it joins an existing gap — and the tag
that fixes it fixes both at once. Worth knowing rather than discovering during
a release.

## Scope

- `longhorn-poodle`: projections from Longhorn domains to Poodle specs,
  renderer-neutral
- the domains Card 163's first target actually needs, and no more
- a parity statement against `longhorn-poodle-svelte`, per domain

## Explicit non-goals

- Porting `packages/*`. The IPC tier does not exist here; nothing serializes.
  Memo 021 measured roughly 18,000 of ~24,000 TypeScript lines as boundary
  machinery that simply disappears.
- Forking Poodle's primitives. `poodle-gpui` renders; this projects onto it.
  Contract 013's rule holds — Longhorn ships the projection that drives the
  kit, and the kit stays generic.
- Feature parity with the Svelte tier. Parity is *stated per domain*, not
  assumed.

## Steps

1. ~~Decide the name~~ — done, see above: `longhorn-poodle`.
2. Pick the domains from the first GPUI target's actual needs — memo 021 says
   config, settings, operations, notifications, licence and update. Layout,
   surfaces and transfer are **not** in that list and should not be built
   speculatively.
3. Build one domain end to end first, and use it to find where the Svelte
   projections encode a webview assumption. Card 163's lesson is that the
   second implementation is what exposes the first one's hidden shape, and
   the projections have had exactly one implementation so far.
4. State parity per domain in a table: projected, deliberately absent, or not
   yet. "Not yet" and "will not" are different claims and must not blur.
5. Take the same dependency-direction discipline: this crate depends on
   `poodle-specs`, and Poodle keeps no reference to Longhorn.

## First domain — notifications, 2026-08-09

`longhorn-poodle` exists and projects `NotificationRecord` into Poodle's
`Toast`. Two tests, and it compiled against `poodle-specs` first try, which is
the useful signal: the two vocabularies mostly already agree.

### What building one domain exposed

**Longhorn has five severities; Poodle has four tones.** `Info`, `Success`,
`Warning` and `Error` map cleanly. `Critical` has nowhere to go — `Danger` is
as loud as Poodle gets — so it shares a tone with `Error`.

This is a real gap rather than an oversight on either side. Poodle's tones are
a *visual* vocabulary and four tints is a reasonable palette; Longhorn's
severities are an *operational* ladder and five levels is a reasonable ladder.
They simply do not line up, and no amount of care on either side would have
made them.

So `tone_for` returns a `ToneMapping` carrying `is_lossy`, true only for
`Critical`. A projection that returned the tone alone would make `Critical`
silently indistinguishable from `Error` at exactly the moment the distinction
matters. A surface that wants it back restores it in text or in an action,
because the tone cannot carry it.

**A toast holds one action; a record may carry several.** The first is
projected and the rest are reachable from the notification centre. That is a
presentation choice, not a loss of record, and it is stated here so nobody
reads it as a bug later.

Neither of these needed a Poodle change, so the stop condition did not fire.

**A path dependency widened the format gate into Poodle.** `cargo fmt --all`
walks every local package in the graph rather than the workspace, so it
reformatted `poodle-specs` under Longhorn's style edition and failed on files
that pass Poodle's own gate. `fmt:rust` now derives its package list from
`cargo metadata --no-deps`. Worth stating plainly: the fix task would have
written into a sibling checkout. The two GitHub workflows still carry the old
form and need the same change.

## Second domain — config restore, 2026-08-09

Restore inspection, chosen because it is the densest thing the config tier
shows: thirteen compatibility classifications, a per-domain choice, and an
evidence block. `longhorn-poodle::config` projects it into `RadioGroupSpec`
and `DetailItemSpec`. Six tests.

### The Svelte tier is three things wearing one name

The config projection is 1,247 lines of `.svelte` and 60 lines of `.ts`.
Reading it apart:

- **Fact to presentation.** `compatibilityLabel`, `canUseArchive`. Pure,
  framework-free, and about Longhorn's own enums. This is the projection.
- **Spec assembly.** Which options exist, which are disabled, which detail
  items appear in which order. Also projectable, and currently inlined in
  markup where it cannot be tested or reused.
- **Session and transport.** `client`, `onMount`, `$state`,
  `crypto.randomUUID()`. Genuinely host-shaped and correctly there.

Only the first two crossed. The third stays, and naming it is what makes the
tier's boundary decidable rather than a matter of taste.

### The webview assumption, found

`crypto.randomUUID()` is the default `nextRequestId` in all three pages. It is
a browser global. A GPUI application has no `crypto` object, so any Rust
counterpart must be handed an id source rather than reaching for one — which
is the same shape `longhorn-gpui-windowing` already uses for its backend seam.
The projection tier does not generate request ids, so nothing here needed it;
the finding is about what the Svelte pages assume, and it would have bitten a
literal port.

### Serde encodings are being used as UI text

`RestoreIntegrityProjection` and `RestoreAuthenticityProjection` are unit
enums. The Svelte page renders them straight — `value={inspection.integrity}`
— so the operator reads `"verified"` and `"unauthenticated"`, which are serde
`rename_all = "camelCase"` outputs, not chosen words. Rust holds the enum and
has to pick text, so `integrity_label` and `authenticity_label` exist.

Two consequences. Renaming a serde variant silently changes UI text in the
webview and changes nothing in Rust. And the two backends will not agree on
these strings until the Svelte side stops rendering the wire form. The Rust
side is the one that is right.

### One duplication that cannot be removed

`compatibility_label` now exists twice — once in Rust, once in
`restore-model.ts`. The Svelte tier cannot call Rust, so this is unavoidable
rather than sloppy, but it is a live parity risk: a fourteenth compatibility
variant means editing two label maps, and only the Rust one fails to compile
if it is missed. Recorded rather than solved; generating the TS map from the
Rust one is possible and is not this card's work.

### Poodle needed no change

`RadioGroupSpec.aria_label` has no `with_` builder while every other field
does, so it is assigned directly. The field exists, so this is an asymmetry
and not a gap, and the stop condition did not fire.

## Third domain — settings navigation, 2026-08-09

Settings was picked because it is the only domain with a `svelte/` directory
next to its `poodle/` one, which made it the sharpest test of whether the tier
has a second, framework-bound layer with no Rust answer. It does, and that
turns out to be correct rather than a problem.

`longhorn-poodle::settings` projects a sealed registry into `SidebarNavSpec`.
Five tests.

### 1,308 of 1,743 lines are not a projection

`settings/svelte/` is session, document and page runtime built on Svelte
runes — `$state`, `$derived`, reactive lifecycle. `settings/poodle/` is one
435-line shell. Only the shell's navigation derivation crossed, and only
because it is pure.

This is the clean case. The runtime is genuinely framework-bound, a GPUI
application will need its own, and the two will not resemble each other. That
is not a parity gap; it is what a projection tier is for. The card's earlier
worry — that the Svelte tier hides a webview assumption in its bones — is only
half right. It does, and the bones are correctly separated from the flesh.

### Domain logic sitting in the client tier

`projectSettingsRegistry` is in `packages/longhorn/src/settings/registry.ts`.
It is a pure function over Rust-generated types that states a Longhorn rule:
drop sections with no pages, then drop modules left with no sections. That is
a rule about Longhorn's own registry, in TypeScript, in the *client* tier
rather than the projection tier.

Same class as `compatibility_label` in the previous domain and a tier further
out. Rust is where the rule belongs; the TypeScript is a port that nothing
checks.

### One implicit dependency, now explicit

The TypeScript projection never sorts. It is correct only because
`SettingsRegistryBuilder::seal` sorts modules, sections, pages and anchors by
explicit `order` then by id. Nothing on the TypeScript side says so. The Rust
projection does not re-sort either, and its doc comment says why — a second
statement of the same ordering rule is a second thing that can drift.

### A divergence banked for whoever projects search

`searchSettingsRegistry` normalises with `toLocaleLowerCase("en-US")`. Rust's
`str::to_lowercase` is Unicode-default and locale-free. The two will not agree
on every input, so a Rust search projection has to state which one is right
rather than assume they match. Not projected here; recorded so it is not
discovered by a bug report.

### Poodle needed no change

`SidebarNavSpec`, `SidebarNavGroup` and `SidebarNavItem` carried everything
the shell uses. Stop condition did not fire.

## Fourth domain — operations, 2026-08-09

`operation/poodle/projectors.ts` is 82 lines, entirely pure, and the closest
thing in the whole Svelte tier to a projection with nothing mixed in. All of
it crossed. `longhorn-poodle::operation` emits `StatusIndicatorSpec` and
`ProgressSpec`. Six tests.

### The Svelte tier re-declares Poodle's specs as prop bundles

`OperationStatusTone` is a local string union with six members. Poodle's
`StatusTone` has the same six, including `Pending`. `OperationProgressView` is
an interface with `indeterminate`, `value`, `max`, `valueText`. Poodle's
`ProgressSpec` has exactly those fields.

Neither was invented because Poodle lacked something. They exist because a
Svelte projection hands *props to a component*, so it needs a prop-shaped
object, while a Rust projection emits the spec itself. The Rust module needs
no intermediate type at all — `progress()` returns `ProgressSpec` and the
function is the projection.

This is the structural difference between the two tiers, stated plainly for
the first time. It also means every re-declared shape is a place the two
backends can drift while both compile.

### A second collapse, and why this one is not lossy

Seven operation states into six tones: `Cancelling` and `Interrupted` are both
`Warning`. The notification domain flagged its five-into-four collapse as
lossy; this one is not, and the difference is real rather than a judgement
call. `status_indicator` always emits the label with the tone, so the two
states are never distinguished by colour alone. A toast tone can stand by
itself; a status indicator's cannot.

`Cancelled` is `Neutral` rather than `Danger`, because the operator asked for
it. Worth stating because it is the one mapping that looks wrong at a glance.

### Poodle needed no change

`StatusTone`, `StatusIndicatorSpec` and `ProgressSpec` carried everything.
Stop condition did not fire — four domains, zero Poodle changes.

## Fifth and sixth domains — licence and update, 2026-08-09

Both projected into `BannerSpec`. Eleven tests between them.

These two are different from the first four: **`longhorn-poodle-svelte` has no
licence projection and no update projection.** There is nothing to port and
nothing to compare, so neither produced a port-versus-source finding. They are
the first statement of their rules rather than the second.

### Rust has no free date formatting, and a webview does

`Timestamp` is Unix seconds and its `Display` prints the integer. That is
right for a log and a defect in a banner. A webview reaches for
`toLocaleString` and gets a localised date from the platform for nothing;
Rust has no equivalent without a date library *and* a locale, neither of which
belongs in a projection crate.

So `usability_banner` takes a `TimestampFormat`. Same shape as
`crypto.randomUUID()` in the config domain and the same underlying fact: the
browser is a platform as well as a renderer, and several things the Svelte
tier gets for free have to be injected on the Rust side. That is a seam, not a
gap — it is exactly how `longhorn-gpui-windowing` handles its backend.

### Silence is a projection decision, twice

`Usability::Active` and `InGrace` produce no banner, because
`warrants_attention` already says grace is deliberate quiet: a backend outage
must never look to a paying customer like a licensing failure. The projection
mirrors that rule rather than deciding again.

`UpdateAvailability::UpToDate` produces no banner either. "You are up to date"
on every launch is noise, and the check's result is available without one.

Returning `Option<BannerSpec>` rather than a neutral banner is what makes both
statements possible.

### The two states that look like a broken updater

`AheadOfChannel` and `ManagedElsewhere` are correct outcomes that a user reads
as a failure unless the surface explains them. `longhorn-update` gave each its
own variant instead of folding it into `UpToDate` precisely so a surface
could; this is the surface doing it.

`ManagedElsewhere` derives its instruction from
`InstallManager::upgrade_command`, which returns `None` for the App Store, an
AppImage and a distribution package. Those get a banner naming the owner and
no command, because inventing an instruction that does not work is worse than
admitting there is not one. This is the presentation half of the live defect
Card 168 found.

### Tone discipline across two domains

`Danger` means the software has stopped working. Licence uses it for every
unusable state. Update never uses it — a mandatory update is `Warning`,
everything else is `Info` — because the application still runs. Stated here
because the two domains are the first place the tones had to agree with each
other rather than only with themselves.

### Poodle needed no change

Six domains, zero Poodle changes. The stop condition has not fired once, which
is the strongest evidence so far that `poodle-specs` is a real contract layer
rather than a Svelte extraction wearing Rust.

## Parity against `longhorn-poodle-svelte`

The earlier version of this table listed only the six domains from step 2 and
read as though it covered the tier. It did not: `commands`, `history`,
`history-tree` and `native-content` were missing entirely, and `layout` was
marked absent from Rust without saying it is present in Svelte. Corrected.

| Domain | Svelte | Rust | Note |
| --- | --- | --- | --- |
| notifications | yes | **projected** | toasts; severity collapse recorded above |
| config | yes | **projected** | restore inspection: choices and evidence block |
| settings | yes | **projected** | sidebar navigation; page runtime stays framework-bound |
| operation | yes | **projected** | status tone, label, progress, cancel eligibility |
| licence | **no** | **projected** | no Svelte counterpart exists; Rust is first |
| update | **no** | **projected** | as above |
| commands | yes | not yet | out of scope for the first target |
| history | yes | not yet | out of scope for the first target |
| history-tree | yes | not yet | out of scope for the first target |
| layout | yes | **deliberately absent** | memo 021 excludes it; do not build speculatively |
| surfaces | yes | **deliberately absent** | as above |
| transfer | yes | **deliberately absent** | as above |
| native-content | yes | **will not** | see below |

`native-content` is the only "will not". `longhorn-native-content` already
exists and is host-agnostic, so the *coordination* is shared. What has no
analogue is the Svelte binding: `nativeContentViewport(node: HTMLElement)`
attaches an island to a DOM element, and a GPUI window has no DOM. A GPUI
application that needs a native island binds it some other way, and that
binding belongs to the host adapter, not to a projection.

"Not yet", "deliberately absent" and "will not" are three different claims and
stay separated. Out-of-scope domains are not failures; they are decisions from
memo 021 about the first GPUI target's actual needs.

## Acceptance Criteria

- [x] the crate name is a recorded decision, not an inherited assumption
- [x] one domain is projected — six are: notifications, config, settings,
  operation, licence, update. Rendering through an adapter is the one
  criterion still open, and it needs an application to render into.
- [x] parity is stated per domain, across the whole Svelte tier, with
  "deliberately absent" and "will not" both distinguished from "not yet"
- [x] no Poodle primitive is forked
- [x] Poodle contains no reference to Longhorn
- [x] the `gpui` dependency's home is decided: it does not arise, because the
  projection emits specs

## Evidence Required

- the projection for one domain, and a running window that draws it
- the parity table
- a statement of which renderers the projection was checked against

## Stop Conditions

- a Longhorn domain cannot be projected without changing a Poodle primitive,
  in which case the gap is raised in Poodle rather than forked here
- the first domain reveals that the Svelte projections encode webview
  assumptions deeply enough to need a contract 013 amendment, in which case
  that amendment is compiled before more domains are built

## Notes

Sequencing: this is the largest remaining GPUI piece and the only one
blocking a real GPUI product. Contract 020's unproven claims — cross-window
transfer, lifecycle teardown under load — stay unproven until an application
exists to exercise them, and this card is what makes such an application
possible.

## Next Task

Whatever the first domain reveals.
