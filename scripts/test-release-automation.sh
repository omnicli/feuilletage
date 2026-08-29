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

detect_release_pr() {
  local pull_requests=$1
  jq -r \
    --arg sha "$expected" --arg repository omnicli/feuilletage --arg app_login 'feuilletage-release[bot]' '
    any(.[];
      .base.ref == "main"
      and (.head.ref | startswith("release-plz-"))
      and .head.repo.full_name == $repository
      and .user.login == $app_login
      and .merged_at != null
      and .merge_commit_sha == $sha
    )
  ' <<< "$pull_requests"
}

release_pr_fixture=$(jq -n \
  --arg sha "$expected" \
  '[{base: {ref: "main"}, head: {ref: "release-plz-2026-08-26", repo: {full_name: "omnicli/feuilletage"}}, user: {login: "feuilletage-release[bot]"}, merged_at: "2026-08-26T12:00:00Z", merge_commit_sha: $sha}]')
[[ $(detect_release_pr "$release_pr_fixture") == true ]]
[[ $(detect_release_pr "$(jq '.[0].head.repo.full_name = "attacker/feuilletage"' <<< "$release_pr_fixture")") == false ]]
[[ $(detect_release_pr "$(jq '.[0].user.login = "attacker"' <<< "$release_pr_fixture")") == false ]]
[[ $(detect_release_pr "$(jq '.[0].merge_commit_sha = "0000000000000000000000000000000000000000"' <<< "$release_pr_fixture")") == false ]]

workflow="$repo_root/.github/workflows/release.yml"
config="$repo_root/release-plz.toml"
readiness="$repo_root/scripts/wait-for-published-versions.sh"

job_block() {
  local job=$1
  awk -v header="  $job:" '
    $0 == header { in_job = 1 }
    in_job && $0 != header && /^  [[:alnum:]_-]+:/ { exit }
    in_job { print }
  ' "$workflow"
}

release_context_job=$(job_block release-context)
publish_job=$(job_block publish)
release_pr_job=$(job_block release-pr)

grep -Fq 'git_release_enable = false' "$config"
grep -Fq 'git_tag_enable = false' "$config"
grep -Fq 'publish = false' "$config"
grep -Eq '^  workflow_dispatch:[[:space:]]*$' "$workflow"
grep -Fq "github.event_name == 'push'" <<< "$release_context_job"
grep -Fq "github.ref == 'refs/heads/main'" <<< "$release_context_job"
grep -Fq ".merge_commit_sha == \$sha" "$workflow"
# These are literal jq and Actions expressions, not shell expansions.
# shellcheck disable=SC2016
grep -Fq '.head.repo.full_name == $repository' "$workflow"
# shellcheck disable=SC2016
grep -Fq '.user.login == $app_login' "$workflow"
# shellcheck disable=SC2016
grep -Fq 'APP_LOGIN: ${{ vars.RELEASE_PR_APP_LOGIN }}' "$workflow"
# shellcheck disable=SC2016
grep -Fq 'APP_ID: ${{ secrets.OMNICLI_APP_ID }}' "$workflow"
# shellcheck disable=SC2016
grep -Fq 'APP_PRIVATE_KEY: ${{ secrets.OMNICLI_PRIVATE_KEY }}' "$workflow"
# shellcheck disable=SC2016
grep -Fq 'app-id: ${{ secrets.OMNICLI_APP_ID }}' "$workflow"
# shellcheck disable=SC2016
grep -Fq 'private-key: ${{ secrets.OMNICLI_PRIVATE_KEY }}' "$workflow"
grep -Fq 'rust-lang/crates-io-auth-action@c6f97d42243bad5fab37ca0427f495c86d5b1a18' "$workflow"
grep -Fq 'actions/create-github-app-token@bcd2ba49218906704ab6c1aa796996da409d3eb1' "$workflow"
grep -Fq 'permission-contents: write' "$workflow"
grep -Fq 'permission-pull-requests: write' "$workflow"
grep -Fq 'run: ./scripts/publish-release.sh' "$workflow"
grep -Fq 'needs: release-context' <<< "$publish_job"
grep -Fq "github.event_name == 'push'" <<< "$publish_job"
grep -Fq "github.ref == 'refs/heads/main'" <<< "$publish_job"
grep -Fq "needs.release-context.outputs.merged-release-pr == 'true'" <<< "$publish_job"
if grep -Fq "github.event_name == 'workflow_dispatch'" <<< "$publish_job"; then
  echo "error: workflow dispatch must not reach publication" >&2
  exit 1
fi
grep -Fq "github.event_name == 'workflow_dispatch'" <<< "$release_pr_job"
grep -Fq "github.ref == 'refs/heads/main'" <<< "$release_pr_job"
if grep -Fq "github.event_name == 'push'" <<< "$release_pr_job"; then
  echo "error: ordinary pushes must not prepare a release PR" >&2
  exit 1
