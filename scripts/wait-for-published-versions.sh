#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)

# shellcheck disable=SC1091
source "$repo_root/scripts/publish-release.sh"

[[ -z $(git status --porcelain) ]] || die "release PR preparation checkout is dirty"

metadata=$(cargo metadata --locked --no-deps --format-version 1)
for package in compote-macros compote; do
  version=$(jq -er --arg package "$package" \
    '.packages[] | select(.name == $package) | .version' <<< "$metadata")
  wait_for_crate_version "$package" "$version"
done

[[ -z $(git status --porcelain) ]] || die "release PR preparation checkout became dirty"
echo "All checked-in package versions resolve from crates.io."
