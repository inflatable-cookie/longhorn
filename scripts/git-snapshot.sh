#!/usr/bin/env bash
# Freezes the exact working tree into a Git snapshot a file:// dependency can
# resolve: clean trees resolve at HEAD; dirty trees copy every existing tracked
# and non-ignored untracked file into `<destination>` and commit them
# deterministically.
#
#   git-snapshot.sh <destination> [commit-message]
#
# Prints three lines on stdout:
#   <commit|snapshot>
#   <source-root>
#   <commit-sha>
set -euo pipefail

if (($# < 1 || $# > 2)) || [[ -z $1 ]]; then
  echo "usage: git-snapshot.sh <destination> [commit-message]" >&2
  exit 2
fi

destination=$1
commit_message=${2:-"Working-tree verification snapshot"}
repo_root=$(git rev-parse --show-toplevel)

if [[ -z $(git -C "$repo_root" status --porcelain) ]]; then
  printf 'commit\n%s\n%s\n' "$repo_root" "$(git -C "$repo_root" rev-parse HEAD)"
  exit 0
fi

mkdir -p "$destination"
destination_abs=$(cd "$destination" && pwd -P)

(
  cd "$repo_root"
  while IFS= read -r -d '' source_path; do
    if [[ -e $source_path || -L $source_path ]]; then
      printf '%s\0' "$source_path"
    fi
  done < <(git ls-files --cached --others --exclude-standard -z) |
    tar --null -T - -cf - |
    tar -xf - -C "$destination_abs"
)

(
  cd "$destination_abs"
  git init -q
  git add -A
  GIT_AUTHOR_DATE=2000-01-01T00:00:00Z \
    GIT_COMMITTER_DATE=2000-01-01T00:00:00Z \
    git \
    -c user.name=Longhorn \
    -c user.email=snapshot@invalid \
    -c commit.gpgsign=false \
    commit -q --allow-empty -m "$commit_message"
  test -z "$(git status --porcelain)"
)

snapshot_commit=$(git -C "$destination_abs" rev-parse HEAD)
printf 'snapshot\n%s\n%s\n' "$destination_abs" "$snapshot_commit"
