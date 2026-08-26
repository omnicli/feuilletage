#!/usr/bin/env bash
set -euo pipefail

metadata=$(cargo metadata --locked --no-deps --format-version 1)
compote_version=$(jq -r '.packages[] | select(.name == "compote") | .version' <<< "$metadata")
macro_version=$(jq -r '.packages[] | select(.name == "compote-macros") | .version' <<< "$metadata")

stable_version='^[0-9]+\.[0-9]+\.[0-9]+$'
if [[ ! "$compote_version" =~ $stable_version || ! "$macro_version" =~ $stable_version ]]; then
  echo "error: checked-in package versions must be stable SemVer versions" >&2
  exit 1
fi

macro_requirement=$(jq -r '
  .packages[]
  | select(.name == "compote")
  | .dependencies[]
  | select(.name == "compote-macros" and .kind == null)
  | .req
' <<< "$metadata")
if [[ "$macro_requirement" != "=$macro_version" ]]; then
  echo "error: compote must depend exactly on compote-macros $macro_version" >&2
  exit 1
fi

cargo package --locked --workspace

echo "workspace package checks passed"