fi
if grep -Eq '^    needs:' <<< "$release_pr_job"; then
  echo "error: dispatch preparation must not depend on skipped push-only jobs" >&2
  exit 1
fi
[[ $(grep -Fc 'uses: release-plz/action@' "$workflow") == 1 ]]
grep -Fq 'uses: release-plz/action@' <<< "$release_pr_job"
grep -Fq 'command: release-pr' <<< "$release_pr_job"
if grep -Eq '^concurrency:' "$workflow"; then
  echo "error: workflow-wide concurrency can displace a release publication event" >&2
  exit 1
fi
if awk '
  /^  publish:/ { in_publish = 1; next }
  /^  [[:alnum:]_-]+:/ { in_publish = 0 }
  in_publish && /^    concurrency:/ { found = 1 }
  END { exit !found }
' "$workflow"; then
  echo "error: publish-job concurrency can displace a release publication event" >&2
  exit 1
fi
[[ $(grep -c '^    concurrency:' "$workflow") == 1 ]]
grep -Fq 'concurrency:' <<< "$release_pr_job"
grep -Fq 'group: release-plz-main' "$workflow"
grep -Fq 'cancel-in-progress: true' "$workflow"
grep -Fq 'git fetch --force --prune origin +refs/heads/main:refs/remotes/origin/main' "$workflow"
grep -Fq 'git checkout --detach refs/remotes/origin/main' "$workflow"
# This is a literal workflow command, not a shell expansion.
# shellcheck disable=SC2016
grep -Fq 'test -z "$(git status --porcelain)"' "$workflow"
grep -Fq './scripts/wait-for-published-versions.sh' "$workflow"
# These are literal helper calls, not shell expansions.
# shellcheck disable=SC2016
grep -Fq 'source "$repo_root/scripts/publish-release.sh"' "$readiness"
# shellcheck disable=SC2016
grep -Fq 'wait_for_crate_version "$package" "$version"' "$readiness"
grep -Fq 'for package in feuilletage-macros feuilletage; do' "$readiness"
grep -Fq 'cargo metadata --locked --no-deps --format-version 1' "$readiness"
if grep -Fq 'cargo publish' "$readiness"; then
  echo "error: the release-readiness guard must not publish" >&2
  exit 1
fi
refresh_line=$(grep -n 'git fetch --force --prune origin' "$workflow" | cut -d: -f1)
gate_line=$(grep -n './scripts/wait-for-published-versions.sh' "$workflow" | cut -d: -f1)
release_plz_line=$(grep -n 'uses: release-plz/action@' "$workflow" | cut -d: -f1)
((refresh_line < gate_line && gate_line < release_plz_line))
if grep -Eq '^[[:space:]]*command: release$' "$workflow"; then
  echo "error: release-plz must not perform publication" >&2
  exit 1
fi
obsolete_release_gate="RELEASE_PLZ_"'ENABLED'
if git -C "$repo_root" grep -n "$obsolete_release_gate"; then
  echo "error: obsolete release enable flag remains in tracked files" >&2
  exit 1
fi
if grep -Fq -- '--allow-dirty' "$workflow" "$repo_root/scripts/publish-release.sh"; then
  echo "error: dirty package publication must not be enabled" >&2
  exit 1
fi
grep -Fq 'validate_release_json' "$repo_root/scripts/publish-release.sh"
grep -Fq 'declare -A selected=()' "$repo_root/scripts/publish-release.sh"
grep -Fq 'declare -A needs_publish=()' "$repo_root/scripts/publish-release.sh"
# Existing packages never need another upload in the same run.
# shellcheck disable=SC2016
grep -Fq 'needs_publish[$package]=false' "$repo_root/scripts/publish-release.sh"
# Missing packages must keep their initial upload decision through publication.
# shellcheck disable=SC2016
grep -Fq 'needs_publish[$package]=true' "$repo_root/scripts/publish-release.sh"
# shellcheck disable=SC2016
grep -Fq 'if [[ ${needs_publish[feuilletage-macros]} == true ]]; then' "$repo_root/scripts/publish-release.sh"
# shellcheck disable=SC2016
grep -Fq 'if [[ ${needs_publish[feuilletage]} == true ]]; then' "$repo_root/scripts/publish-release.sh"
if grep -Eq 'needs_publish\[[^]]+\].*! crate_exists|selected\[[^]]+\].*! crate_exists' \
  "$repo_root/scripts/publish-release.sh"; then
  echo "error: an upload decision must not be changed by a second registry probe" >&2
  exit 1
fi
# This assertion intentionally matches the literal implementation call.
# shellcheck disable=SC2016
grep -Fq 'remote_tag_commit "$repository" "$tag"' "$repo_root/scripts/publish-release.sh"

echo "release automation tests passed"
