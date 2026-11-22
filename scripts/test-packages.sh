#!/usr/bin/env bash
# Runs the bun-native package suites.
#
# Vitest suites live beside them under per-package vitest configs. Those
# packages are listed by hand here, so the split does not depend on what a
# config file happens to contain. A package that moves to vitest is added to
# VITEST_PACKAGES and its tests are then run by `test:vitest`.
#
# Note also that `bun test <dir>` treats its argument as a substring filter,
# not a path, so a bare tests/ argument would also match tests-svelte/.
# Explicit files are passed for that reason.
set -euo pipefail

cd "$(cd "$(dirname "$0")/.." && pwd)"

VITEST_PACKAGES=(
  longhorn-poodle-svelte
)

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
  local vitest_package
  for vitest_package in "${VITEST_PACKAGES[@]}"; do
    [[ "$vitest_package" == "$1" ]] && return 0
  done
  return 1
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
