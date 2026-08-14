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

vitest_owned_tests_dir() {
  local config="packages/$1/vitest.config.ts"
  [[ -f "$config" ]] && grep -q "packages/$1/tests" "$config"
}

files=()
for dir in packages/*/tests; do
  package=$(basename "$(dirname "$dir")")
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
