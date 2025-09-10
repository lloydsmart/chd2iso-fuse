# Releasing chd2iso-fuse

This project uses a git-flow style process with **tag-driven releases**
and CI automation.

## Branches

-   `develop` --- active development\
-   `main` --- stable, released code\
-   `release/x.y.z` --- temporary branch to prepare a release\
-   `hotfix/x.y.z` --- urgent fix branch cut from `main`

------------------------------------------------------------------------

## TL;DR (10-step quick ref)

1.  `git flow release start x.y.z`\
2.  `git push -u origin release/x.y.z`\
3.  Stabilize on `release/x.y.z` (commit fixes).\
4.  CI **autobump** updates version & changelogs on each push (bot
    commits with `[skip ci]`).\
5.  Open PR: `release/x.y.z` → `main`.\
6.  Ensure PR CI (fmt, clippy, tests) is green; review the
    bump/changelog changes.\
7.  Merge PR.\
8.  **On-merge workflow** auto-creates tag `vX.Y.Z` on the merge
    commit.\
9.  **Release workflow** (on tag) builds, verifies versions, publishes
    GitHub Release.\
10. A **back-merge PR** `main → develop` is opened automatically; review
    and merge it.

Hotfixes follow the same pattern with `hotfix/x.y.z`.

------------------------------------------------------------------------

## What CI does for you

### 1) CI (Build & Test) --- `ci.yml`

-   Runs on PRs and pushes.
-   Checks: `cargo fmt --check`, `clippy -D warnings`,
    `cargo test --locked --all-features`.
-   Uses cargo registry/git caches for speed.
-   (Push only, on `main`/`release/*`) builds `.deb` packages and
    uploads as artifacts.
-   Ignores bot commits containing `[skip ci]`.

### 2) Release Prep (Autobump) --- `autobump.yml`

-   Runs on **pushes to `release/*`** (optionally `hotfix/*`).
-   Derives version from branch name and:
    -   Updates `Cargo.toml`
    -   Regenerates `debian/changelog`
    -   Regenerates `CHANGELOG.md` & `RELEASE_NOTES.md` (via
        `git-cliff`)
-   Commits with `chore(release): prepare vX.Y.Z [skip ci]` (prevents
    loops).

### 3) On merge to main --- `on-merge.yml`

-   Runs on **pushes to `main`**.
-   Detects if the push came from a **merged PR** whose source was
    `release/*` or `hotfix/*`.
-   **Verifies versions** using the reusable guard
    (`_verify-release.yml`).
-   **Creates annotated tag** `vX.Y.Z` (no code changes to `main`).
-   **Opens a PR** `main → develop` if `develop` is behind or diverged.

### 4) Release (on tag) --- `release.yml`

-   Runs when **tag `v*`** is pushed.
-   **Guard:** fails early if the tag's commit isn't contained in
    `main`.
-   **Builds & packages**, **verifies versions** (Cargo.toml ↔ Debian ↔
    binary), then **publishes a GitHub Release** with artifacts and
    notes.
-   (Optional) Can call APT publishing via `_release.yml` when you
    enable it.

### 5) Nightly --- `nightly.yml`

-   Builds `develop` nightly (and on manual dispatch).
-   Produces artifacts + a simple manifest.

------------------------------------------------------------------------

## Standard Release Flow (details)

### 1) Start a release branch

``` bash
git flow release start x.y.z
git push -u origin release/x.y.z
```

### 2) Stabilize on the release branch

-   Commit fixes normally.
-   On each push, **Autobump**:
    -   Ensures `Cargo.toml` version matches `release/x.y.z`
    -   Updates `debian/changelog`
    -   Regenerates `CHANGELOG.md` and `RELEASE_NOTES.md`
    -   Commits with `[skip ci]` so other workflows don't rerun

> Tip: If you need to skip CI for a one-off commit, include `[skip ci]`
> in your own message too.

### 3) Open a PR to `main`

-   Title: `Release x.y.z`.
-   CI must pass (fmt, clippy, tests). Review the version/changelog
    diffs as part of the PR.

### 4) Merge the PR

-   Use your usual "Merge" policy.
-   After GitHub completes the merge:
    -   **On-merge** creates the **annotated tag** `vX.Y.Z` on the merge
        commit.
    -   It also opens a **back-merge PR** `main → develop` if needed.

### 5) Release build & publish

-   Tag push automatically triggers **Release**:
    -   Guard ensures the tag is on `main`.
    -   Build, package, verify versions.
    -   Draft + publish the GitHub Release with `.deb`, checksums, and
        notes.

### 6) Back-merge to develop

-   A PR `main → develop` is opened for you.
-   Resolve conflicts if any; merge to keep `develop` in sync.

### 7) Cleanup

-   The `release/x.y.z` branch is auto-deleted on merge (remote).

-   Locally:

    ``` bash
    git fetch --prune
    ```

------------------------------------------------------------------------

## Hotfix Flow

1.  `git flow hotfix start x.y.z` (from `main`), push `hotfix/x.y.z`.\
2.  Stabilize; Autobump keeps version/changelog updated.\
3.  PR `hotfix/x.y.z` → `main`, merge.\
4.  On-merge auto-tags `vX.Y.Z`; Release workflow publishes.\
5.  Back-merge `main → develop` PR is opened; merge it.

------------------------------------------------------------------------

## Conventions & Safeguards

-   **Tag-driven releases:** Only `v*` tags publish. A guard ensures the
    tag points to a commit contained in `main`.
-   **No post-merge mutations:** Workflows never push new commits to
    `main`; they only create tags and PRs.
-   **Version guard:** `_verify-release.yml` ensures:
    -   `Cargo.toml` has **no** `-dev` suffix
    -   Debian upstream version matches (RCs mapped to `~rcN`)
    -   `binary --version` == Cargo version
-   **Skip loops:** Bot commits include `[skip ci]`. CI jobs have guards
    for that phrase.
-   **Required checks:** Only **Build & Test (Linux)** must be green to
    merge PRs (you may also require CodeQL/security).

------------------------------------------------------------------------

## Manual overrides (if needed)

-   **Tag locally** (if you prefer or automation is disabled):

    ``` bash
    git checkout main
    git pull
    git tag -s vX.Y.Z -m "chd2iso-fuse vX.Y.Z"
    git push origin vX.Y.Z
    ```

    The Release workflow still enforces "tag must be on main".

-   **Re-generate notes** on a release branch:

    ``` bash
    ./scripts/gen-release-notes.sh
    git commit -am "docs(changelog): refresh [skip ci]"
    git push
    ```

------------------------------------------------------------------------

## Troubleshooting

-   **Release didn't run on tag:** Ensure the tag commit is reachable
    from `main` (guard may have stopped it).
-   **Auto-tag didn't fire:** Confirm the merge was from `release/*` or
    `hotfix/*`. Re-run the `on-merge` workflow for that commit, or tag
    manually.
-   **Back-merge PR missing:** It won't open if `develop` isn't
    behind/diverged. Check `on-merge` logs; open a PR manually if
    needed.
-   **Version mismatch errors:** Run the release prep again on the
    release branch (push to retrigger Autobump), or fix with the
    provided scripts and commit.
