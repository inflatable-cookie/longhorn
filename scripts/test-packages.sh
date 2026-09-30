#!/usr/bin/env bash
# Runs the bun-native package suites.
#
# Vitest suites live beside them under per-package vitest configs, so the
# split is derived from the configs rather than a hardcoded package list.
#
# Note also that `bun test <dir>` treats its argument as a substring filter,
# not a path, so a bare tests/ argument would also match tests-svelte/.
# Explicit files are passed for that reason.
set -euo pipefail

cd "$(cd "$(dirname "$0")/.." && pwd)"

requested_package="${1:-}"
if [[ $# -gt 1 ]]; then
  echo 'usage: scripts/test-packages.sh [package-directory]' >&2
  exit 2
fi
if [[ -n "$requested_package" && ! -d "packages/$requested_package/tests" ]]; then
  echo "no Bun-native package tests for $requested_package" >&2
  exit 2
fi

vitest_owned_tests_dir() {
  local config="packages/$1/vitest.config.ts"
  [[ -f "$config" ]] && grep -q "packages/$1/tests" "$config"
}

files=()
for dir in packages/*/tests; do
  package=$(basename "$(dirname "$dir")")
  [[ -n "$requested_package" && "$package" != "$requested_package" ]] && continue
  vitest_owned_tests_dir "$package" && continue
  while IFS= read -r file; do
    files+=("$file")
  done < <(find "$dir" -name '*.test.ts')
done

if [[ ${#files[@]} -eq 0 ]]; then
  echo 'no bun-native package tests found' >&2
  exit 1
fi

exec bun test "${files[@]}"
