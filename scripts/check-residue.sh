#!/usr/bin/env bash
set -euo pipefail

repo_root=${1:-"$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"}
for command_name in rg cargo; do
  if ! command -v "$command_name" >/dev/null 2>&1; then
    echo "required residue scan command is unavailable: $command_name" >&2
    exit 1
  fi
done

required_paths=(
  "$repo_root/Cargo.toml"
  "$repo_root/schemas"
  "$repo_root/action"
)
for required_path in "${required_paths[@]}"; do
  if [[ ! -e "$required_path" ]]; then
    echo "required residue scan path is missing: $required_path" >&2
    exit 1
  fi
done

if [[ ! -s "$repo_root/action/action.yml" && ! -s "$repo_root/action/action.yaml" ]]; then
  echo 'required Action manifest is missing or empty' >&2
  exit 1
fi
schema_files=("$repo_root"/schemas/*.json)
has_schema=false
for schema_file in "${schema_files[@]}"; do
  if [[ -s "$schema_file" ]]; then has_schema=true; fi
done
if [[ "$has_schema" != true ]]; then
  echo 'required schema files are missing or empty' >&2
  exit 1
fi

shopt -s nullglob
crate_src_paths=("$repo_root"/crates/*/src)
shopt -u nullglob
if (( ${#crate_src_paths[@]} == 0 )); then
  echo "required residue scan path is missing: $repo_root/crates/*/src" >&2
  exit 1
fi

runtime_pattern='(?i:gsd-core|\.planning|gemini-reviewer|\bagy\b|oh-my-pi|\bomp\b|\bmcp\b|\bhashline\b)'
scan_paths=(
  "${crate_src_paths[@]}"
  "$repo_root/schemas"
  "$repo_root/action"
  "$repo_root/Cargo.toml"
)
set +e
runtime_matches=$(rg --hidden --no-ignore -n "$runtime_pattern" -- "${scan_paths[@]}" 2>&1)
runtime_status=$?
set -e
case "$runtime_status" in
  0)
    printf '%s\n' "$runtime_matches" >&2
    echo 'forbidden runtime residue found' >&2
    exit 1
    ;;
  1) ;;
  *)
    printf '%s\n' "$runtime_matches" >&2
    echo "residue search failed with status $runtime_status" >&2
    exit 1
    ;;
esac

set +e
cargo_tree=$(cd "$repo_root" && cargo tree --locked -p policy-core 2>&1)
cargo_status=$?
set -e
if (( cargo_status != 0 )); then
  printf '%s\n' "$cargo_tree" >&2
  echo "cargo tree failed with status $cargo_status" >&2
  exit 1
fi

dependency_pattern='(^|[[:space:]│├└])(?:reqwest|octocrab|clap|tokio) v[0-9]'
set +e
dependency_matches=$(rg -n "$dependency_pattern" <<<"$cargo_tree" 2>&1)
dependency_status=$?
set -e
case "$dependency_status" in
  0)
    printf '%s\n' "$dependency_matches" >&2
    echo 'forbidden policy-core dependency found' >&2
    exit 1
    ;;
  1) ;;
  *)
    printf '%s\n' "$dependency_matches" >&2
    echo "policy-core dependency search failed with status $dependency_status" >&2
    exit 1
    ;;
esac

echo 'runtime residue scan passed'
