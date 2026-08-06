#!/usr/bin/env bash
# Proves the release commit is consumable as a git dependency: builds a
# throwaway consumer against it and asserts every longhorn crate resolves
# from a git source rather than a path. Catches missing crates, path
# leakage, and manifests that only resolve because sibling checkouts exist.
set -euo pipefail

release_repo_root=$(cd "$(dirname "$0")/.." && pwd)
cd "$release_repo_root"

release_tmp=$(mktemp -d)
trap 'rm -rf "$release_tmp"' EXIT

release_consumer_root="$release_tmp/consumer"
mkdir -p "$release_consumer_root/src"

if [[ -z $(git status --porcelain) ]]; then
  release_source_root="$release_repo_root"
  release_source_commit=$(git rev-parse HEAD)
  release_source_kind=commit
else
  release_source_root="$release_tmp/longhorn-source"
  mkdir -p "$release_source_root"
  release_source_list="$release_tmp/source-files.txt"
  while IFS= read -r -d '' release_source_path; do
    if [[ -e "$release_source_path" || -L "$release_source_path" ]]; then
      printf '%s\0' "$release_source_path"
    fi
  done < <(git ls-files --cached --others --exclude-standard -z) \
    > "$release_source_list"

  tar --null -T "$release_source_list" -cf - |
    tar -xf - -C "$release_source_root"

  (
    cd "$release_source_root"
    git init -q
    git add -A
    GIT_AUTHOR_DATE=2000-01-01T00:00:00Z \
      GIT_COMMITTER_DATE=2000-01-01T00:00:00Z \
      git \
      -c user.name=Longhorn \
      -c user.email=source-gate@invalid \
      commit -q -m 'Source consumer verification snapshot'
    test -z "$(git status --porcelain)"
  )

  release_source_commit=$(git -C "$release_source_root" rev-parse HEAD)
  release_source_kind=snapshot
fi

release_source_url="file://$release_source_root"

# One crate per consumed domain, plus a Tauri host crate so the host path is
# proved too. Extend when a new domain gains external consumers.
release_probe_crates=(
  longhorn-core
  longhorn-config
  longhorn-windowing
  longhorn-layout
  longhorn-surfaces
  longhorn-transfer
  longhorn-history
  longhorn-operation
  longhorn-notifications
  longhorn-settings
  longhorn-command
  longhorn-bridge
  longhorn-native-content
  longhorn-tauri-config
)

{
  cat <<EOF
[package]
name = "longhorn-source-consumer"
version = "0.0.0"
edition = "2024"
publish = false
rust-version = "1.95"

[dependencies]
EOF
  for release_probe_crate in "${release_probe_crates[@]}"; do
    printf '%s = { git = "%s", rev = "%s" }\n' \
      "$release_probe_crate" "$release_source_url" "$release_source_commit"
  done
} > "$release_consumer_root/Cargo.toml"

{
  printf 'fn main() {\n'
  for release_probe_crate in "${release_probe_crates[@]}"; do
    printf '    use %s as _;\n' "${release_probe_crate//-/_}"
  done
  printf '}\n'
} > "$release_consumer_root/src/main.rs"

cargo generate-lockfile --manifest-path "$release_consumer_root/Cargo.toml"
cargo check --manifest-path "$release_consumer_root/Cargo.toml" --locked

release_metadata="$release_tmp/metadata.json"
cargo metadata \
  --manifest-path "$release_consumer_root/Cargo.toml" \
  --format-version 1 \
  --locked \
  > "$release_metadata"
# Every Longhorn package except the throwaway consumer itself must carry a git
# source at the exact commit.
#
# `select(.source != null)` *excluded* a path-resolved package instead of
# failing on it -- the leak being hunted became invisible, leaving only the
# count. Longhorn has 46 workspace crates against 14 probes, so
# thirty-two could leak to path and the count would still clear. Measured on signal
# by pointing one probe at a path: the old filter printed "external source
# consumer passed"; this one fails. `cargo check` succeeded in both, which is
# exactly what a consumer with sibling checkouts sees.
#
# Selecting by name and excluding only the throwaway consumer package -- which
# is itself prefixed and legitimately sourceless, and is why the old filter
# reached for `.source != null` -- removes the slack. The count stays as a guard
# against the probe list silently shrinking.
jq -e \
  --arg commit "$release_source_commit" \
  --argjson expected "${#release_probe_crates[@]}" '
  [
    .packages[] |
    select(.name | startswith("longhorn-")) |
    select(.name != "longhorn-source-consumer")
  ] as $packages |
  ($packages | length) >= $expected and
  all($packages[];
    (.source // "") | startswith("git+file://") and endswith("#" + $commit)
  )
' "$release_metadata" > /dev/null

printf 'external source consumer passed at exact %s %s\n' \
  "$release_source_kind" \
  "$release_source_commit"
