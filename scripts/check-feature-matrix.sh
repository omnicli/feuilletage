#!/usr/bin/env bash
set -euo pipefail

feature_sets=("" yaml json toml "std,json" all-formats)

for features in "${feature_sets[@]}"; do
  args=(-p compote --no-default-features --locked)
  label=no-default
  if [[ -n "$features" ]]; then
    args+=(--features "$features")
    label=$features
  fi

  echo "Checking compote features: $label"
  cargo check "${args[@]}" --lib
  cargo test "${args[@]}" --doc
done

echo "Checking compote with all features"
cargo check -p compote --all-features --locked --lib
cargo test -p compote --all-features --locked --doc
