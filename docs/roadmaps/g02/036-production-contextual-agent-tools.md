# g02.036 Production Contextual Agent Tools

Status: L1 ready; D1 consumer successor blocked on accepted L1; production held
Owner: Longhorn maintainers; Desktop Coordinator owns delivery
Governing refs: contract 023, Swallowtail Contract063, Desktop contract031

## Promotion authority and dispatch

Operator-confirmed 2026-09-07; independently accepted Desktop PR144 merged
at `2a1bad3f`. Source: Desktop
`docs/handoffs/20260907-contract063-provider-free-consumer-composition.md`.
This promotes only the provider-free L1/D1 composition below. Production
adoption, provider execution, dependency changes and releases remain held.
Coordinator `914728cd` owns dispatch: L1 first, independent exact-head review
and merge, then D1 pinned to that accepted L1 artifact.

Mutable ownership: L1 owns a new opt-in Longhorn host-dispatch source/test
surface, public API baseline and focused evidence; D1 owns Desktop agent-host
fixture source and one evidence log. Shared contracts, roadmap indexes and
dispatch closeout remain Coordinator/Chatterbox reserved. No competing writer.
Worker capability: Rust typed lifecycle and integration testing. L1 and D1
are serial; disjoint Desktop UI and Swallowtail work may continue. Escalation:
Chatterbox for boundary changes; no second registry/listener or fake admission.
Completion requires the acceptance below, focused Effigy checks, API/diff
checks and independent exact-head callback-order review before merge.

## Exact source baseline

- Desktop ownership/admission: `30a338f2`; current consumer baseline
  `a7aae310133aa8972bca0eb7753242537a4adf16`.
- Longhorn ownership and proposed dispatch contract: `edc21078`.
- Swallowtail Batch A: `6c52b9f9`.
- Shared kernel: `c214ac5e`.
- Selected skill/projection: `70909172`.
- Qualified Codex binding: `d7e93e5552c5b272e55ddef8a531b5dd32e81bf0`.

No scope may replace these with an unreconciled branch head. A newer accepted
Swallowtail head may be used only after source-equivalence or focused review.
This revision is a source-equivalent rebase of the accepted composition plan
onto `a7aae310`; PR142's merged passage/context surface is preserved and remains
outside the fixture's dependency-acquisition mechanism.

## Isolated source-consumer fixture route

L1 and D1 compile and run through one provider-free fixture outside every
source repository. The fixture is disposable evidence infrastructure, not a
Desktop dependency-adoption mechanism.

1. Resolve source roots at runtime from the gitignored machine-local keys
   `AGENT_LONGHORN_REPO`, `AGENT_PROJECTS_ROOT` and the Desktop repository root. The
   runner may derive the Swallowtail checkout from `AGENT_PROJECTS_ROOT`, or use
   an explicitly documented symbolic key if one is added later. It must never
   write resolved absolute paths into a tracked file.
2. Create a temporary directory with `mktemp -d`. Write the fixture's Cargo
   manifest, lockfile and source only there. Path dependencies point at
   read-only source checkouts resolved in step 1; no `Cargo.toml`, lockfile,
   package override or global Cargo configuration in Desktop, Longhorn or
   Swallowtail changes.
3. Before resolving dependencies, require clean source trees and exact commits:
   Swallowtail `d7e93e5552c5b272e55ddef8a531b5dd32e81bf0`, or a separately reviewed
   source-compatible descendant; the independently accepted L1 commit; and the
   Desktop D1 fixture source commit. Stop on a dirty tree, SHA mismatch or
   unreviewed descendant.
4. Generate the same normalized manifest deterministically for a given source
   tuple, run Cargo with the generated lockfile locked, and record SHA-256
   hashes of the normalized manifest and lockfile. Sanitized evidence records
   symbolic source keys, exact source commits, hashes, command outcomes and
   safe protocol states only. It excludes resolved paths, arguments, schema
   bodies, credentials, endpoints and bearer material.
5. Compile and run L1 and D1 acceptance through that external fixture. Remove
   the temporary directory after the evidence hashes and results are recorded.
   The source repositories must remain clean and their dependency graphs
   byte-identical.

The accepted Longhorn L1 SHA is intentionally not guessed here. L1 promotion,
implementation and independent review produce it; D1 stays blocked until that
exact artifact exists. Real Desktop integration remains separately blocked on
an accepted dependency-adoption or released-artifact decision. Passing this
fixture does not make consumer production code ready.

## Ownership

| Owner | Fixture responsibility | Explicit exclusion |
| --- | --- | --- |
| Desktop | one read-only domain tool name, input/output schemas, effect/retry policy, bounded app context, durable task/session/attempt admission, packaging fixture and receipts | no registry, bridge listener/lease, provider wire or generic dispatch semantics |
| Longhorn | generic typed validation and callback dispatch, safe result/error mapping, recording dispatcher used by the fixture | no tool/schema ownership, admission issuer, registry, listener, lease, correlation kernel or daemon |
| Swallowtail | immutable registration snapshot/selection/preparation, host-service readiness, operation binding, result/cancellation lifecycle and Codex topology qualification | no Desktop business policy/schema and no Longhorn substitute registry |

