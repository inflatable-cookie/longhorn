#!/usr/bin/env bash
# Longhorn's one Cargo switch point. Every route that runs Cargo or a
# Cargo-wrapping tool names this script and nothing else, so the CI-aware rule,
# the escape and the loud missing-Oxide failure have exactly one implementation.
#
#   rust-command.sh <cargo command> [args...]
#       Runs the Cargo command through the local runner (`oxide`) or plain
#       `cargo`. Non-building subcommands pass straight through either way.
#
#   rust-command.sh exec [--manifest-path <Cargo.toml>] [--profile <p>] \
#       -- <tool> [args...]
#       Runs one tool that calls Cargo itself (a proof's staged-workspace
#       build, `cargo tauri build`) in a single lease over one workspace and
#       toolchain. `--profile` and `--manifest-path` are Oxide's, so plain
#       Cargo drops them: on a runner or under the escape the tool runs
#       directly and behaves exactly as it did before this form existed.
#
# Routes:
#   GITHUB_ACTIONS=true   plain `cargo` / the tool itself. Hosted CI keeps the
#                         Cargo behavior it always had.
#   EFFIGY_OXIDE_ESCAPE=cargo
#                         plain `cargo` / the tool itself, the one local
#                         escape. Any other value is refused rather than
#                         silently ignored.
#   otherwise             `oxide` / `oxide exec`. Missing Oxide is a loud
#                         failure, never a Cargo fallback.
#
# A tool running under `exec` must call plain `cargo` inside the lease, never
# oxide again: a nested lease takes its own slot and can deadlock the host
# permits. See the routing table in AGENTS.md.
set -euo pipefail

usage() {
  cat >&2 <<'USAGE'
usage: rust-command.sh <cargo command> [args...]
       rust-command.sh exec [--manifest-path <Cargo.toml>] [--profile quick|debug|release] -- <tool> [args...]
USAGE
}

escape="${EFFIGY_OXIDE_ESCAPE:-}"
case "$escape" in
  "" | cargo) ;;
  *)
    printf 'EFFIGY_OXIDE_ESCAPE=%s is unsupported; only "cargo" is a valid escape\n' "$escape" >&2
    exit 2
    ;;
esac

mode=cargo
if [[ "${1:-}" == "exec" ]]; then
  mode=exec
  shift
fi

manifest_path=
profile=
if [[ "$mode" == "exec" ]]; then
  while (($# > 0)); do
    case "$1" in
      --manifest-path)
        if (($# < 2)); then
          usage
          exit 2
        fi
        manifest_path=$2
        shift 2
        ;;
      --profile)
        if (($# < 2)); then
          usage
          exit 2
        fi
        profile=$2
        shift 2
        ;;
      --)
        shift
        break
        ;;
      *)
        printf 'rust-command.sh exec: unexpected argument %s before --\n' "$1" >&2
        usage
        exit 2
        ;;
    esac
  done
  case "$profile" in
    "" | quick | debug | release) ;;
    *)
      printf 'rust-command.sh exec: --profile %s is not quick, debug or release\n' "$profile" >&2
      exit 2
      ;;
  esac
  if (($# == 0)); then
    usage
    exit 2
  fi
fi

if [[ "$mode" == "cargo" ]] && (($# == 0)); then
  usage
  exit 2
fi

if [[ "${GITHUB_ACTIONS:-}" == "true" || "$escape" == "cargo" ]]; then
  runner=cargo
else
  runner=oxide
  if ! command -v oxide >/dev/null 2>&1; then
    cat >&2 <<'MESSAGE'
oxide is required for local Rust routes. Install oxide, or run this route with EFFIGY_OXIDE_ESCAPE=cargo (or hand-run Cargo yourself for a one-off investigation). There is no silent Cargo fallback.
MESSAGE
    exit 127
  fi
fi

if [[ "$mode" == "cargo" ]]; then
  printf 'rust runner: %s' "$runner" >&2
  printf ' %q' "$@" >&2
  printf '\n' >&2
  exec "$runner" "$@"
fi

printf 'rust runner: %s exec' "$runner" >&2
if [[ -n "$manifest_path" ]]; then
  printf ' --manifest-path %q' "$manifest_path" >&2
fi
if [[ -n "$profile" ]]; then
  printf ' --profile %q' "$profile" >&2
fi
printf ' --' >&2
printf ' %q' "$@" >&2
printf '\n' >&2

if [[ "$runner" == "cargo" ]]; then
  exec "$@"
fi

exec_args=(exec)
if [[ -n "$manifest_path" ]]; then
  exec_args+=(--manifest-path "$manifest_path")
fi
if [[ -n "$profile" ]]; then
  exec_args+=(--profile "$profile")
fi
exec_args+=(--)
exec "$runner" "${exec_args[@]}" "$@"
