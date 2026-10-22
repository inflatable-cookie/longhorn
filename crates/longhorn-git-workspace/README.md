# Longhorn Git Workspace

Generic Git workspace mechanism for desktop applications. Contract 029 owns its
behaviour; this crate is part 5a.

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
- `run` executes one argument vector under a hard wall clock.

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
from its head and its standard error from its tail, each within
`MAX_CAPTURED_OUTPUT_BYTES`; a failure's detail is redacted with the caller's
redactor and the invocation secret, then cut to `FAILURE_DETAIL_BOUND`.
`redact_git_detail` exposes the same treatment for the caller's own
diagnostics.

The crate names no product. Isolation, the allowed remote, branch naming, the
integration base, contract-readable root admission, and every user-facing
sentence stay with the application. Failures are `GitFailure` values that carry
a `GitFailureCode` wire string, Git's own redacted words, and no product copy.

Part 5b adds GitHub sign-in to the same crate: device flow, token exchange and
refresh, askpass, commit identity, and credential storage over contract 021's
credential slots. It is not part of this part.

Tests run real `git` against temporary local repositories and bare remotes, with
no network and no stored credential. They are integration tests under `tests/`;
`tests/support/` builds the fixtures and scripted policies they share.
