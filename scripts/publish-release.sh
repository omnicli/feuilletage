#!/usr/bin/env bash
set -euo pipefail

die() {
  echo "error: $*" >&2
  exit 1
}

crate_exists() {
  local crate=$1
  local version=$2
  local response status
  response=$(mktemp)
  if ! status=$(curl --silent --show-error \
    --user-agent "omnicli/feuilletage release workflow" \
    --output "$response" --write-out '%{http_code}' \
    "https://crates.io/api/v1/crates/$crate/$version"); then
    rm -f "$response"
    die "could not query crates.io for $crate $version"
  fi
  case "$status" in
    200)
      rm -f "$response"
      return 0
      ;;
    404)
      rm -f "$response"
      return 1
      ;;
    *)
      cat "$response" >&2
      rm -f "$response"
      die "crates.io returned HTTP $status for $crate $version"
      ;;
  esac
}

wait_for_crate_version() {
  local crate=$1
  local version=$2
  local attempts=${CRATES_IO_WAIT_ATTEMPTS:-40}
  local delay=${CRATES_IO_WAIT_SECONDS:-15}
  local attempt log
  [[ "$attempts" =~ ^[1-9][0-9]*$ ]] || die "CRATES_IO_WAIT_ATTEMPTS must be positive"
  [[ "$delay" =~ ^[0-9]+$ ]] || die "CRATES_IO_WAIT_SECONDS must be non-negative"
  log=$(mktemp)
  for ((attempt = 1; attempt <= attempts; attempt++)); do
    if cargo info "$crate@$version" --registry crates-io >"$log" 2>&1; then
      rm -f "$log"
      echo "$crate $version resolves from crates.io."
      return
    fi
    if ((attempt < attempts)); then
      echo "Waiting for $crate $version to resolve from crates.io ($attempt/$attempts)."
      sleep "$delay"
    fi
  done
  cat "$log" >&2
  rm -f "$log"
  die "$crate $version did not resolve from crates.io"
}

