# 029 Git Workspace

Status: active compiled boundary for part 5a
Owner: Longhorn maintainers
Created: 2026-10-22
Depends on: contracts 001, 003, 012, 024 and 025
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

Tom approved unit 5's plan on 2026-10-22 and split it in two. Part 5a is this
contract: execution with the policy hook, status, remote-refusal preflight,
Legacy Sync and branch switch, clone with staging and an exclusive rename, and
attach. Part 5b adds GitHub sign-in to the same crate and is not yet specified.

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
  environment values carry it;
- the allowed remote, the operator branch naming scheme, and the integration
  base a new branch starts from;
- contract-readable root admission, and which root is admitted for a read;
- the outcome projection, progress, and every message a person reads;
- Tauri commands, event transport and TypeScript bindings.

No product constant, product module, Tauri type or product sentence enters this
crate. A product decision reaches it only as an argument, a `GitPolicy`
implementation, or a `GitCredential` value.

## Public API

Crate `longhorn-git-workspace`, at
[`crates/longhorn-git-workspace`](../../../crates/longhorn-git-workspace/README.md).
It depends on `longhorn-redact` and `longhorn-support-log` for output
treatment, `secrecy` for the credential value, `serde` for wire projections,
`uuid` for staging names, and `rustix` on Linux and macOS for the exclusive
rename. It has no Tauri, no HTTP client, no credential store, no shell and no
`unsafe` code.

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
  and the value to keep out of every detail. `env_vars()` reads the pairs.

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
  original failure.
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
and is reported as `Cancelled`. Standard error keeps its tail within the
capture bound; its refusal detail is redacted with the caller's redactor and
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
  local repositories and bare remotes, with no network and no stored
  credential. They cover status, conflicts, ahead and behind, detached
  checkouts, a preflight refusal per class, the ambient-helper rule, sync in
  every stop state, branch switching, clone staging and the exclusive rename
  including an existing target and a failed clone, attach in every reading, the
  credential hook and its delivered environment, redaction of successful
  standard output, of an attach reading's observed remote and of failure
  detail, a private-key block longer than any tail window, the capture bound
  and its withheld-overflow note, and the hard wall clock both against a
  sleeping descendant that holds the output pipes and against a Git process
  that exits before its backgrounded descendant does. A clone against a remote
  that hangs proves cancellation returns at its flag.
- `effigy lint:git-workspace` compiles every target with clippy at `-D
  warnings`; `effigy docs:git-workspace` builds rustdoc with warnings denied.
- `effigy check:git-workspace` aggregates those three and
  `check:api-reference`; the `git-workspace` QA group selects it.

Limits. No test reaches a real host, so the network and authentication classes
are proved by their classification and by local refusals, not by a live
refusal. An offline host cannot provoke a real authentication refusal, so the
refresh-and-retry path is compiled and reviewed but not exercised; the scripted
policy proves the credential hook, its delivered environment and its failure
mode. The exclusive rename is proved on Linux and macOS only; elsewhere the
function reports `Unsupported`. Process-group termination is Unix only; on a
host without it the direct child is killed, so a descendant that keeps an
output pipe open can delay cleanup there. One invocation retains at most 64 KiB
of standard output and 1 MiB of standard error, plus the redaction copies of
those bytes, for the length of the call.

## Part 5b: GitHub sign-in

Not yet specified. Part 5b adds the GitHub device flow, token exchange and
refresh, the askpass bridge and its private program, commit identity, and
credential storage over contract 021's credential slots. `GitCredential` stays
the boundary type the mechanism sees; how the application builds one is 5b's
subject. Nothing in this part's API promises 5b's shape.

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
- `GitCredentialBridge`, its temporary directory, the askpass program bytes,
  the token variable name and `AskpassConfig` are part 5b's subject. Part 5a's
  `GitCredential` is the environment pair list plus the secret to redact.
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
  in part 5a. The merge-tree fallback and blob reads belong to the
  application's readiness and issuance flows; a commit identity arrives with
  5b.
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
- The crate has no `sha2` and no `longhorn-core` dependency: exact-path commit,
  review fingerprints and credential slots stay with the application until
  part 5b and their own tasks.
- `longhorn-redact` and `longhorn-support-log` replace the donor's local
  redaction call, so the application's own secrets participate through the
  `Redactor` it hands the handle.

## Extraction state

Contract 003 steps 1 to 4 are done for this part. The donor is admitted at
`4993376c`, the crate exists with its contract, its public API, its real-Git
integration tests, its selectors, its QA group and its documentation. Step 5,
consumer migration, needs a Longhorn release after this merges and Tom's
approval, then the Acowtancy Supervisor routes the Desktop change; Longhorn does
not write to Acowtancy. Step 6's second-consumer question is open for this
unit: no second consumer is named, and contract 003's caveat applies. Part 5b
must merge before unit 5 can be called complete.

## Stop conditions

Stop and report when:

- a donor behaviour cannot be kept without a product value in Longhorn;
- the policy hook would need a product type or a Tauri type in its signature;
- real-Git tests cannot run without network or elevated privileges on the
  build host.
