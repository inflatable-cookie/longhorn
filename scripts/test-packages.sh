#!/usr/bin/env bash
# Runs the bun-native package suites.
#
# Package test layout is not uniform: most packages keep bun tests in tests/
# and Svelte/vitest suites in tests-svelte/, but a few (svelte, poodle,
# native-content-svelte, history-tree) have vitest configs that claim tests/
# outright. Rather than hardcode that list, derive it from the configs.
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
