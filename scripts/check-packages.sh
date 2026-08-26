#!/usr/bin/env bash
set -euo pipefail

metadata=$(cargo metadata --locked --no-deps --format-version 1)
feuilletage_version=$(jq -r '.packages[] | select(.name == "feuilletage") | .version' <<< "$metadata")
macro_version=$(jq -r '.packages[] | select(.name == "feuilletage-macros") | .version' <<< "$metadata")

stable_version='^[0-9]+\.[0-9]+\.[0-9]+$'
if [[ ! "$feuilletage_version" =~ $stable_version || ! "$macro_version" =~ $stable_version ]]; then
  echo "error: checked-in package versions must be stable SemVer versions" >&2
  exit 1
fi

macro_requirement=$(jq -r '
  .packages[]
  | select(.name == "feuilletage")
  | .dependencies[]
  | select(.name == "feuilletage-macros" and .kind == null)
  | .req
' <<< "$metadata")
if [[ "$macro_requirement" != "=$macro_version" ]]; then
  echo "error: feuilletage must depend exactly on feuilletage-macros $macro_version" >&2
  exit 1
fi

cargo package --locked --workspace

echo "workspace package checks passed"
