#!/usr/bin/env bash
set -euo pipefail

if (( $# != 2 )); then
  echo 'usage: check-public-api.sh RUSTDOC_DIRECTORY SCHEMA_DIRECTORY' >&2
  exit 2
fi

rustdoc_dir=$1
schema_dir=$2
required_paths=(
  "$rustdoc_dir/policy_core"
  "$rustdoc_dir/policy_contracts"
  "$schema_dir"
)
for required_path in "${required_paths[@]}"; do
  if [[ ! -d "$required_path" ]]; then
    echo "required public API scan path is missing: $required_path" >&2
    exit 1
  fi
done
if ! command -v rg >/dev/null 2>&1; then
  echo 'required public API scan command is unavailable: rg' >&2
  exit 1
fi

public_pattern='(?i:gsd-core|\.planning|gemini-reviewer|\bagy\b|oh-my-pi|\bomp\b|\bmcp\b|\bhashline\b)'
set +e
matches=$(rg -n "$public_pattern" -- \
  "$rustdoc_dir/policy_core" \
  "$rustdoc_dir/policy_contracts" \
  "$schema_dir" 2>&1)
search_status=$?
set -e
case "$search_status" in
  0)
    printf '%s\n' "$matches" >&2
    echo 'forbidden public API or schema residue found' >&2
    exit 1
    ;;
  1) ;;
  *)
    printf '%s\n' "$matches" >&2
    echo "public API residue search failed with status $search_status" >&2
    exit 1
    ;;
esac

echo 'public API and schema residue scan passed'
