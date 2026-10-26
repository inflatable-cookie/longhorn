#!/usr/bin/env bash
# Prints a fresh private Cargo output directory for one shell route.
#
#   external-scratch.sh <label>
#
# The shell counterpart of `taskScratch` in `scripts/rust-command.ts`, so the
# two kinds of route share one rule: the scratch root must already exist, only
# its own `longhorn` child is ever created, and a root inside the internal temp
# directory, the account home, a boot-volume top-level directory or a volume
# mount root is refused. Nothing here creates a missing build volume, so an
# unmounted `/Volumes` cannot become an ordinary directory on the internal
# drive. Hosted CI keeps its runner-owned temporary storage.
#
# The caller owns the printed directory and removes it when done.
set -euo pipefail

if (($# != 1)) || [[ ! "${1:-}" =~ ^[A-Za-z0-9._-]+$ ]]; then
  printf 'usage: external-scratch.sh <label>\n' >&2
  exit 2
fi
label=$1

if [[ "${GITHUB_ACTIONS:-}" == "true" ]]; then
  runner_temp=${TMPDIR:-/tmp}
  mktemp -d "${runner_temp%/}/${label}-XXXXXX"
  exit 0
fi

fail() {
  printf 'build scratch %s\n' "$1" >&2
  exit 1
}

configured=${OXIDE_TASK_SCRATCH_ROOT:-}
if [[ -n "$configured" ]]; then
  owner=$configured
  base=$owner
else
  owner=/Volumes/Builds/scratch
  base="$owner/longhorn"
fi

if [[ ! -d "$owner" ]]; then
  fail "$owner does not exist. No local build output is created on the internal drive, the account home or a drive root; set OXIDE_TASK_SCRATCH_ROOT to an existing directory on a build volume."
fi

resolved=$(cd "$owner" && pwd -P)
internal_temp=$(cd "${TMPDIR:-/tmp}" 2>/dev/null && pwd -P) || internal_temp=
account_home=$(cd "${HOME:-/nonexistent}" 2>/dev/null && pwd -P) || account_home=

if [[ "$resolved" == "/" ]]; then
  fail "$resolved is a drive root. Local build outputs stay on external build scratch, never there."
fi
if [[ "$(dirname "$resolved")" == "/" ]]; then
  fail "$resolved is a boot-volume top-level directory. Local build outputs stay on external build scratch, never there."
fi
if [[ "$resolved" =~ ^/Volumes/[^/]+$ ]]; then
  fail "$resolved is a volume mount root. Local build outputs stay on external build scratch, never there."
fi
check_root() {
  local root=$1
  local kind=$2
  if [[ -z "$root" ]]; then
    return 0
  fi
  if [[ "$resolved" == "$root" || "$resolved" == "$root"/* ]]; then
    fail "$resolved is $kind. Local build outputs stay on external build scratch, never there."
  fi
}
check_root "$internal_temp" "the internal temporary directory"
check_root "$account_home" "the account home"

# One level below an existing owner root, never a recursive chain.
if ! mkdir "$base" 2>/dev/null; then
  [[ -d "$base" ]] || fail "cannot create $base; its parent must already be an external build scratch root."
fi

mktemp -d "$base/${label}-XXXXXX"
