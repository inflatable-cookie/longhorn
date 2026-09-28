#!/usr/bin/env bash
# Prints the pinned stable channel from rust-toolchain.toml, the one place the
# version is declared. `ci.yml` and `release.yml` install exactly this channel
# instead of floating `stable`, so a new Clippy lint reaches Longhorn only
# through a deliberate bump. Refuses anything that is not a three-part version,
# so a malformed edit cannot silently fall back to `stable`.
set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
channel=$(sed -n 's/^channel = "\(.*\)"$/\1/p' "$root/rust-toolchain.toml")

if [[ ! "$channel" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
  printf 'rust-toolchain.toml must declare a channel like "1.97.1"; got: %s\n' \
    "${channel:-<none>}" >&2
  exit 1
fi

printf '%s\n' "$channel"
