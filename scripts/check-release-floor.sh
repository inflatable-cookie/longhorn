#!/usr/bin/env bash
# Enforces the declared MSRV with real lints and tests, not a bare check.
# Longhorn has no per-crate MSRV override today; if one is ever added, split
# this into an excluded-workspace pass plus a per-crate pass at the higher
# toolchain (see swallowtail's check-release-floor.sh).
set -euo pipefail

release_repo_root=$(cd "$(dirname "$0")/.." && pwd)
cd "$release_repo_root"

source release-baselines/rust-toolchains.env

if ! rustup toolchain list | rg -q "^${LONGHORN_GENERAL_MSRV}(-|$)"; then
  printf 'missing required Rust toolchain: %s\n' "$LONGHORN_GENERAL_MSRV" >&2
  exit 1
fi

nice -n 5 rustup run "$LONGHORN_GENERAL_MSRV" cargo clippy \
  --workspace --all-targets --locked -- -D warnings
nice -n 5 rustup run "$LONGHORN_GENERAL_MSRV" cargo test \
  --workspace --locked

printf 'floor-toolchain Clippy and full tests passed at %s\n' \
  "$LONGHORN_GENERAL_MSRV"
