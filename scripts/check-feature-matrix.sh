#!/usr/bin/env bash
set -euo pipefail

feature_sets=("" yaml json toml "std,json" all-formats)

for features in "${feature_sets[@]}"; do
  args=(-p feuilletage --no-default-features --locked)
  label=no-default
  if [[ -n "$features" ]]; then
    args+=(--features "$features")
    label=$features
  fi

  echo "Checking feuilletage features: $label"
  cargo check "${args[@]}" --lib
  cargo test "${args[@]}" --doc
done

echo "Checking feuilletage with all features"
cargo check -p feuilletage --all-features --locked --lib
cargo test -p feuilletage --all-features --locked --doc
