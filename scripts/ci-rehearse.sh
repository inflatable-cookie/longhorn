#!/usr/bin/env bash
# Reproduce the four developer-machine properties that made local `effigy qa`
# green while the first release runs failed.
#
#   1. No path dependence on a sibling checkout
#   2. No dependence on a warm cargo cache holding deleted pin versions
#   3. No dependence on uncoloured tool output (CI always sets CI=1)
#   4. No dependence on tools the runner does not install (rg)
#
# Cheaper than one failed release run. Not a substitute for dispatching
# release.yml — it does not pack tarballs or exercise trusted publishing.
set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
cd "$root"

poodle_tmp=""
cargo_home=""
cleanup() {
  if [[ -n "$cargo_home" && -d "$cargo_home" ]]; then
    rm -rf "$cargo_home"
  fi
  if [[ -n "$poodle_tmp" && -d "$poodle_tmp" ]]; then
    rm -rf "$poodle_tmp"
  fi
}
trap cleanup EXIT

step() {
  printf '\n== ci:rehearse %s ==\n' "$1"
}

step "1/4 containment (sibling paths cannot satisfy the gate)"
effigy check:repo-containment
effigy check:consumer-isolation

step "2/4 runner tools (scripts must not invoke rg)"
bun scripts/check-runner-tools.ts

step "3/4 fresh CARGO_HOME (lockfile pins must fetch without a warm cache)"
cargo_home=$(mktemp -d)
CARGO_HOME="$cargo_home" cargo fetch --locked
CARGO_HOME="$cargo_home" cargo metadata --locked --offline --format-version 1 >/dev/null
printf 'fetch + offline metadata ok under CARGO_HOME=%s\n' "$cargo_home"

step "4/4 CI=1 proofs under that CARGO_HOME (coloured vitest + cold cache)"
# Greenfield packs Poodle from source. Prefer an existing sibling or
# POODLE_REPO; otherwise clone the tagged release into a temp dir.
if [[ -z "${POODLE_REPO:-}" && ! -d "$(dirname "$root")/poodle" ]]; then
  poodle_tmp=$(mktemp -d)
  git clone --quiet --depth 1 --branch v0.1.0 \
    https://github.com/inflatable-cookie/poodle.git \
    "$poodle_tmp/poodle"
  export POODLE_REPO="$poodle_tmp/poodle"
  printf 'cloned Poodle v0.1.0 for greenfield at %s\n' "$POODLE_REPO"
fi
CI=1 CARGO_HOME="$cargo_home" effigy proof:artifacts

printf '\nci:rehearse passed\n'
