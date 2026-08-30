# Releasing

Feuilletage uses [release-plz](https://release-plz.dev/) only to prepare version,
changelog, exact local dependency, and lockfile updates in a pull request. The
repository-owned publisher handles crates.io uploads, package tags, and GitHub
Releases after that pull request is merged.

## Release model

`.github/workflows/release.yml` has three explicitly separate entry points:

- a manual `workflow_dispatch` on `main` prepares or updates the release PR;
- a push to `main` can only detect and publish an authorized merged release PR;
- a manual dispatch with a release PR number recovers publication after a
  missed or failed merge-push run.

The workflow enforces this model as follows:

1. A maintainer manually runs the workflow on `main` to prepare a release PR.
   That job resets a clean local `main` to the latest `origin/main`, requires a clean
   worktree, and waits boundedly until both exact checked-in package versions
   resolve from crates.io before invoking release-plz.
2. A short-lived GitHub App installation token creates or updates the ordinary
   release PR, which allows the repository's pull-request CI to run.
3. Every push to `main` asks GitHub which pull request produced that exact
   pushed commit, but does not run release-plz.
4. Publication is allowed only when that exact SHA is the `merge_commit_sha` of
   a merged `release-plz-*` pull request whose base is `main`, whose head branch
   is in this repository, and whose author is the configured release GitHub App.
5. `scripts/publish-release.sh` reads both checked-in package versions and
   determines explicitly whether each exact version exists on crates.io. Before
   skipping an existing immutable version, it packages the checkout and compares
   the two archives after removing only Cargo-generated VCS provenance and
   `Cargo.lock`; every source, manifest, and path difference remains fatal.
6. Missing packages are published from the clean merged commit with
   `cargo publish --locked`, never `--allow-dirty`. `feuilletage-macros` is
   published first when needed. Before publishing `feuilletage`, bounded retries of
   `cargo publish --locked --dry-run -p feuilletage` prove that the exact macro
   dependency resolves from the registry.
7. The workflow creates any missing annotated package tags and reconciles
   GitHub Releases. `feuilletage-macros` uses `macros-vX.Y.Z`; `feuilletage` uses
   `vX.Y.Z`. Existing releases must be non-draft, non-prerelease, have the exact
   title, and reference the verified tag at the release commit.

An ordinary dispatch cannot reach publication: leaving `release_pr` blank can
only prepare a release PR. Recovery requires an explicit PR number and rechecks
the same base branch, release branch, source repository, App author, merged
state, and merge-commit requirements. It then checks out that exact merge commit
and proves it is an ancestor of current `main` before crates.io authentication
or publication in the protected `crates-io` job. All dispatch modes are accepted
only when the selected ref is `refs/heads/main`.

release-plz has `publish = false`, `git_tag_enable = false`, and
`git_release_enable = false`. Do not use `release-plz release` in this
repository.

The two workspace packages have independent SemVer versions. If only Feuilletage
changes, the macro version remains unchanged. If the macro changes, release-plz
updates Feuilletage's exact macro dependency, such as `=0.2.0`, and the lockfile;
that dependency change also selects Feuilletage for release.

The macro crate's ordinary release boundary is `feuilletage-macros/Cargo.toml`,
`feuilletage-macros/README.md`, and its packaged files under `feuilletage-macros/src/`.
The root `LICENSE-APACHE` and `LICENSE-MIT` files are the only shared source
files Cargo copies into the macro archive. Treat a license edit as a change to
both crates and verify both are selected in the release PR. A root `README.md`
edit affects only `feuilletage`.

## Publication security and recovery

The publish job checks out the pushed merge SHA with full history and disabled
credential persistence. It fails if `HEAD` differs from that SHA or tracked or
untracked files make the checkout dirty. The protected `crates-io` environment
and job-level `id-token: write` permission allow the SHA-pinned
`rust-lang/crates-io-auth-action` to exchange GitHub OIDC identity for a
short-lived crates.io token. There is no long-lived crates.io repository
secret.

Publication intentionally has no workflow or job concurrency group: GitHub keeps
only one pending run per group, so grouping publication could displace the exact
push event that merged a release PR. Only release-PR preparation is grouped;
new preparation runs cancel older ones because every run refreshes to latest
`origin/main` before checking registry readiness. Push publication never starts
release preparation after it completes. The release-PR App token requests only
repository Contents and Pull requests write permissions.

Publication is retry-safe:

- an unpublished checked-in version is uploaded;
- the initial registry check fixes both package selection and upload intent for
  the run; publication does not perform a second existence check;
- a version already uploaded from the same release commit skips upload and
  still repairs missing tags or GitHub Releases;
- an unchanged version published from an older clean commit is skipped only
  after normalized archive equality is proved, and its existing tag is left
  alone;
- a same-version archive with any package-content difference fails closed;
- an existing annotated tag must already point to the release commit, or the
  workflow fails rather than moving it;
- an existing lightweight tag is rejected rather than replaced;
- an existing GitHub Release is accepted only when its tag, title, draft state,
  prerelease state, and resolved tag commit all match the expected release.

If another publisher wins a race after this run selected an upload, Cargo's
immutable-version collision fails the run. Re-run the same workflow event; the
normal provenance, archive-equivalence, and release-record checks then validate
the published package before reconciliation.

Retry a failed run with GitHub's **Re-run jobs** control first. This preserves
the original push event and release SHA. If the merge-push run cannot be
recovered, manually dispatch the workflow from `main` and provide the merged
release PR number in `release_pr`. Published versions are immutable; a bad
package requires a corrective version rather than a moved tag or overwritten
release.

## Continuous integration

Pull requests and `main` must pass formatting, strict Clippy, workspace builds
and tests, Rust 1.88 checks, the feature matrix, bare-metal `no_std`, rustdoc
and example coverage, all examples, `cargo-machete`, workspace package checks,
release-helper tests, and the RustSec audit.

`./scripts/check-packages.sh` requires stable checked-in versions, requires
Feuilletage's exact macro requirement to match the local macro package, and runs
`cargo package --locked --workspace`. `./scripts/test-publish-release.sh`
exercises normalized archive equality and mismatch handling, bounded registry
readiness and publication dry-run retries, published-package provenance, and
strict GitHub Release metadata handling.

## External setup

### Initial crates.io bootstrap

crates.io Trusted Publishing can only be configured after a crate exists.
Before bootstrap, confirm the operator has crates.io owner or publish access to
the reserved `feuilletage` crate. From one clean, reviewed `main` commit containing
version `0.1.0`, record the exact commit and use a maintainer's short-lived
bootstrap credential:

```bash
git switch --detach origin/main
test -z "$(git status --short)"
BOOTSTRAP_COMMIT=$(git rev-parse HEAD)
./scripts/check-packages.sh
cargo publish --locked -p feuilletage-macros
```

Wait until `cargo info feuilletage-macros@0.1.0 --registry crates-io` succeeds,
then publish Feuilletage:

```bash
cargo publish --locked -p feuilletage
```

Wait until both exact versions resolve. Confirm the checkout is still clean and
at the recorded commit, then create and push both annotated tags:

```bash
test "$(git rev-parse HEAD)" = "$BOOTSTRAP_COMMIT"
test -z "$(git status --short)"
git tag -a macros-v0.1.0 "$BOOTSTRAP_COMMIT" -m "feuilletage-macros v0.1.0"
git tag -a v0.1.0 "$BOOTSTRAP_COMMIT" -m "feuilletage v0.1.0"
git push origin macros-v0.1.0 v0.1.0
```

Create the two initial GitHub Releases from those verified tags if desired.
The automated publisher owns all subsequent package tags and Releases.

### crates.io and GitHub environment

For both crates, configure a crates.io Trusted Publisher with:

- GitHub owner: `omnicli`
- repository: `feuilletage`
- workflow filename: `release.yml`
- environment: `crates-io`

Create and protect the `crates-io` GitHub environment, require the intended
reviewers, and restrict deployment branches to `main`.

### Release GitHub App

Do not rely on a PR created with the workflow's `GITHUB_TOKEN`: GitHub suppresses
new workflow runs for those token-generated events. Create or reuse a GitHub App
installed on `omnicli/feuilletage` with these repository permissions:

- Contents: read and write
- Pull requests: read and write
- Metadata: read

Add its numeric App ID as the repository secret `OMNICLI_APP_ID`, its name as
the repository variable `OMNICLI_APP_NAME`, and its private key as the
repository secret `OMNICLI_PRIVATE_KEY`. The name may be either the App slug
(for example, `omnicli`) or its exact bot login (`omnicli[bot]`), in any letter
case; the workflow normalizes the login and adds the `[bot]` suffix when needed.
The workflow uses the SHA-pinned
`actions/create-github-app-token` action to mint a short-lived installation
token with explicit Contents write permission for tags and GitHub Releases, and
Pull requests write permission when preparing a release PR. Release PR
preparation and publication reconciliation do not fall back to `GITHUB_TOKEN`
or a PAT.

Finally, make the normal CI, `no_std`, and security jobs required checks on
`main`. Do not manually prepare or merge a release PR until the bootstrap and
external setup are complete.

## Normal maintainer flow

1. Merge conventional commits into `main`. Ordinary pushes do not create or
   update a release PR.
2. In the Actions UI, open **Release**, choose **Run workflow**, select `main`,
   and run it to create or update the release PR.
3. Review the generated release PR, including every selected package, version,
   exact dependency update, lockfile, and changelog.
4. Correct the release PR if a version or changelog is wrong.
5. Wait for all required checks, then merge normally. Merging the reviewed
   App-authored release PR is the publication authorization.
6. Confirm the Release workflow for that merged push published the expected
   crates and created the expected annotated tags and GitHub Releases.

Inspect Cargo's package boundaries with:

```bash
cargo package --locked --list -p feuilletage
cargo package --locked --list -p feuilletage-macros
```

To defer a release, do not dispatch the preparation workflow and do not merge an
existing release PR. Ordinary pushes to `main` remain non-releasing.
