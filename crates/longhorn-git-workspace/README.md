# Longhorn Git Workspace

Generic Git workspace mechanism for desktop applications. Contract 029 owns its
behaviour; this crate is parts 5a and 5b.

One `Git` handle carries the caller's `GitPolicy` and the caller's
`longhorn-redact` `Redactor`. Every reading is a method on it:

- `status` reads staged, modified, untracked and conflicted counts and the
  ahead/behind distance, and computes cleanliness once.
- `inspect_worktree` reads the canonical root, current branch, detached state,
  upstream, every local and remote-tracking branch, and the status.
- `origin_url` and `push_capability` read the remote and probe write access
  without changing either side.
- `preflight_remote` classifies one remote refusal: a missing credential, a
  refused credential, an unreachable host, an unavailable repository, or
  anything else.
- `sync_workspace` is Legacy Sync: clean, upstream-only, fetch, compare,
  fast-forward only, push only when ahead.
- `switch_branch` switches onto one branch already observed in a prior reading.
  Tracked, staged or conflicted work blocks it; untracked content does not.
- `read_attach` compares a folder against an expected branch and remote and
  returns ready, a switch offer, a dirty wrong branch, a wrong remote, or
  not-a-repository. It never mutates the checkout.
- `clone_into` clones into the invocation's own empty staging folder, bounded
  by the caller's cancellation flag.
- `run` executes one argument vector under a hard wall clock over Git
  processes and their output; the synchronous policy hooks run outside it.

Staging helpers are free functions: `target_already_present`, `staging_parent`,
`create_owned_staging`, `rename_exclusive` and `remove_owned_staging`. The
composition rule for a new workspace is the caller's: create the staging folder
beside the target, clone into it, remove it on every failure, and move it into
place only with `rename_exclusive`, which fails instead of replacing an existing
target.

Process execution is always the fixed program `git` with an argument vector, in
one explicit folder. Prompts are disabled, author and committer identity are
removed from the inherited environment, and the caller's isolation is applied
before the invocation credential. A credential is supplied by
`GitPolicy::credential` for a credential-scoped remote, or explicitly as a
`GitCredential` for a preflight or a clone. Git's standard output is retained
from its head within `MAX_CAPTURED_OUTPUT_BYTES`, and its standard error from
its head within `MAX_CAPTURED_STDERR_BYTES`, because a tail cut can hide a
private-key block's opening line. Successful standard output and every failure
detail are redacted with the caller's redactor and the invocation secret; a
failure's detail is then cut to `FAILURE_DETAIL_BOUND`, and `origin_url` and an
attach reading's observed remote are redacted at the public boundary.
`redact_git_detail` exposes the same treatment for the caller's own
diagnostics. Branch names, upstream refs and revision shorthands stay exact:
they are addresses the caller hands back, not prose.

A child runs in its own process group on Unix, so `run`'s wall clock and a
clone's cancellation kill the whole group, and the wall clock covers the drain:
a descendant that keeps an output pipe open cannot stretch either bound.

The crate names no product. Isolation, the allowed remote, branch naming, the
integration base, contract-readable root admission, and every user-facing
sentence stay with the application. Failures are `GitFailure` values that carry
a `GitFailureCode` wire string, Git's own redacted words, and no product copy.

Part 5b is GitHub sign-in in the same crate:

- `run_device_flow` runs GitHub's device and poll protocol — a one-time code,
  GitHub's pinned verification page, a five-second floor on the interval, a
  `slow_down` increase, a wait bounded by the shorter of the declared expiry and
  fifteen minutes, and the caller's cancellation flag. The mechanism opens no
  browser and authors no sentence.
- `TokenGrant`, `read_grant`, `write_grant`, `read_credential_presence` and
  `sign_out` store one grant in the caller's `longhorn-core` credential slot, in
  the version 1 JSON document contract 029 defines.
- `fresh_access_token` and `force_refresh_token` refresh inside the five-minute
  skew under one process-wide lock, so concurrent callers cause one exchange,
  and a refused refresh reports `SignInExpired` while leaving the grant in place.
- `GithubCommitIdentityResolver` derives the operator's Git name and GitHub
  noreply address from the stored grant, caching per account and per token
  digest.
- `AskpassConfig` names the token variable; `GitCredentialBridge::from_token`
  writes this crate's own askpass helper into an owner-only temporary folder and
  `credential()` hands the policy a `GitCredential`. The token reaches Git only
  as the child environment value the helper prints, and the folder is removed
  when the bridge is dropped.

The application composes 5b inside its `GitPolicy`: `credential` reads a fresh
token, builds a bridge and returns its credential; `refresh_credential` passes
the rejected credential's `secret()` to `force_refresh_token`, builds a new
bridge and returns its credential. Part 5a then retries once under the same
deadline, so a refresh never restarts the wall clock.

Tests run real `git` against temporary local repositories and bare remotes, and
drive the sign-in against a loopback fixture server with the in-memory
credential store, with no network. They are integration tests under `tests/`;
`tests/support/` builds the fixtures and scripted policies they share, and the
`fixture-server` feature supplies the loopback server and the loopback endpoint
constructors.
