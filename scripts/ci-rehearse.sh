#!/usr/bin/env bash
# Reproduce the five developer-machine properties that made local `effigy qa`
# green while the first release runs failed.
#
#   1. No path dependence on a sibling checkout
#   2. No dependence on a warm cargo cache holding deleted pin versions
#   3. No dependence on uncoloured tool output (CI always sets CI=1)
#   4. No dependence on tools the runner does not install (rg)
#   5. No active bun global link masking a missing published dependency
#
# Cheaper than one failed release run. Not a substitute for dispatching
# release.yml — it does not pack tarballs or exercise trusted publishing.
set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
cd "$root"

cargo_home=""
cleanup() {
  if [[ -n "$cargo_home" && -d "$cargo_home" ]]; then
    rm -rf "$cargo_home"
  fi
}
trap cleanup EXIT

step() {
  printf '\n== ci:rehearse %s ==\n' "$1"
}

step "1/5 containment (sibling paths cannot satisfy the gate)"
effigy check:repo-containment
effigy check:consumer-isolation

step "2/5 bun global-link state (a registered link is reported; an active one fails)"
# g02.026 stage 1: a `bun link` left over from Poodle development can keep a
# gate green against bytes no consumer resolves. The report makes the state
# visible on every rehearsal instead of assuming the machine is clean.
effigy check:bun-links

step "3/5 runner tools (scripts must not invoke rg)"
bun scripts/check-runner-tools.ts

step "4/5 fresh CARGO_HOME (lockfile pins must fetch without a warm cache)"
cargo_home=$(mktemp -d)
CARGO_HOME="$cargo_home" cargo fetch --locked
CARGO_HOME="$cargo_home" cargo metadata --locked --offline --format-version 1 >/dev/null
printf 'fetch + offline metadata ok under CARGO_HOME=%s\n' "$cargo_home"

step "5/5 CI=1 proofs under that CARGO_HOME (coloured vitest + cold cache)"
# Greenfield installs the registry Poodle release the manifests pin
# (poodleRelease()); it takes no source checkout. A stale sibling clone or a
# POODLE_REPO override would only feed bytes no proof reads.
CI=1 CARGO_HOME="$cargo_home" effigy proof:artifacts

printf '\nci:rehearse passed\n'
