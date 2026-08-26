#!/usr/bin/env bash
set -euo pipefail

mapfile -t examples < <(
  cargo metadata --locked --no-deps --format-version 1 |
    jq -r '.packages[] | select(.name == "feuilletage") | .targets[] | select(.kind | index("example")) | .name' |
    sort
)

if ((${#examples[@]} == 0)); then
  echo "error: cargo metadata found no Feuilletage examples" >&2
  exit 1
fi

for example in "${examples[@]}"; do
  echo "Running example: $example"
  cargo run --locked -p feuilletage --example "$example" --all-features
done
