#!/usr/bin/env bash
# Prints a fresh private Cargo output directory for one shell route.
#
#   external-scratch.sh <label>
#
# Locally, this is thin glue over `taskScratch` in `scripts/rust-command.ts`,
# the one implementation of the external-scratch rule: the root comes from
# OXIDE_TASK_SCRATCH_ROOT or the removable pool Oxide reports, must already
# exist, and only its own `longhorn` child is ever created. A root on the boot
# volume, the account home, a drive root, a top-level directory or a volume
# mount root is refused. Hosted CI uses runner-owned temporary storage directly
# so this pure Rust route does not need Bun.
#
# The caller owns the printed directory and removes it when done.
set -euo pipefail

if (($# != 1)) || [[ ! $1 =~ ^[A-Za-z0-9._-]+$ ]]; then
  echo "usage: external-scratch.sh <label>" >&2
  exit 2
fi

if [[ ${GITHUB_ACTIONS:-} == true ]]; then
  if [[ -n ${RUNNER_TEMP:-} ]]; then
    scratch_base=$RUNNER_TEMP
  elif [[ -n ${TMPDIR:-} ]]; then
    scratch_base=$TMPDIR
  else
    scratch_base=/tmp
  fi
  exec mktemp -d "${scratch_base%/}/$1-XXXXXX"
fi

here=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)
exec bun "$here/rust-command.ts" scratch "$@"
