#!/usr/bin/env bash
set -euo pipefail

target="${1:-thumbv7em-none-eabi}"

if ! rustup target list --installed | grep -Fxq "$target"; then
  echo "error: Rust target '$target' is not installed" >&2
  echo "install it with: rustup target add $target" >&2
  exit 1
fi

cargo check -p compote --lib --no-default-features --locked --target "$target"
cargo check \
  --manifest-path compote/tests/no_std_consumer/Cargo.toml \
  --locked \
  --target "$target" \
  --target-dir target/no_std_consumer
