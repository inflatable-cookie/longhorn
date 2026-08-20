# 239 Agent Control Webview Targeting

Status: done 2026-08-20
Owner: Longhorn maintainers
Roadmap: g02.035
Governing refs: contract 022 (amended 2026-08-20 — the opt-in rule this
card implements); the Figmatic triage note; contracts 001, 006, 012;
Cards 232-233 closeouts (shim, marshalling), Card 238 closeout
(per-webview capture mechanics)
Depends on: g02.034 complete
Auto-start next card: yes — Card 240 in the same lane

## Objective

The contract's new rule becomes mechanism: semantic and input tools can
address an opted-in child webview explicitly, refs are scoped to the
webview that stamped them, and everything not opted in refuses typed.

## Scope

- **Opt-in config.** `AgentControlConfig` names semantic child labels
  (working shape: `with_semantic_child(label)`, repeatable). The set is
  fixed at mount — no runtime mutation. Empty set = today's behavior
  exactly.
- **Vocabulary.** Semantic/input requests (`snapshot`, `click`, `type`,
  `press`, `scroll`, `drag`, `wait_for`, `evaluate`) gain an optional
  `webview` field addressing a child by label; absent means the UI
  webview, so every existing wire shape is unchanged (additive per
  contract 012 — old clients keep working). Snapshot results name the
  webview they came from. `screenshot`, `command`, window ops:
  untouched.
- **Ref scoping.** A ref resolves only against the webview whose
  snapshot stamped it. Design the honest mechanism (per-webview ref
  prefixes in the shim, or resolution keyed by the request's `webview`
  target — decide in-card and record why); the observable rule is the
  contract's: cross-webview resolution fails as `UnresolvedRef`, never
  a wrong-element hit. Two clients interleaving on different webviews
  of one window stay interference-free.
- **Typed refusals.** Targeting a child that exists but is not opted in
  → `Unsupported` naming the opt-in absence; a label that matches no
  hosted webview → `UnknownWindow`-family typed error (pick the
  vocabulary's honest fit; extend the vocabulary only if nothing fits,
  and record it).
- **Marshalling.** The Card 231/233 evaluate bridge already takes a
  `Webview` handle; route per-target. The shim is already injected into
  every webview (Card 230's mount loop + initialization script) —
  verify that holds for children created after mount, and fix within
  scope if it does not.
- **Conformance fixtures** (mock runtime where executable, packaged
  deferrals to Card 240): opt-in respected; default closed byte-compat
  (existing fixtures untouched and green); typed refusals both kinds;
  wire-shape additivity (a request without `webview` deserializes to
  today's meaning).
- **Drift lock.** The skill tool table gains the `webview` column
  content in Card 240's truthfulness pass; if the drift check trips on
  argument text (it locks names), note it — names are unchanged here.

## Acceptance Criteria

- [x] empty opt-in set: all existing fixtures pass unmodified
- [x] opted-in child addressable on every semantic/input tool; absent
      `webview` still means the UI webview on the old wire shape
- [x] cross-webview ref resolution fails as `UnresolvedRef`, fixtured
- [x] both refusal cases typed and fixtured
- [x] shim reaches children attached after mount, fixtured or honestly
      recorded as a Card 240 packaged check
- [x] `effigy qa` passes; release-absence scan green both directions

## Validation

`effigy qa`; the plugin and core fixture suites; the release-absence
scan.

## Stop Conditions

- ref scoping cannot be made collision-safe without a server-side table
  that outlives the DOM — that contradicts contract 022's stateless
  posture; stop and report the design choices;
- the vocabulary needs a non-additive wire change — contract 012
  question, stop;
- anything wants runtime mutation of the opt-in set — that is a new
  authority surface, stop.

## Continuation

Card 240 proves it packaged, updates the teaching surfaces, and hands
Figmatic its opt-in instructions.

## Closeout

Status: done 2026-08-20, on `t3code/read-webview-targeting-handoff` in
worktree `/Users/tom/.t3/worktrees/longhorn/t3code-3feafabe` (launcher-
provided). Same PR as Card 240.

**Ref scoping.** Resolution is keyed by the request's `webview` target
(the handler evals in that webview). Collision safety is in the DOM, not
a server-side table: the shim stamps UI refs as `eN` (today's shape) and
child refs as `{encodeURIComponent(label)}:eN`. Same local seq in two
documents cannot match. Sending an island ref to the UI webview, or a UI
ref into the island, is `UnresolvedRef` — never a wrong-element hit.
Chose prefix-in-DOM over a wire-only wrap so the live attribute is the
scoped id and the core stays stateless. Prefix is injected per call
(`__longhornAgentRefPrefix`); late-attached children pick it up on the
first semantic call because every call includes the shim source.

**Vocabulary.** Additive `webview` on semantic/input requests; absent
means the UI webview. Snapshot results name a child and omit the field
for the UI webview. `UnknownWebview` is a new error: a label that
matches no hosted webview of the window. Exists-but-not-opted-in is
`Unsupported` naming the opt-in absence. `UnknownWindow` stays windows.
The opt-in set is `AgentControlConfig::with_semantic_child`, fixed at
mount.

**Fixtures.** Core: omitted `webview` serializes as today's JSON;
`webview` deserializes additively; `UnknownWebview` round-trips. Shim:
collision case, two documents, same seq, no cross-hit; byte-lock green.
Plugin: late-attach after mount, unknown label → `UnknownWebview`,
closed child → `Unsupported`, opted-in child is found (mock runtime
then fails `evaluationFailed`, not a targeting error). Empty opt-in is
the existing mount fixtures unmodified.

**Late-attach.** Plugin fixture attaches `preview` after
`mount_agent_control` and the opted-in snapshot is not a targeting
refusal. Card 240's packaged drive is the JS-executes proof.
