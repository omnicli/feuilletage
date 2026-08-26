#!/usr/bin/env bash
set -euo pipefail

# rustdoc counts public items that contain at least one example. This is not
# the number of doctests: one item can contain multiple doctest code blocks.
readonly MIN_EXAMPLE_ITEMS=108
readonly MIN_EXAMPLE_PERCENT=39.6

repo_root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
cd "$repo_root"

output_file=$(mktemp)
trap 'rm -f "$output_file"' EXIT

CARGO_TARGET_DIR="$repo_root/target/rustdoc-coverage" \
  RUSTC_BOOTSTRAP=1 cargo rustdoc -p feuilletage --lib --all-features --locked -- \
  -Z unstable-options --show-coverage 2>&1 | tee "$output_file"

read -r example_items example_percent < <(
  awk -F '|' '
    $2 ~ /^[[:space:]]*Total[[:space:]]*$/ {
      gsub(/[[:space:]]/, "", $5)
      gsub(/[[:space:]%]/, "", $6)
      print $5, $6
      found = 1
    }
    END { if (!found) exit 1 }
  ' "$output_file"
)

if [[ ! "$example_items" =~ ^[0-9]+$ ]] ||
  [[ ! "$example_percent" =~ ^[0-9]+([.][0-9]+)?$ ]]; then
  echo "error: could not parse rustdoc example coverage total" >&2
  exit 1
fi

failed=0
if ((example_items < MIN_EXAMPLE_ITEMS)); then
  echo "error: rustdoc example-covered public items regressed: ${example_items} < ${MIN_EXAMPLE_ITEMS}" >&2
  failed=1
fi

if ! awk -v actual="$example_percent" -v minimum="$MIN_EXAMPLE_PERCENT" \
  'BEGIN { exit !(actual >= minimum) }'; then
  echo "error: rustdoc public item example coverage regressed: ${example_percent}% < ${MIN_EXAMPLE_PERCENT}%" >&2
  failed=1
fi

if ((failed)); then
  exit 1
fi

echo "rustdoc public item example coverage: ${example_items} items, ${example_percent}% (minimum: ${MIN_EXAMPLE_ITEMS} items, ${MIN_EXAMPLE_PERCENT}%)"
