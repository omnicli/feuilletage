#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
temp_dir=$(mktemp -d)
trap 'rm -rf "$temp_dir"' EXIT

# shellcheck disable=SC1091
source "$repo_root/scripts/publish-release.sh"

expect_failure() {
  local expected=$1
  shift
  local output
  if output=$("$@" 2>&1); then
    echo "error: command unexpectedly succeeded: $*" >&2
    exit 1
  fi
  [[ "$output" == *"$expected"* ]] || {
    echo "error: expected '$expected', got: $output" >&2
    exit 1
  }
}

make_archive() {
  local name=$1
  local metadata=$2
  local root="$temp_dir/$name/feuilletage-1.2.3"
  mkdir -p "$root"
  printf '%s\n' "$metadata" > "$root/.cargo_vcs_info.json"
  printf '%s\n' "${3:-lock-$name}" > "$root/Cargo.lock"
  printf '%s\n' "${4:-source}" > "$root/lib.rs"
  printf '%s\n' "${5:-manifest}" > "$root/Cargo.toml"
  tar -czf "$temp_dir/$name.crate" -C "$temp_dir/$name" feuilletage-1.2.3
}

expected=0123456789abcdef0123456789abcdef01234567
other=89abcdef0123456789abcdef0123456789abcdef
make_archive clean "{\"git\":{\"sha1\":\"$expected\",\"dirty\":false}}"
make_archive other "{\"git\":{\"sha1\":\"$other\"}}"
make_archive dirty "{\"git\":{\"sha1\":\"$expected\",\"dirty\":true}}"
make_archive equivalent "{\"git\":{\"sha1\":\"$other\",\"dirty\":false}}" different-lock
make_archive source-diff "{\"git\":{\"sha1\":\"$expected\",\"dirty\":false}}" lock source-diff
make_archive manifest-diff "{\"git\":{\"sha1\":\"$expected\",\"dirty\":false}}" lock source manifest-diff

[[ $(published_commit_from_archive "$temp_dir/clean.crate" feuilletage 1.2.3) == "$expected" ]]
[[ $(published_commit_from_archive "$temp_dir/other.crate" feuilletage 1.2.3) == "$other" ]]
expect_failure "came from a dirty worktree" \
  published_commit_from_archive "$temp_dir/dirty.crate" feuilletage 1.2.3
archives_equivalent "$temp_dir/clean.crate" "$temp_dir/equivalent.crate" feuilletage 1.2.3
expect_failure "differs from the checked-out package" \
  require_archives_equivalent "$temp_dir/clean.crate" "$temp_dir/source-diff.crate" feuilletage 1.2.3
expect_failure "differs from the checked-out package" \
  require_archives_equivalent "$temp_dir/clean.crate" "$temp_dir/manifest-diff.crate" feuilletage 1.2.3

valid_release=$(jq -n \
  --arg tag v1.2.3 --arg title "feuilletage v1.2.3" \
  '{tag_name: $tag, name: $title, draft: false, prerelease: false}')
validate_release_json "$valid_release" v1.2.3 "feuilletage v1.2.3"
expect_failure "is a draft" validate_release_json \
  "$(jq '.draft = true' <<< "$valid_release")" v1.2.3 "feuilletage v1.2.3"
expect_failure "is a prerelease" validate_release_json \
  "$(jq '.prerelease = true' <<< "$valid_release")" v1.2.3 "feuilletage v1.2.3"
expect_failure "has title" validate_release_json \
  "$(jq '.name = "wrong"' <<< "$valid_release")" v1.2.3 "feuilletage v1.2.3"
expect_failure "references tag" validate_release_json \
  "$(jq '.tag_name = "v9.9.9"' <<< "$valid_release")" v1.2.3 "feuilletage v1.2.3"

cargo_calls=0
# Invoked indirectly by wait_for_publish_dry_run.
# shellcheck disable=SC2317
cargo() {
  cargo_calls=$((cargo_calls + 1))
  printf '%s\n' "$*" >> "$temp_dir/cargo-calls"
  ((cargo_calls >= 3))
}
CRATES_IO_WAIT_ATTEMPTS=3 CRATES_IO_WAIT_SECONDS=0 wait_for_publish_dry_run feuilletage
[[ $cargo_calls == 3 ]]
grep -Fxq 'publish --locked --dry-run -p feuilletage' "$temp_dir/cargo-calls"
unset -f cargo

cargo_info_calls=0
# Invoked indirectly by wait_for_crate_version.
# shellcheck disable=SC2317
cargo() {
  cargo_info_calls=$((cargo_info_calls + 1))
  printf '%s\n' "$*" >> "$temp_dir/cargo-info-calls"
  ((cargo_info_calls >= 3))
}
CRATES_IO_WAIT_ATTEMPTS=3 CRATES_IO_WAIT_SECONDS=0 wait_for_crate_version feuilletage 1.2.3
[[ $cargo_info_calls == 3 ]]
grep -Fxq 'info feuilletage@1.2.3 --registry crates-io' "$temp_dir/cargo-info-calls"
unset -f cargo

# Invoked indirectly by wait_for_crate_version.
# shellcheck disable=SC2317
cargo() {
  printf '%s\n' "$*" >> "$temp_dir/cargo-info-failures"
  return 1
}
CRATES_IO_WAIT_ATTEMPTS=2 CRATES_IO_WAIT_SECONDS=0 \
  expect_failure "feuilletage 1.2.3 did not resolve from crates.io" \
  wait_for_crate_version feuilletage 1.2.3
[[ $(wc -l < "$temp_dir/cargo-info-failures") == 2 ]]
unset -f cargo

# Invoked indirectly by remote_tag_commit.
# shellcheck disable=SC2317
gh() {
  if [[ ${MOCK_GH_RESULT:-} == missing ]]; then
    echo "gh: Not Found (HTTP 404)" >&2
  else
    echo "gh: service unavailable (HTTP 503)" >&2
  fi
  return 1
}

MOCK_GH_RESULT=missing
if remote_tag_commit omnicli/feuilletage v1.2.3 >/dev/null 2>&1; then
  echo "error: a missing remote tag unexpectedly resolved" >&2
  exit 1
fi
MOCK_GH_RESULT=error
expect_failure "could not query GitHub tag v1.2.3" \
  remote_tag_commit omnicli/feuilletage v1.2.3
unset -f gh

echo "publish release helper tests passed"
