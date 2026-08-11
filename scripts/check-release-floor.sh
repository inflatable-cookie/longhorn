#!/usr/bin/env bash
# Enforces the declared MSRV with real lints and tests, not a bare check.
# Longhorn has no per-crate MSRV override today; if one is ever added, split
# this into an excluded-workspace pass plus a per-crate pass at the higher
# toolchain (see swallowtail's check-release-floor.sh).
set -euo pipefail

release_repo_root=$(cd "$(dirname "$0")/.." && pwd)
cd "$release_repo_root"

source release-baselines/rust-toolchains.env

# grep, not rg. Under `set -o pipefail` a missing ripgrep fails the pipeline,
# which this reads as the toolchain being absent -- so on a runner without it
# the script reported a missing 1.95.0 that was in fact a missing `rg`.
if ! rustup toolchain list | grep -qE "^${LONGHORN_GENERAL_MSRV}(-|$)"; then
  printf 'missing required Rust toolchain: %s\n' "$LONGHORN_GENERAL_MSRV" >&2
  exit 1
fi

# The lockfile sync that used to live here belongs to effigy from 0.9.1:
# `sync-files = ["Cargo.lock"]` in config/release.toml runs
# `cargo update --workspace` before the gates and refuses any change beyond the
# bumped workspace version. This script no longer depends on `floor` sorting
# ahead of the other --locked gates, so gates can be renamed freely.

# Both feature passes, matching the `lint` / `lint:features` release gates:
# default features and --all-features compile disjoint code, and the floor
# claim has to cover everything a consumer can switch on.
nice -n 5 rustup run "$LONGHORN_GENERAL_MSRV" cargo clippy \
  --workspace --all-targets --locked -- -D warnings
nice -n 5 rustup run "$LONGHORN_GENERAL_MSRV" cargo clippy \
  --workspace --all-targets --all-features --locked -- -D warnings
nice -n 5 rustup run "$LONGHORN_GENERAL_MSRV" cargo test \
  --workspace --locked

printf 'floor-toolchain Clippy and full tests passed at %s\n' \
  "$LONGHORN_GENERAL_MSRV"
