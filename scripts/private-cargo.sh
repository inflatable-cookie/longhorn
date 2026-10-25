#!/usr/bin/env bash
# Plain Cargo for one disposable workspace, with its final outputs in a private
# directory instead of that workspace's `target/`.
#
#   private-cargo.sh --target <dir> [--build-dir] -- <cargo command> [args...]
#
# The command after `--` is written the way a route would run it, so a leading
# `cargo` names the tool and is dropped; `-- check --locked` means the same
# thing. Everything after that is Cargo's own argv, unchanged.
#
# This script makes no routing decision. It is only ever the tool behind
# `rust-command.sh exec`, so the caller has already chosen the runner
# (`oxide exec` locally, plain Cargo on a runner or under
# EFFIGY_OXIDE_ESCAPE=cargo), and calling it outside a lease would bypass the
# switch point.
#
# `--target` moves final outputs only. Intermediates keep the build directory
# the lease supplies, so they still pool: an inner CARGO_TARGET_DIR is not a
# second build store. `--build-dir` also pins CARGO_BUILD_BUILD_DIR to <dir>,
# which only the two release-absence scans use, because they read Cargo's own
# `target/<profile>/deps` layout and must keep each feature state's
# intermediates disjoint from every other build. Those two intermediates do not
# pool; the routing table in AGENTS.md names them.
set -euo pipefail

usage() {
  printf 'usage: private-cargo.sh --target <dir> [--build-dir] -- <cargo command> [args...]\n' >&2
}

target=
build_dir=0
while (($# > 0)); do
  case "$1" in
    --target)
      if (($# < 2)); then
        usage
        exit 2
      fi
      target=$2
      shift 2
      ;;
    --build-dir)
      build_dir=1
      shift
      ;;
    --)
      shift
      break
      ;;
    *)
      printf 'private-cargo.sh: unexpected argument %s before --\n' "$1" >&2
      usage
      exit 2
      ;;
  esac
done

if [[ -z "$target" ]] || (($# == 0)); then
  usage
  exit 2
fi

export CARGO_TARGET_DIR="$target"
if ((build_dir)); then
  export CARGO_BUILD_BUILD_DIR="$target"
fi

if [[ "${1:-}" == "cargo" ]]; then
  shift
fi

exec cargo "$@"
