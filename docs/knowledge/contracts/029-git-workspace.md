# 029 Git Workspace

Status: active compiled boundary for parts 5a and 5b
Owner: Longhorn maintainers
Created: 2026-10-22
Depends on: contracts 001, 003, 012, 021, 024 and 025
Affects: `longhorn-git-workspace`; Acowtancy Desktop's Git workspace facade
until it migrates

## Problem

A desktop application that opens a Git working copy has to run Git without a
shell, read its status and branches, classify a remote refusal before it
writes anything, sync one branch safely, switch branches without losing work,
clone into a folder the operator named without ever replacing an existing one,
and decide whether a chosen folder is the workspace it expected. Acowtancy
Desktop built that mechanism in `apps/bovine-desktop/src-tauri/src/git.rs`
against its own contract 028, and it split the generic mechanism out of its
product policy on 2026-10-22 (acowtancy-v2 PR 984, merged at `4993376c`).
Every desktop app that opens a Git workspace needs one tested copy of the
mechanism, and none of them needs Acowtancy's remote, branch names or copy.

Source: unit 5 of the Praxis map
[`shared-tooling-extraction.md`](https://github.com/inflatable-cookie/praxis/blob/dbad7b093f0452800c4df3a96c0798ebf788b08f/docs/knowledge/architecture/shared-tooling-extraction.md)
at `dbad7b0`, read from `acowtancy/acowtancy-v2` at `1d2d13022`. Donor for this
part: `acowtancy/acowtancy-v2` at `4993376c0dd76ff15a92860307ab05d676087b04`,
`apps/bovine-desktop/src-tauri/src/git/mechanism/`. From this contract on, this
file owns the mechanism's behaviour; the map and the donor are evidence.

Tom approved unit 5's plan on 2026-10-22 and split it in two. Part 5a is
execution with the policy hook, status, remote-refusal preflight, Legacy Sync
and branch switch, clone with staging and an exclusive rename, and attach. Part
5b is GitHub sign-in in the same crate: the device flow, the stored grant and
its single-use refresh, sign-out, the commit identity, and the
invocation-scoped askpass credential bridge. The donor's sign-in half is
`git/mechanism/auth.rs` at `4993376c`, with the application's facade above it
in `git/auth.rs`.

## Generic versus product

Longhorn owns the mechanism: the policy hook, the subprocess executor, output
capture bounds and redaction, the failure classes and their wire codes,
worktree status and branch reading, remote refusal classification and
preflight, the Legacy Sync sequence, the explicit branch switch, clone staging
and the same-parent exclusive rename, attach reading, and remote identity
comparison and display.

The application owns every value and every sentence:

- process isolation: the environment and Git configuration a child Git process
  gets, including where its global config lives;
- the credential: how it is obtained, stored, refreshed and expired, and which
  environment values carry it. Part 5b keeps that split: the OAuth client id
  and any environment precedence, the credential service, namespace and slot,
  the endpoints a build uses (including any development override), the user
  agent and transport branding, the askpass token variable name, when a sign-in
  is started, the browser step, and every sentence remain the application's;
- the allowed remote, the operator branch naming scheme, and the integration
  base a new branch starts from;
- contract-readable root admission, and which root is admitted for a read;
- the outcome projection, progress, and every message a person reads;
- Tauri commands, event transport and TypeScript bindings.

Longhorn owns the rest of part 5b: the device and poll protocol and its bounds,
the stored grant format and its refresh skew, the serialized single-use
refresh, the token-exchange classification, the failure and progress
vocabulary, the credential-store readings, the commit-identity derivation, and
the askpass bridge with the helper program it writes and its secrecy rules.

No product constant, product module, Tauri type or product sentence enters this
crate. A product decision reaches it only as an argument, a `GitPolicy`
implementation, or a `GitCredential` value.

## Public API

Crate `longhorn-git-workspace`, at
[`crates/longhorn-git-workspace`](../../../crates/longhorn-git-workspace/README.md).
It depends on `longhorn-redact` and `longhorn-support-log` for output
treatment, `longhorn-core` for the credential-slot and credential-store
vocabulary, `secrecy` for every secret value, `serde` and `serde_json` for the
wire projections and the stored grant, a blocking `reqwest` with rustls for the
device flow, the token exchange and the profile read, `sha2` for the token
digest the identity cache keys on, `tempfile` for the bridge's private folder,
`uuid` for staging names, and `rustix` on Linux and macOS for the exclusive
rename. It has no Tauri, no shell, no `unsafe` code and no credential-store
implementation: the store is injected.

The handle:

- `Git::new(policy: &dyn GitPolicy, redactor: Redactor)`. The redactor is the
  caller's own, seeded with its secrets and rules.
- `Git::run(root, arguments, code, timeout)`: one argument vector under a hard
  wall clock, returning Git's trimmed standard output.

The policy hook:

- `GitPolicy::isolate(&self, &mut Command)` applies application isolation.
- `GitPolicy::credential(&self, remote) -> Result<Option<GitCredential>,
  GitFailure>` supplies the credential for a remote the mechanism identified as
  credential-scoped. `Ok(None)` means none, `Err` means the application could
  not decide and the operation stops.
- `GitPolicy::refresh_credential(&self, rejected: &GitCredential) ->
  Option<GitCredential>` supplies a replacement after a refusal.
- `GitCredential::new(env_vars, secret)` carries the child environment values
  and the value to keep out of every detail. `env_vars()` reads the pairs, and
  `secret()` names the same value back to the policy: part 5b's refresh takes
  the token the remote rejected.

The readings, all methods on `Git`:

- `status(&self, root) -> Result<GitStatus, GitFailure>`;
- `inspect_worktree(&self, root) -> Result<GitWorkspaceState, GitFailure>`;
- `origin_url(&self, root) -> Option<String>`, redacted with the caller's
  redactor;
- `push_capability(&self, root) -> Result<(), RemoteAccessFailure>`;
- `preflight_remote(&self, remote, credential: Option<&GitCredential>) ->
  Result<(), RemoteAccessFailure>`;
- `sync_workspace(&self, before, reload) -> Result<SyncReading, GitFailure>`;
- `switch_branch(&self, before, branch_id, reload) -> Result<SwitchReading,
  GitFailure>`;
- `read_attach(&self, root, expected_branch, expected_remote) -> AttachReading`;
- `clone_into(&self, remote, target, cancel, credential) -> Result<(),
  CloneFailure>`.

Free functions:

- staging: `target_already_present`, `staging_parent`, `create_owned_staging`,
  `rename_exclusive`, `remove_owned_staging`;
- classification and comparison: `is_https_github`, `classify_remote_failure`,
  `RemoteAccessFailure`, `credential_bridge_configured`,
  `is_remote_credential_operation`, `remotes_match`, `remote_identity`,
  `remote_for_display`, `has_switch_blocking_dirt`;
- output treatment: `redact_git_detail`, `FAILURE_DETAIL_BOUND`,
  `MAX_CAPTURED_OUTPUT_BYTES`, `MAX_CAPTURED_STDERR_BYTES`.

Wire projections: `GitWorkspaceState`, `GitStatus`, `GitBranch`,
`GitBranchKind` derive `serde::Serialize` with the donor's key shapes
(`camelCase` fields, `snake_case` branch kinds, `local_target` never
serialized). Readings that carry a product outcome (`SyncReading`,
`SwitchReading`, `AttachReading`) are not serializable: the application projects
its own result and copy from them.

Part 5b's surface:

- scope and vocabulary: `CredentialScope { store, slot }`,
  `GithubAuthFailure` (ten classes), `GithubAuthProgress` (two readings),
  `CredentialStoreReading` (`Missing`, `Present`, `Unavailable`);
- device flow: `DeviceFlowEndpoints::{github, new, loopback_for_tests}`,
  `run_device_flow(client_id, endpoints, cancel, progress)`;
- token exchange: the `TokenClient` trait, `AccessTokenResponse` fields,
  `GithubTokenClient::{new, loopback_for_tests}`, `classify_token_exchange`;
- the grant: `TokenGrant` with its four public fields, `read_grant`,
  `write_grant`, `read_credential_presence`, `sign_out`, `fresh_access_token`,
  `force_refresh_token`;
- the bridge: `AskpassConfig::{new, token_env}`,
  `GitCredentialBridge::{from_token, credential}`;
- the commit identity: `GitCommitIdentity`, `GithubCommitIdentityResolver::{new,
  loopback_for_tests, resolve_identity, clear_cache}`;
- constants: `GITHUB_DEVICE_CODE_URL`, `GITHUB_ACCESS_TOKEN_URL`,
  `GITHUB_DEVICE_VERIFICATION_URL`, `DEFAULT_POLL_INTERVAL`, `MAX_DEVICE_WAIT`,
  `ACCESS_REFRESH_SKEW`, `TOKEN_REQUEST_TIMEOUT`;
- test support behind the off-by-default `fixture-server` feature: the
  loopback constructors above and `FixtureServer`, `FixtureRoute`,
  `FixtureRequest`.

## Execution rules

- The program is the fixed name `git`, resolved on `PATH`, with an argument
  vector. No code path builds a shell string. A working folder is passed as
  `git -C <root>`.
- The child environment sets `GIT_TERMINAL_PROMPT=0` and `LC_ALL=C` and removes
  `GIT_AUTHOR_NAME`, `GIT_AUTHOR_EMAIL`, `GIT_COMMITTER_NAME` and
  `GIT_COMMITTER_EMAIL`.
- `GitPolicy::isolate` runs before the invocation credential is applied, so a
  credential environment value wins.
- A credential is applied only to `fetch`, `push` or `ls-remote` naming
  `origin` on a GitHub remote over HTTP or HTTPS
  (`is_remote_credential_operation`). Every other invocation, including a
  status or a branch read on such a worktree, never asks the policy for one.
- When a credential-scoped call fails and `classify_remote_failure` reads the
  failure as `AuthenticationRequired`, the policy is asked once for a refreshed
  credential and the call is retried once. A refusal to refresh keeps the
  original failure. The retry runs under the same deadline as the first
  attempt: a bounded call fixes its deadline when it starts, and that deadline
  covers the origin lookup, both attempts and the policy's refresh. A refresh
  that answers after the deadline, with a replacement or without one, reports
  `GitFailure::TimedOut` without a retry.
- The wall clock bounds Git processes and their output, not caller code.
  `GitPolicy::credential` and `GitPolicy::refresh_credential` run
  synchronously on the calling thread and the mechanism cannot preempt them: a
  hook that blocks past the bound delays the return until it answers, and the
  call then reports `TimedOut` without starting Git again. Keeping the hooks
  prompt, or giving a slow lookup its own limit, is the policy's obligation.
- Standard output is retained from its head within
  `MAX_CAPTURED_OUTPUT_BYTES` (64 KiB). A successful call whose standard output
  exceeds that bound fails with `git.output_too_large` instead of returning a
  partial answer.
- Standard error is retained from its head within
  `MAX_CAPTURED_STDERR_BYTES` (1 MiB). A tail cut can remove the opening line
  of a private-key block, and redaction then no longer recognises the body that
  follows, so the capture keeps the head: a cut at the end leaves an
  unterminated block, which the rules redact through the end of the text. Past
  the bound, later output is not retained and the returned detail says so.
- Both streams are drained, so a chatty child cannot fill a pipe and stall the
  wait.
- Successful standard output is redacted with the caller's redactor and the
  invocation secret before `Git::run` returns it. Parsing reads (`status`,
  branch discovery, the attach comparison, the credential decision) take the
  exact bytes, because redaction would corrupt an identifier; the values they
  publish (`origin_url`) are redacted at the public boundary.
- `Git::run` is the only bounded call. The child runs in its own process group
  on Unix, and its `timeout` covers both the wait and the drain. On reaching
  the bound the group is killed, the child is reaped and both readers are
  joined, so the returned `GitFailure::TimedOut` comes at the bound even when
  Git has already exited and a shell alias, a remote helper or another
  descendant still holds its output pipes: the call has not finished until that
  output does. The readings in this part impose no wall clock; a caller that
  needs one uses `run`, and a clone is bounded by its cancellation flag, which
  kills the same process group.
- Capture readers are joined on every exit, including a kill, so a call leaves
  no reader or child behind.

## Failure classes and wire codes

`GitFailure` carries machine facts only. The application maps a variant and a
code to its own copy; the mechanism authors no operator sentence.

| Class | Variant | Codes |
| --- | --- | --- |
| Git refused | `Refused { code, detail }` | `git.status_failed`, `git.fetch_failed`, `git.push_failed`, `git.fast_forward_failed`, `git.switch_failed`, `git.push_capability`, `git.remote_preflight_failed` |
| Wall clock | `TimedOut { code, after }` | the caller's code |
| Could not run or trust | `Unavailable { code, detail }` | `git.command_unavailable`, `git.output_too_large` |
| Not a worktree | `NotWorktree { code }` | `git.repository_unavailable` |
| Folder unavailable | `FolderUnavailable { code, detail }` | `git.repository_unavailable` |
| Branch not observed | `BranchMissing { code }` | `git.branch_not_found` |

Branch and status reads also use `git.branch_unavailable`,
`git.origin_unavailable` and `git.branch_discovery_failed`. `GitFailureCode`
exposes `as_str()` with the donor's wire strings, so an application that
already classifies those strings keeps its classification. A refusal's
`detail()` is Git's own redacted words; `NotWorktree`, `BranchMissing` and
`TimedOut` carry no Git text.

## Worktree reading

`inspect_worktree` canonicalises the folder, requires
`git rev-parse --is-inside-work-tree` to answer `true`, reads the current
branch through `git symbolic-ref --quiet --short HEAD` (falling back to
`git rev-parse --short HEAD` with `detached = true`), reads `@{upstream}` when
not detached, discovers local and remote-tracking branches with two
`for-each-ref` reads, and reads status.

`status` parses `git status --porcelain=v1 --branch`. A `??` line counts as
untracked; `DD`, `AU`, `UD`, `UA`, `DU`, `AA` and `UU` count as conflicted;
every other line counts its index and worktree columns as staged and modified.
The `## ` header's `ahead` and `behind` distances are read from the bracket
list. `clean` is computed once, as the sum of the four dirt counters being
zero; no caller reconstructs cleanliness.

Branch ids are `local:<name>` and `remote:<name>`. A remote reference records
its tracking local branch, if one exists, as `local_target`; that value is
never serialized. A remote reference whose `symref` is set, or whose short name
ends in `/HEAD`, is not a branch and is skipped.

## Remote refusal preflight

`classify_remote_failure` separates five readings from Git's own stderr:
`CredentialHelperMissing`, `AuthenticationRequired`, `NetworkUnavailable`,
`RepositoryUnavailable` and `Unavailable`. It matches the donor's phrases and
its precedence: network and DNS first, then missing prompts and helpers, then
refused credentials and permission, then repository availability.

`preflight_remote` is read-only: it runs
`git ls-remote --heads -- <remote>` with no repository and no mutation, and
returns only the classification. An unreachable or absent remote reads as
`RepositoryUnavailable`.

A GitHub remote over the HTTP or HTTPS client route — the donor treats both
spellings alike — requires a supplied credential and is refused as
`CredentialHelperMissing` before Git runs when none is supplied. Ambient
credential helpers are never consulted: an isolation configuration carrying a
`gh`-style URL-scoped helper changes nothing. What the machine has configured
cannot make an uncredentialled remote readable, and cannot be used as a
fallback.

`push_capability` runs `git push --dry-run origin HEAD` in the worktree and
classifies a refusal with the same vocabulary. `--dry-run` contacts the remote
and resolves the ref without creating, updating or deleting anything on either
side.

## Legacy Sync

`sync_workspace` takes one prior reading and a `reload` closure that re-reads
the same workspace through the caller's own admission path. The sequence is
fixed:

1. stop with `Detached` on a detached checkout;
2. stop with `Dirty` on any staged, modified or conflicted path;
3. stop with `NoUpstream` when the branch tracks nothing;
4. `fetch`; re-read;
5. stop with `DirtyAfterFetch` if the worktree is now dirty, or
   `DivergedAfterFetch` if it is both ahead and behind;
6. when behind, `merge --ff-only @{upstream}`; re-read;
7. stop with `DirtyBeforePush` or `DivergedBeforePush` on the same readings;
8. when ahead, `push`; re-read.

There is no rebase, force, stash, reset, clean or branch picker. A failure at
a step returns the step's code. `SyncReading::Completed` reports whether the
fetch ran, whether the workspace fast-forwarded and whether the branch was
pushed; `SyncReading::Stopped` reports the stop, the last reading and how far
the sequence got.

## Branch switch

`switch_branch` accepts only a branch id already present in the prior reading;
an unknown id is `git.branch_not_found`. A branch that is already current is
`AlreadyCurrent` and runs nothing. Tracked, staged or conflicted work blocks it
with `Blocked`. Untracked content does not: it is in-progress work, not dirt.

A local branch switches by name. A remote branch with a tracking local branch
switches to that local branch; a remote branch without one switches with
`--track`. After a switch the caller's `reload` re-reads the workspace, so an
application that admits its roots keeps owning that admission.

## Clone staging and the exclusive rename

An invocation owns one unique staging folder under the destination's parent.
`create_owned_staging` names it `<target>.creating-<uuid-v7>` and retries a
bounded sixteen times on a collision. `clone_into` requires that empty folder,
spawns `git clone --progress -- <remote> <staging>`, applies isolation before
the caller's credential environment, and bounds the clone by the caller's
cancellation flag: a cancelled clone has its process group killed, is reaped
and is reported as `Cancelled`. The flag also bounds the drain after Git
exits: a remote helper or another descendant that still holds standard error
cannot delay a cancel, which kills the group by the id taken at spawn. Standard
error keeps its head within the capture bound; its refusal detail is redacted with the caller's redactor and
the credential secret, then cut to `FAILURE_DETAIL_BOUND`.

`rename_exclusive` moves the staging folder into place with a same-parent
exclusive rename: `RENAME_NOREPLACE` on Linux and `RENAME_EXCL` on macOS,
through `rustix`, and `Unsupported` elsewhere. It never replaces an existing
destination.

The composition rule is the application's, and this contract pins it: check
`target_already_present` first, preflight the remote before any filesystem
write, create the staging folder next to the target, clone into it, run the
application's branch and validation steps inside it, remove the staging folder
on every failure with `remove_owned_staging`, and move it into place only with
`rename_exclusive`. A target that appears mid-run survives unchanged and the
staging folder is removed; a failed clone leaves no partial target.

## Attach

`read_attach` compares one folder against an explicit expected branch and
expected remote. It never mutates the checkout. The readings are:

- `Ready` when the expected branch is current;
- `SwitchOffered` when another branch is current, or the checkout is detached,
  and only untracked content is present. `create_from_integration_base` is true
  when neither the expected branch nor `origin/<expected>` was observed, so the
  application knows the branch must be created from its own integration base;
- `WrongBranchDirty` when a tracked, staged or conflicted path exists on
  another branch;
- `WrongRemote` when `origin` does not match, carrying the observed remote in
  `remote_for_display` form with any user-info credential removed, redacted
  with the caller's redactor like every other Git text the crate returns;
- `NotRepository` when the folder cannot be read as a worktree, carrying the
  classified failure.

`remotes_match` compares `remote_identity` values: the trailing slash and
`.git` suffix are removed, a scheme is removed, the user is removed, `:` becomes
`/`, and case is folded. That makes `git@github.com:owner/repo.git`,
`ssh://git@github.com/owner/repo.git` and
`https://github.com/owner/repo` the same repository.

## Redaction and bounds

The invocation secret and the caller's `longhorn-redact` `Redactor` run over
every Git text the crate returns. `redact_git_detail(redactor, detail, secret)`
is the failure form: the secret pass, then the caller's redactor, then
`longhorn-support-log::bounded_tail` with `FAILURE_DETAIL_BOUND` — at most 40
lines and 4096 bytes, the newest bytes kept, moved to a character boundary
with `OMITTED_MARKER` in front. Every failure detail the crate returns goes
through it; a clone's refused stderr too.

Standard error is redacted from its captured head, before any cut. A stream
past `MAX_CAPTURED_STDERR_BYTES` carries a note that later output was not
retained, so a cut never returns a private-key body whose opening line the cut
removed.

Successful standard output takes the same two redaction passes before
`Git::run` returns it, then `longhorn-support-log::bound_field` keeps it within
`MAX_CAPTURED_OUTPUT_BYTES`; a stream beyond that bound is refused, not
truncated. `origin_url` is redacted the same way, so a credential in the
remote's user-info component never leaves the crate; `remotes_match` and
`remote_identity` remain usable on the redacted value because they remove that
component before comparing, and `remote_for_display` stays available for raw
values a caller obtained itself. An attach reading's `WrongRemote.observed`
takes the same redaction after its display form.

Structured identifiers Git reports stay exact: branch names and ids, upstream
references, the current branch or detached revision shorthand, and the
canonical root. They are addresses, not prose — a branch id is handed back to
`switch_branch`, so a redacted identifier would stop addressing anything. A
caller that displays one applies its own redaction policy, as it does for its
own paths.

## Required evidence

- `effigy test:git-workspace` runs the crate's integration and doc tests under
  `crates/longhorn-git-workspace/nextest.toml` (30-second slow-test period,
  two-period termination, one-second forced-kill grace, immediate stop after
  the first failure). The tests run real `git` on `PATH` against temporary
  local repositories and bare remotes, with no network and no OS credential
  backend. They cover status, conflicts, ahead and behind, detached
  checkouts, a preflight refusal per class, the ambient-helper rule, sync in
  every stop state, branch switching, clone staging and the exclusive rename
  including an existing target and a failed clone, attach in every reading, the
  credential hook and its delivered environment, redaction of successful
  standard output, of an attach reading's observed remote and of failure
  detail, a private-key block longer than any tail window, the capture bound
  and its withheld-overflow note, and the hard wall clock both against a
  sleeping descendant that holds the output pipes and against a Git process
  that exits before its backgrounded descendant does. A clone against a remote
  that hangs proves cancellation returns at its flag, and a clone whose Git
  exits while a descendant holds standard error proves a cancel during the
  drain does too. A scripted `ext::` remote refuses for authentication: one
  test proves a refreshed credential is retried once and succeeds, and one
  proves the retry stays inside the first attempt's deadline, and one proves a
  declined refresh that answers late is a timeout. A credential lookup that
  fails after the bound is a timeout too.
- Part 5b's tests need the `fixture-server` feature, so `test:git-workspace`,
  `lint:git-workspace` and `docs:git-workspace` compile with
  `--features longhorn-git-workspace/fixture-server`. They drive the device
  flow, the token exchange and the profile read against a loopback server: a
  flow that succeeds, one the operator denies, one that reaches its wait, one
  that is told to slow down (which proves the lengthened interval by polling
  once inside it and cancelling), one cancelled before any request and one
  cancelled during a wait, an outage and an unusable answer. They cover the
  client id and device code posted (never a token), the stored grant's document
  and both expiries, the refresh inside the skew, an access token outside it,
  an expired and a refused refresh token (grant kept), a token-endpoint outage
  (grant kept), a registration error, two concurrent callers causing one
  exchange, two forced refreshes reusing a rotated grant, a refresh whose
  exchange signs out (the slot stays empty) and one whose exchange completes a
  new sign-in (that grant is adopted and kept), a bare stored value, the store
  readings, sign-out, and the exchange classification. Values an endpoint or a
  store supplies that the clock cannot represent are proved not to panic: a
  stored grant with `u64::MAX` expiries reads as no declared expiry and still
  refreshes on a forced call, a token response with `u64::MAX` expiries stores
  no declared expiry, a device `interval` of `u64::MAX` waits only for the
  declared second of wait, and an `expires_in` of `u64::MAX` is capped at
  `MAX_DEVICE_WAIT` and stays cancellable. A response whose `error` echoes the
  access token is proved absent from `Debug`, and a successful answer is proved
  to carry no error text. The production token client is proved against the
  loopback server for a non-JSON 429, 503 and 200 answer (the first two
  `Offline`, the third `Failed`) and for a body that stalls past the request
  clock (`TimedOut`). The commit
  identity suite covers the noreply address, the cache across a rotated token
  and a cleared cache, the display-name fallback, an unauthorized, rate-limited
  and unusable profile, an empty slot, and an identity read that refreshes.
  The askpass suite runs real `git`: the helper's answers through `git
  credential fill` for both prompts, owner-only permissions, the folder removed
  with its bridge, a real fetch from a local bare remote through a bridge
  credential, a recorded child environment that carries the askpass values with
  no token in any argument, and a refresh-and-retry where the first fetch is
  refused by a scripted `ext::` program and the retried fetch brings the
  remote's commit. The loopback endpoints refuse a non-loopback host, and the
  production constructors refuse plain `http`.
- `effigy lint:git-workspace` compiles every target with clippy at `-D
  warnings`; `effigy docs:git-workspace` builds rustdoc with warnings denied.
- `effigy check:git-workspace` aggregates those three and
  `check:api-reference`; the `git-workspace` QA group selects it.

Limits. No test reaches a real host, so the network and authentication classes
are proved by their classification and by local refusals, not by a live
refusal. The authentication refusal that drives refresh-and-retry is a scripted
local `ext::` program, not a real host's; the scripted policy proves the
credential hook, its delivered environment, its failure mode and its refresh. The
wall clock cannot preempt a synchronous policy hook: a slow `credential` or
`refresh_credential` delays `Git::run` past its bound, and the call reports
`TimedOut` once the hook answers. The exclusive rename is proved on Linux and macOS only; elsewhere the
function reports `Unsupported`. Process-group termination is Unix only; on a
host without it the direct child is killed, so a descendant that keeps an
output pipe open can delay cleanup there. One invocation retains at most 64 KiB
of standard output and 1 MiB of standard error, plus the redaction copies of
those bytes, for the length of the call.

Part 5b's own limits. The device flow, the token exchange and the profile read
are proved on a loopback server, so the `https` requirement of the production
constructors is proved by refusal, not by a live TLS request. The flow's wait
bound is proved at one second of declared wait, not at the 900-second ceiling.
The body timeout is proved against a loopback server that sends headers and
withholds the body, at the real ten-second clock. The credential store has no
conditional write (contract 021), so a refresh closes the sign-out and new
sign-in windows by re-reading the slot immediately before it writes; a
sign-out that lands between that read and the write is the one window the crate
cannot close.
The askpass bridge and the commit identity are proved on the current host only,
and the permission rules are checked on Unix. The token exists in `secrecy`
memory in three places at once during an invocation — the refreshed grant, the
bridge and the invocation credential — so a process dump taken while Git runs
can still see it; the crate keeps it out of arguments, files, logs, errors and
`Debug` output, not out of process memory. The `fixture-server` feature is off
by default but available in any build that enables it, so a shipped build that
enables it could reach a loopback endpoint; confining the feature to test
builds is the consumer's obligation.

## Part 5b: GitHub sign-in

Part 5b signs a desktop application in to GitHub, keeps the grant in contract
021's credential slots, and hands Git one invocation-scoped credential.
`GitCredential` stays the boundary type part 5a's policy returns; 5b is how the
application builds one.

### Transport

Every production endpoint is `https`. Each request follows no redirect, has a
`TOKEN_REQUEST_TIMEOUT` (10 s) wall clock, and reads at most 64 KiB of the
answer. A request that times out is `TimedOut`; any other transport error is
`Offline`. A body that outlives the same clock — headers received, body still
arriving — is `TimedOut` too: `reqwest` reports it as an opaque I/O error that
wraps its own timeout, and the mechanism reads the wrapped value before it
trusts the kind. A rate limit or a server fault is `Offline` whatever its body
holds: 429 and 5xx are classified before the body is parsed, because GitHub's
edge answers a rate limit with a page, not a token document. A response body is
parsed once and never echoed into a failure, a log line or a `Debug` output:
`AccessTokenResponse`'s `Debug` redacts the `error` field, because that field is
body text rather than a fixed code and can echo a token. A successful answer is
returned with that field cleared, for the same reason.

Plain `http` exists only through the `fixture-server` feature's loopback
constructors (`DeviceFlowEndpoints::loopback_for_tests`,
`GithubTokenClient::loopback_for_tests`,
`GithubCommitIdentityResolver::loopback_for_tests`), which accept scheme `http`
or `https` on `127.0.0.1` with an explicit port and nothing else. The
production constructors refuse anything but `https` with `NotConfigured`.

### Device flow

`run_device_flow` posts `client_id` to the device-code endpoint. An empty
client id is `NotConfigured`; 429 and 5xx are `Offline`; any other non-success
status is `Failed`. The answer must name a device code and a user code.

The verification address reported to the application is always
`GITHUB_DEVICE_VERIFICATION_URL`. A device response naming another address is
reported as that pinned page, so the application only ever shows and opens
GitHub's own page. The one-time user code and that page are the only values
progress carries: no device code, no token, no account value.

Progress is `AwaitingVerification` once with the code, then `Polling` after each
poll that has not answered yet.

Each poll posts `client_id`, `device_code` and the device-code grant type.
The interval is the device response's `interval` or `DEFAULT_POLL_INTERVAL`
(5 s), never below 5 s. An `authorization_pending` answer keeps polling; a
`slow_down` answer lengthens the interval to the server's suggestion or by 5 s,
never below 5 s; `access_denied` is `Denied`; `expired_token` and
`token_expired` are `TimedOut`; anything else is `Failed`; 429 and 5xx on a poll
are `Offline`.

A poll interval too large to represent waits only until the flow's deadline, so
an endpoint's `interval` suggestion cannot panic the wait.

The flow waits for the shorter of the device response's `expires_in` and
`MAX_DEVICE_WAIT` (900 s). A wait that ends at that bound reports `TimedOut`
instead of polling again, so a poll is never sent after the operator's window
closed; an in-flight request keeps only its own `TOKEN_REQUEST_TIMEOUT`. It
explores the caller's cancellation flag before the first request, before every
poll, and every 100 ms of a poll wait, and reports `Cancelled` when it is set.
The mechanism never opens a browser: the application shows the code and opens
the pinned page from its own action.

### The stored grant

`TokenGrant` holds the access token and the refresh token as
`secrecy::SecretString`, each with an optional expiry. Expiries are absent only
when the token endpoint declared none. `TokenGrant`'s `Debug` names no token.

The stored document is JSON:

```
{"v":1,"access_token":"…","access_expires_at":1729600000,
 "refresh_token":"…","refresh_expires_at":1730200000}
```

Absent expiries and absent refresh tokens are omitted; times are whole seconds
since the Unix epoch.

An expiry the clock cannot represent — a stored document or a token response
declaring an absurd number of seconds — is read as no declared expiry, which
the grant already covers as "does not expire on its own". No duration that an
endpoint or a store supplies panics the mechanism.

A stored value that is not that document — not JSON, another version, or a
blank access token — is read as a bare access token with no expiry and no
refresh token. That is the shape an earlier build of the application wrote, and
refusing it would strand a signed-in operator; a forced refresh on such a value
is `SignInExpired`.

Every read, write and removal goes through the caller's `CredentialStore` and
`CredentialSlot`. A store failure is `CredentialStoreUnavailable`, never "not
signed in". `read_credential_presence` reads presence and never the value, and
carries the backend's own words in `Unavailable(detail)`.

### Refresh

The five-minute `ACCESS_REFRESH_SKEW` is kept: an access token that expires
within the skew, or has already expired, is refreshed before it is used. A
grant with no declared expiry is used as it is unless the caller forces a
refresh.

One process-wide refresh lock covers the read, the exchange and the write, so
two callers that need a refresh at once cause one exchange and the second
adopts the rotated grant. `fresh_access_token(scope, client_id, client, now,
force, rejected_access)` is the one entry point; `force_refresh_token` is its
after-a-rejection form with the current time and `force` set.

A rejection names the access token the remote refused. When the stored grant
already holds a different token, that rotated grant is used and no exchange
happens, so concurrent Git and identity callers share one refresh.

The exchange posts `client_id`, `grant_type=refresh_token` and the refresh
token in a form body, never in a URL. A successful answer replaces both tokens
in the store; an answer without a refresh token is `Failed` and the stored
grant is left in place.

A refresh writes only over the grant it read. The exchange takes time and the
store has no conditional write, so the slot can be emptied by `sign_out` or
replaced by a new sign-in while it runs. The slot is read again after the
exchange and the write is abandoned when it no longer holds this grant: a
signed-out slot stays empty and the call reports no token, and a newer sign-in
is adopted as it stands rather than overwritten by a stale refresh.

A missing or expired refresh token is `SignInExpired` when the caller forced a
refresh or the grant declares an access expiry; otherwise the stored token is
returned. A refused refresh token (`bad_refresh_token`, `invalid_grant`) is
`SignInExpired` and the stored grant stays, so the application can offer a new
sign-in without losing the account's last value. `incorrect_client_credentials`
and `unauthorized_client` are `NotConfigured`; 429 and 5xx are `Offline`; a
success is a non-blank access token whatever the status; anything else is
`Failed`.

The refresh sits inside part 5a's deadline rules, unchanged: the application
performs it inside `GitPolicy::refresh_credential`, which part 5a runs
synchronously and then checks its own clock against. The mechanism adds one
bounded HTTP request and no other wait, so a hook that returns promptly keeps
`Git::run`'s bound, and a hook that answers late makes the call report
`TimedOut` exactly as part 5a specifies. The composition is:

- `credential` reads `fresh_access_token`, builds a `GitCredentialBridge` from
  the token, and returns `bridge.credential()`;
- `refresh_credential` passes the rejected credential's `secret()` to
  `force_refresh_token`, builds a new bridge from the result, and returns its
  credential;
- a failure returns `Err`/`None` instead of a credential-less answer, so part
  5a stops rather than running Git with whatever the machine has configured.

### The askpass bridge

`AskpassConfig::new` names the environment variable the bridge carries the
token in. The name must be a shell-safe identifier (`[A-Za-z_][A-Za-z0-9_]*`);
anything else is `NotConfigured`. The application supplies no program.

`GitCredentialBridge::from_token` writes the crate's own helper program into a
new temporary folder with owner-only permissions — folder and program both
`0700` on Unix — keeps the token in `secrecy` memory, and refuses a blank token
with `CredentialStoreUnavailable`. The folder exists for exactly that bridge's
lifetime and is removed when it is dropped.

The helper prints the token for a password prompt and `x-access-token` for a
username prompt, which is GitHub's token-authentication convention. It reads
the named variable and holds no token itself.

`bridge.credential()` gives the invocation `GitCredential`: `GIT_ASKPASS`
pointing at the helper, the named token variable, `GIT_TERMINAL_PROMPT=0`, and
one environment-config entry that clears `credential.helper` for this child
only, so a stale machine-wide helper never runs first. The credential's secret
is the token, so every Git text the crate returns is redacted of it.

The token never appears in a Git argument, a file, a log line, an error or a
`Debug` output. It reaches Git only as the child environment value the helper
prints, and the bridge must stay alive until the invocation finishes.

### Commit identity

`GithubCommitIdentityResolver::new` takes the `https` profile endpoint and a
non-empty user agent; anything else is `NotConfigured`. `resolve_identity` reads
a fresh token (`NotSignedIn` when the slot is empty), then the profile with the
token as a bearer credential and the caller's user agent.

The account id must be non-zero and the login must be ASCII alphanumeric or `-`.
The email is `<id>+<login>@users.noreply.github.com`. The name is the trimmed
display name when it is non-empty and holds no control character, and the login
otherwise. A 401 is `SignInExpired`, 429 and 5xx are `Offline`, any other
failure status or unparsable body is `Failed`.

The resolver caches the account per token digest (SHA-256, never the token
itself) and the identity per account id, so a repeated read costs no request
and a rotated token is looked up afresh. `clear_cache` forgets both, and the
application calls it on sign-out.

### Failure classes

| Class | Variant | Reported by |
| --- | --- | --- |
| Bad configuration | `NotConfigured` | an empty client id, a non-`https` endpoint, a malformed askpass name, a refused registration |
| No stored grant | `NotSignedIn` | an identity read on an empty slot |
| Store unavailable | `CredentialStoreUnavailable` | a store read, write or removal failure; a blank token to the bridge |
| Unreachable or busy | `Offline` | a transport error, 429, 5xx |
| Declined | `Denied` | `access_denied` |
| Out of time | `TimedOut` | the flow's wait, an `expired_token` answer, a request past its timeout |
| Cancelled | `Cancelled` | the caller's flag |
| Browser | `BrowserUnavailable` | never by this crate: the application's own browser step reports it into this vocabulary |
| Unusable answer | `Failed` | a bad exchange, an unusable profile, a refresh without a refresh token |
| Sign in again | `SignInExpired` | a missing, expired or refused refresh token; a 401 profile read |

No variant carries a token, a response body or an account detail. The
application maps a variant to its own copy.

## Compatibility for consumers

Deltas from Acowtancy Desktop's mechanism at `4993376c`:

- The mechanism's modules become one crate. `git.rs`, `git/auth.rs`,
  `workspace_attach.rs`, `workspace_create.rs` and `durability.rs` stay in the
  application: root admission, the allowed remote, branch naming, copy,
  developer-tool checks, durability and publishing observation.
- `DesktopGit` becomes the application's `GitPolicy` implementation. Its
  `isolate` moves unchanged. `credential` returns the caller's `GitCredential`,
  built by the application rather than by the mechanism. `refresh_credential`
  returns `Option<GitCredential>` and receives the rejected credential; the
  mechanism no longer knows a `GithubAuthFailure`.
- `GitCredentialBridge` is this crate's type, but the temporary directory and
  the askpass program are the crate's too. `AskpassConfig` now carries only the
  token variable name: the application supplies no program bytes. Part 5a's
  `GitCredential` is still the environment pair list plus the secret to redact.
- `preflight_remote_with_env(remote, env, token)` becomes
  `preflight_remote(remote, Option<&GitCredential>)`. The `GIT_ASKPASS` presence
  reading becomes "a credential was supplied", so a caller that supplies a
  credential without a bridge is no longer silently treated as uncredentialled.
- `run_git_unbound_with_env` becomes a private unbound probe; callers use
  `preflight_remote`.
- Failure codes are the same wire strings, carried by `GitFailureCode`.
  `GitFailure::Worded(GitCommandError)` is removed: the application owns its
  error type and maps the variant and code itself. `TimedOut` reports
  `after: Duration` instead of whole `seconds`; a sub-second bound no longer
  reads as zero.
- `GitFailure::UnusableMergeTree`, `committed_blob`, `merge_tree_write_tree`,
  `merge_tree_write_tree_supported`, `run_git_as` and `run_git_exact` are not
  in this crate. The merge-tree fallback and blob reads belong to the
  application's readiness and issuance flows; the commit identity is part 5b's
  `GithubCommitIdentityResolver`, not a Git call.
- Readings replace pre-wired results: `sync_workspace` returns `SyncReading`
  and `switch_branch` returns `SwitchReading`. `GitSyncResult`,
  `BranchSwitchResult`, `GitCommandError`, `SyncStop`'s messages and every
  outcome label stay in the application.
- `GitWorktreeFacts` is removed. `GitWorkspaceState`'s fields are public and
  `GitStatus::clean` is computed once, so a readiness check reads the same
  facts the mechanism computed.
- Output is bounded and redacted where the donor was not: 64 KiB of standard
  output and 1 MiB of standard error per invocation, a failure detail cut to
  4096 bytes / 40 lines, successful standard output redacted with the caller's
  redactor and the invocation secret, and `origin_url` and an attach reading's
  observed remote redacted at the public boundary. A donor caller that expected
  full stderr in a diagnostic now receives the redacted head or a withheld
  note, and one that expected raw `git log`-style output from `run_git` reads
  the parsed readings instead.
- A timeout or a clone cancellation kills the child's process group on Unix
  instead of only the direct child, so the bound holds when Git's own children
  (a shell alias, a remote helper) are still writing.
- The crate took `longhorn-core` and `sha2` with part 5b: the credential
  slots come from contract 021, and the identity cache keys on a token digest.
  Exact-path commit and review fingerprints still stay with the application.
- `longhorn-redact` and `longhorn-support-log` replace the donor's local
  redaction call, so the application's own secrets participate through the
  `Redactor` it hands the handle.

Part 5b's deltas from `git/mechanism/auth.rs` at `4993376c`:

- `AskpassConfig` loses its `script` field: it carries the token variable name,
  validated as a shell-safe identifier, and the crate writes the helper.
- `GithubAuthProgress::Starting { detail }` is removed: it carried product
  copy. The mechanism reports `AwaitingVerification` once and `Polling` after
  each unanswered poll, and no longer repeats `Polling` immediately after
  `AwaitingVerification` as the donor did.
- `CredentialScope`, `CredentialStoreReading`, `GithubAuthFailure`,
  `GithubAuthProgress`, `TokenGrant`, `DeviceFlowEndpoints`, `run_device_flow`,
  `TokenClient`, `AccessTokenResponse`, `GithubTokenClient`,
  `classify_token_exchange`, `read_grant`, `write_grant`,
  `read_credential_presence`, `sign_out`, `fresh_access_token`,
  `force_refresh_token`, `AskpassConfig`, `GitCredentialBridge`,
  `GitCommitIdentity` and `GithubCommitIdentityResolver` are public here; in
  the donor they were `pub(crate)` behind the application's facade.
- `ensure_fresh` is folded into `fresh_access_token`; `serialize_grant`,
  `parse_grant`, `form_body`, `form_encode`, `classify_device_error`,
  `sleep_with_cancel` and the response types' fields that the exchange does not
  need stay private.
- Secrets use `secrecy::SecretString`, not `zeroize::Zeroizing<String>`:
  `TokenGrant`'s token fields are public and typed `SecretString`,
  `fresh_access_token` returns `Option<SecretString>`, `force_refresh_token`
  takes the rejected token as `&SecretString`, and the bridge takes its token as
  `SecretString`. `GitCredential::secret()` became public so a policy can name
  the rejected token back to `force_refresh_token`.
- `read_grant`, `write_grant` and `sign_out` report a `GithubAuthFailure`, not a
  string or a store error; the store error is `CredentialStoreUnavailable`.
- `open_browser` is not in the crate: opening the pinned page is the
  application's host step, and `BrowserUnavailable` stays in the vocabulary for
  that step to report. `admit_loopback_user_url` is replaced by the
  `fixture-server` loopback constructors, which every client shares.
- The device-code, access-token and user-profile addresses are explicit values.
  GitHub's public pair stays a default (`DeviceFlowEndpoints::github`), and
  plain `http` is refused outside the loopback constructors; the donor's URL
  constants and any environment override stay with the application.
- `GithubTokenClient` is built from its endpoint and keeps the donor's form post
  with `Accept: application/json`; it no longer reads an override.
- The stored grant document is unchanged — version 1 JSON with whole-second
  expiries, absent optionals omitted, and a bare value read as a legacy access
  token — so a slot written by the donor's build is read without a rewrite.

## Extraction state

Contract 003 steps 1 to 4 are done for unit 5. The donor is admitted at
`4993376c`; part 5a and part 5b are compiled, specified here, and proved by
real-Git and loopback integration tests; the crate has its selectors, its QA
group and its documentation. Unit 5 is complete in Longhorn.

Step 5, consumer migration, needs a Longhorn release after this merges and
Tom's approval, then the Acowtancy Supervisor routes the Desktop change;
Longhorn does not write to Acowtancy.

Step 6 is done by the neutral second consumer at
`examples/desktop-support-conformance/` (task 078, Tom approved 2026-10-23). A
consumer crate staged from the packaged crates into a disposable private
workspace replays this contract's rows through the public API of parts 5a and
5b, with real local Git and no network: a clone into an invocation-owned
staging folder and through the exclusive rename, a second clone refused with the
existing workspace unchanged, a failed clone that leaves no target, worktree
status and branch reading, a missing-credential and a missing-repository
preflight refusal, a sync that fast-forwards and then pushes, a branch switch in
its switched, already-current, not-found, blocked and remote-tracking readings,
an attach in all five readings, and a loopback device-flow sign-in whose grant
lands in an in-memory store under a consumer-scoped slot, read back through
`fresh_access_token` and spent on a fetch through the askpass bridge, with the
token proved absent from the child's arguments, the failure detail and the
returned output. Limits: the flow, the token exchange and the askpass fetch use
the crate's loopback server with a local bare repository behind the remote, so
no live GitHub host, real OAuth registration or published release is involved;
the clone, sync, switch and attach rows run on temporary local repositories, so
the network and authentication classes are proved by classification and local
refusals only; the proof is one row set per reading and runs on Unix hosts only.

Open:

- Step 5 for unit 5: no Longhorn release carrying this crate has been cut and
  Acowtancy Desktop has not migrated yet.

## Stop conditions

Stop and report when:

- a donor behaviour cannot be kept without a product value in Longhorn;
- the policy hook would need a product type or a Tauri type in its signature;
- the token cannot be kept out of Git arguments, files, logs, errors or `Debug`
  output on some platform;
- the refresh cannot keep part 5a's deadline rules without changing part 5a's
  public API;
- real-Git tests cannot run without network or elevated privileges on the
  build host.
