#!/usr/bin/env bash
# Prints a fresh private Cargo output directory for one shell route.
#
#   external-scratch.sh <label>
#
# Thin glue over `taskScratch` in `scripts/rust-command.ts`, the one
# implementation of the scratch rule: the root comes from
# OXIDE_TASK_SCRATCH_ROOT or the removable pool Oxide reports, must already
# exist, and only its own `longhorn` child is ever created. A root on the boot
# volume, the account home, a drive root, a top-level directory or a volume
# mount root is refused. Hosted CI keeps its runner-owned temporary storage.
#
# The caller owns the printed directory and removes it when done.
set -euo pipefail

here=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)
exec bun "$here/rust-command.ts" scratch "$@"