published_commit_from_archive() {
  local archive=$1
  local crate=$2
  local version=$3
  local metadata dirty commit
  metadata=$(mktemp)
  if ! tar -xOzf "$archive" "$crate-$version/.cargo_vcs_info.json" > "$metadata" 2>/dev/null; then
    rm -f "$metadata"
    die "published $crate $version has no Cargo VCS provenance"
  fi
  if ! commit=$(jq -er '.git.sha1 | select(type == "string" and test("^[0-9a-f]{40}$"))' "$metadata"); then
    rm -f "$metadata"
    die "published $crate $version has invalid Cargo VCS provenance"
  fi
  if ! dirty=$(jq -r '
    (.git.dirty // false) as $dirty
    | if ($dirty | type) == "boolean" then $dirty else error("invalid dirty flag") end
  ' "$metadata"); then
    rm -f "$metadata"
    die "published $crate $version has invalid dirty-worktree provenance"
  fi
  rm -f "$metadata"
  [[ "$dirty" == "false" ]] || die "published $crate $version came from a dirty worktree"
  printf '%s\n' "$commit"
}

normalize_archive() {
  local archive=$1
  local crate=$2
  local version=$3
  local destination=$4
  local path root="$destination/$crate-$version"
  mkdir -p "$destination"
  while IFS= read -r path; do
    [[ "$path" == "$crate-$version" || "$path" == "$crate-$version/"* ]] ||
      die "archive for $crate $version contains an unexpected path: $path"
  done < <(tar -tzf "$archive")
  tar -xzf "$archive" -C "$destination"
  [[ -d "$root" ]] || die "archive for $crate $version has no package root"

  # Cargo generates these files while packaging. VCS provenance necessarily
  # differs by commit, and Cargo.lock can differ with the registry index even
  # when every publishable package input is identical.
  rm -f "$root/.cargo_vcs_info.json" "$root/Cargo.lock"
}

archives_equivalent() {
  local first=$1
  local second=$2
  local crate=$3
  local version=$4
  local temp_dir
  temp_dir=$(mktemp -d)
  normalize_archive "$first" "$crate" "$version" "$temp_dir/first"
  normalize_archive "$second" "$crate" "$version" "$temp_dir/second"
  if diff --no-dereference --recursive --brief \
    "$temp_dir/first/$crate-$version" "$temp_dir/second/$crate-$version" >/dev/null; then
    rm -rf "$temp_dir"
    return 0
  fi
  rm -rf "$temp_dir"
  return 1
}

require_archives_equivalent() {
  local published=$1
  local local_archive=$2
  local crate=$3
  local version=$4
  archives_equivalent "$published" "$local_archive" "$crate" "$version" ||
    die "published $crate $version differs from the checked-out package"
}

download_published_archive() {
  local crate=$1
  local version=$2
  local output=$3
  curl --fail --location --silent --show-error \
    --user-agent "omnicli/feuilletage release workflow" \
    --output "$output" \
    "https://crates.io/api/v1/crates/$crate/$version/download"
}

package_local_archive() {
  local crate=$1
  local version=$2
  local output=$3
  local target_directory archive
  cargo package --locked --no-verify -p "$crate" >/dev/null
  target_directory=$(cargo metadata --locked --no-deps --format-version 1 | jq -r '.target_directory')
  archive="$target_directory/package/$crate-$version.crate"
  [[ -f "$archive" ]] || die "cargo package did not create $archive"
  cp "$archive" "$output"
}

wait_for_publish_dry_run() {
  local crate=$1
  local attempts=${CRATES_IO_WAIT_ATTEMPTS:-40}
  local delay=${CRATES_IO_WAIT_SECONDS:-15}
  local attempt log
  [[ "$attempts" =~ ^[1-9][0-9]*$ ]] || die "CRATES_IO_WAIT_ATTEMPTS must be positive"
  [[ "$delay" =~ ^[0-9]+$ ]] || die "CRATES_IO_WAIT_SECONDS must be non-negative"
  log=$(mktemp)
  for ((attempt = 1; attempt <= attempts; attempt++)); do
    if cargo publish --locked --dry-run -p "$crate" >"$log" 2>&1; then
      rm -f "$log"
      echo "$crate passed a registry-resolving publication dry run."
      return
    fi
    if ((attempt < attempts)); then
      echo "Waiting for $crate dependencies to resolve from crates.io ($attempt/$attempts)."
      sleep "$delay"
    fi
  done
  cat "$log" >&2
  rm -f "$log"
  die "$crate did not pass a registry-resolving publication dry run"
}

remote_tag_commit() {
  local repository=$1
  local tag=$2
  local ref_json object_type object_sha tag_json error_file error
  error_file=$(mktemp)
  if ! ref_json=$(gh api "repos/$repository/git/ref/tags/$tag" 2>"$error_file"); then
    error=$(<"$error_file")
    rm -f "$error_file"
    [[ "$error" == *"HTTP 404"* ]] && return 1
    die "could not query GitHub tag $tag: $error"
  fi
  rm -f "$error_file"
  object_type=$(jq -r '.object.type' <<< "$ref_json")
  object_sha=$(jq -r '.object.sha' <<< "$ref_json")
  [[ "$object_type" == "tag" ]] || die "existing tag $tag is not annotated"
  tag_json=$(gh api "repos/$repository/git/tags/$object_sha")
  [[ $(jq -r '.object.type' <<< "$tag_json") == "commit" ]] || die "tag $tag does not point directly to a commit"
  jq -r '.object.sha' <<< "$tag_json"
}

ensure_tag() {
  local repository=$1
  local tag=$2
  local title=$3
  local commit=$4
  local existing payload tag_object
  if existing=$(remote_tag_commit "$repository" "$tag"); then
    [[ "$existing" == "$commit" ]] || die "tag $tag points to $existing, not release commit $commit; refusing to move it"
    echo "Annotated tag $tag already points to $commit."
    return
  fi

  payload=$(jq -n \
    --arg tag "$tag" --arg message "$title" --arg object "$commit" \
    '{tag: $tag, message: $message, object: $object, type: "commit"}')
  tag_object=$(gh api --method POST "repos/$repository/git/tags" --input - <<< "$payload")
  payload=$(jq -n --arg ref "refs/tags/$tag" --arg sha "$(jq -r '.sha' <<< "$tag_object")" \
    '{ref: $ref, sha: $sha}')
  if ! gh api --method POST "repos/$repository/git/refs" --input - <<< "$payload" >/dev/null; then
    existing=$(remote_tag_commit "$repository" "$tag") || die "failed to create annotated tag $tag"
    [[ "$existing" == "$commit" ]] || die "tag $tag was concurrently created at $existing, not $commit"
  fi
  echo "Created annotated tag $tag at $commit."
}

validate_release_json() {
  local release_json=$1
  local tag=$2
  local title=$3
  local actual_tag actual_title draft prerelease
  actual_tag=$(jq -er '.tag_name | select(type == "string")' <<< "$release_json") ||
    die "GitHub Release $tag has invalid tag metadata"
  actual_title=$(jq -er '.name | select(type == "string")' <<< "$release_json") ||
    die "GitHub Release $tag has invalid title metadata"
  draft=$(jq -er 'if (.draft | type) == "boolean" then (.draft | tostring) else error("invalid") end' <<< "$release_json") ||
    die "GitHub Release $tag has invalid draft metadata"
  prerelease=$(jq -er 'if (.prerelease | type) == "boolean" then (.prerelease | tostring) else error("invalid") end' <<< "$release_json") ||
    die "GitHub Release $tag has invalid prerelease metadata"
  [[ "$actual_tag" == "$tag" ]] || die "GitHub Release $tag references tag $actual_tag"
  [[ "$actual_title" == "$title" ]] || die "GitHub Release $tag has title '$actual_title', not '$title'"
  [[ "$draft" == false ]] || die "GitHub Release $tag is a draft"
  [[ "$prerelease" == false ]] || die "GitHub Release $tag is a prerelease"
}

existing_release_json() {
  local repository=$1
  local tag=$2
  local error_file error release_json
  error_file=$(mktemp)
  if ! release_json=$(gh api "repos/$repository/releases/tags/$tag" 2>"$error_file"); then
    error=$(<"$error_file")
    rm -f "$error_file"
    [[ "$error" == *"HTTP 404"* ]] && return 1
    die "could not query GitHub Release $tag: $error"
  fi
  rm -f "$error_file"
  printf '%s\n' "$release_json"
}

ensure_release() {
  local repository=$1
  local tag=$2
  local title=$3
  local commit=$4
  local existing_tag release_json
  existing_tag=$(remote_tag_commit "$repository" "$tag") || die "release tag $tag is missing"
  [[ "$existing_tag" == "$commit" ]] ||
    die "release tag $tag points to $existing_tag, not release commit $commit"
  if release_json=$(existing_release_json "$repository" "$tag"); then
    validate_release_json "$release_json" "$tag" "$title"
    echo "GitHub Release $tag already exists with the expected metadata."
    return
  fi
  if ! gh release create "$tag" --repo "$repository" --verify-tag \
    --target "$commit" --title "$title" --notes "Release of \`$title\`."; then
    release_json=$(existing_release_json "$repository" "$tag") || die "failed to create GitHub Release $tag"
  else
    release_json=$(existing_release_json "$repository" "$tag") || die "created GitHub Release $tag could not be read back"
  fi
  validate_release_json "$release_json" "$tag" "$title"
  echo "Created GitHub Release $tag."
}

if [[ ${BASH_SOURCE[0]} != "$0" ]]; then
  return 0
fi

release_commit=${RELEASE_COMMIT:?RELEASE_COMMIT is required}
repository=${RELEASE_REPOSITORY:?RELEASE_REPOSITORY is required}
[[ "$release_commit" =~ ^[0-9a-f]{40}$ ]] || die "RELEASE_COMMIT must be a full lowercase SHA-1"
[[ $(git rev-parse HEAD) == "$release_commit" ]] || die "checkout does not match release commit $release_commit"
[[ -z $(git status --porcelain) ]] || die "release checkout is dirty"
[[ -n ${CARGO_REGISTRY_TOKEN:-} ]] || die "OIDC crates.io token is missing"
[[ -n ${GH_TOKEN:-} ]] || die "GitHub token is missing"

metadata=$(cargo metadata --locked --no-deps --format-version 1)
feuilletage_version=$(jq -r '.packages[] | select(.name == "feuilletage") | .version' <<< "$metadata")
macro_version=$(jq -r '.packages[] | select(.name == "feuilletage-macros") | .version' <<< "$metadata")
release_temp=$(mktemp -d)
trap 'rm -rf "$release_temp"' EXIT

declare -A selected=()
declare -A needs_publish=()
for package in feuilletage-macros feuilletage; do
  if [[ "$package" == "feuilletage-macros" ]]; then
    version=$macro_version
  else
    version=$feuilletage_version
  fi
  if crate_exists "$package" "$version"; then
    published_archive="$release_temp/published-$package-$version.crate"
    local_archive="$release_temp/local-$package-$version.crate"
    download_published_archive "$package" "$version" "$published_archive"
    source_commit=$(published_commit_from_archive "$published_archive" "$package" "$version")
    package_local_archive "$package" "$version" "$local_archive"
    require_archives_equivalent "$published_archive" "$local_archive" "$package" "$version"
    needs_publish[$package]=false
    if [[ "$source_commit" == "$release_commit" ]]; then
      selected[$package]=true
      echo "$package $version is already published from this release commit; release records will be reconciled."
    else
      selected[$package]=false
      echo "$package $version is equivalent to the package published from $source_commit; this unchanged package will be skipped."
    fi
  else
    selected[$package]=true
    needs_publish[$package]=true
    echo "$package $version is unpublished and will be published from $release_commit."
  fi
done

if [[ ${needs_publish[feuilletage-macros]} == true ]]; then
  cargo publish --locked -p feuilletage-macros
fi

if [[ ${needs_publish[feuilletage]} == true ]]; then
  wait_for_publish_dry_run feuilletage
  cargo publish --locked -p feuilletage
fi

for package in feuilletage-macros feuilletage; do
  [[ ${selected[$package]} == true ]] || continue
  if [[ "$package" == "feuilletage-macros" ]]; then
    version=$macro_version
    tag="macros-v$version"
  else
    version=$feuilletage_version
    tag="v$version"
  fi
  title="$package v$version"
  ensure_tag "$repository" "$tag" "$title" "$release_commit"
  ensure_release "$repository" "$tag" "$title" "$release_commit"
done

[[ -z $(git status --porcelain) ]] || die "release checkout became dirty"
echo "Release reconciliation completed for commit $release_commit."
