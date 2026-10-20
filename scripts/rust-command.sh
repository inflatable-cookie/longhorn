#!/usr/bin/env bash
set -euo pipefail

if (($# == 0)); then
  printf 'usage: rust-command.sh <cargo command> [args... ]\n' >&2
  exit 2
fi

if [[ "${GITHUB_ACTIONS:-}" == "true" ]]; then
  runner=cargo
else
  runner=oxide
  if ! command -v oxide >/dev/null 2>&1; then
    cat >&2 <<'MESSAGE'
oxide is required for local Rust build selectors. Install oxide or use one of these plain-Cargo escape selectors:
  effigy lint:rust:cargo
  effigy test:rust:cargo
  effigy test:pty:cargo
MESSAGE
    exit 127
  fi
fi

printf 'rust runner: %s' "$runner" >&2
printf ' %q' "$@" >&2
printf '\n' >&2
exec "$runner" "$@"