## Exact composition surface

The promoted scopes consume these merged public seams rather than inventing a
parallel adapter:

1. Desktop builds `RegisteredToolDeclaration` values and passes them into
   `RegisteredToolSnapshot::new(RegisteredToolSnapshotInput { ... })`.
2. Desktop selects the exact native tool and qualified protocol through
   `RegisteredToolSelection::new(...)`.
3. Desktop binds its trusted durable identity through
   `ConsumerAdmissionBinding::new(...)`, including process incarnation,
   workspace/task generation, session/task/attempt and live revocation source.
4. Desktop constructs `RegisteredToolPreparation::new(...)`; preparation calls
   `RegisteredToolPreparation::prepare(&HostServices, ...)`.
5. The fixture installs the Swallowtail bridge with
   `HostServices::with_registered_tool_bridge(...)`. It does not start a
   listener, daemon or provider.
6. The mounted bridge receives a Longhorn implementation of
   `RegisteredToolDispatcher::dispatch(RegisteredToolCall,
   RegisteredToolDispatchContext)` and records the one admitted callback.
7. Codex qualification uses `CodexRegisteredToolBinding::qualify(...)` and the
   existing `CodexSessionProfileInput::with_registered_tools(...)` /
   `CodexAppServerDriver::with_registered_tools(...)` seam. Tests inspect the
   prepared session input and callback exchange only; they do not spawn Codex.

Provider-direct MCP stays withheld. Claude Card116 remains Unqualified on
ATTACH-01. Grok Card118 remains withheld. No parity claim follows from this
fixture.

## Scope L1 — Longhorn typed dispatcher fixture

Status after promotion: ready.

Scope: a new opt-in Longhorn host-dispatch crate/module, its generic fixture,
public API baseline and focused docs. Contract022 remains byte-for-byte
dev-only. No Desktop dependency, provider dependency, transport, server,
listener, daemon or release feature.

Acceptance:

- [ ] validate exact registration revision, schema digest/version, execution
      kind, trusted binding and positive bounds before callback invocation
- [ ] invoke one recording callback exactly once and return one bounded typed
      result
- [ ] refuse unknown schema/version, stale/foreign/revoked binding and
      duplicate or post-terminal result before callback execution
- [ ] cancellation reaches the in-flight recording callback once, produces one
      terminal cancellation outcome and cannot replay mutation
- [ ] diagnostics contain no arguments, schema bodies, credentials, endpoint,
      process path or bearer material
- [ ] default/release-absence proof contains no Contract022 evaluate, input,
      shell or arbitrary command surface

Validation must use Longhorn's focused Effigy selectors plus API/diff checks.
An independent exact-head review must inspect real callback call order, not
source-regex assertions alone.

The accepted L1 evidence must include a compile-and-run receipt from the
isolated source-consumer fixture route: readiness and pre-dispatch refusal,
one callback and one result exactly once, cancellation exactly once, and no
post-terminal or mutation replay. The receipt pins the accepted L1 SHA and the
Swallowtail source SHA plus deterministic sanitized manifest/lockfile hashes.

## Scope D1 — Desktop provider-free mounted composition

Status after promotion: blocked on accepted L1 artifact, then ready.

Scope: a fixture-only Desktop Rust integration under the existing agent host
test surface, generated bindings only if an existing public Desktop boundary
must change, and one named evidence log. No renderer UX, provider process,
credential/auth/config change, dependency pin, package/release mutation or
live content.

Acceptance:

- [ ] construct the Desktop-owned read-only tool declaration and exact schemas,
      then prove Swallowtail owns the immutable snapshot/selection
- [ ] issue a real Desktop `TaskContext` task/session/attempt binding and map it
      one-to-one into `ConsumerAdmissionBinding`; identity never comes from tool
      arguments or PID
- [ ] mount the accepted Longhorn recording dispatcher behind
      `HostServices::with_registered_tool_bridge(...)`
- [ ] prove absent bridge, wrong topology, stale generation and revocation fail
      before Longhorn dispatch
- [ ] qualify only Codex native callbacks, with MCP/Claude/Grok dispositions
      remaining explicitly withheld or Unqualified
- [ ] prove ready -> one call -> one result -> joined close, plus refusal and
      cancellation, each exactly once with no replay
- [ ] preserve Desktop queue/task state and record only sanitized identities and
      safe outcomes

The fixture may use Swallowtail's provider-free test host and scripted callback
surfaces. It must not launch a CLI, contact a provider, read credentials, alter
global configuration or claim packaged/live acceptance.

D1 acceptance uses the same isolated source-consumer fixture route and the
independently accepted L1 SHA. Its compile-and-run receipt must cover readiness,
pre-dispatch refusal, exactly one admitted callback and result, cancellation,
joined close and no replay. It must also prove the three source repositories
remain clean and their committed manifests and lockfiles are unchanged. This
receipt qualifies only the provider-free source composition; it is not a
Desktop dependency adoption, release proof or production integration.


## Stop conditions

No dependency pin, provider run, transport, daemon, release, credential/config
change or production adoption. Exact clean source tuples and reproducible
sanitized receipts are mandatory. Any scope change returns to Chatterbox.
