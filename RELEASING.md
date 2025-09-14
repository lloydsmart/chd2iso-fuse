# Releasing chd2iso-fuse

This document describes the automated release process for **chd2iso-fuse**.  
The pipeline is powered by GitHub Actions workflows in `.github/workflows`.

---

## Branching model

- We follow **Git Flow**:
  - `develop`: ongoing development
  - `release/x.y.z`: release preparation branches
  - `hotfix/x.y.z`: urgent fixes off `main`
  - `main`: stable, tagged releases only

---

## Release steps

### 1. Prepare release branch
- Create a branch from `develop`:
  ```bash
  git flow release start x.y.z
  ```
- Update version references (Cargo, Debian changelog) via `autobump.yml` workflow.
- Push the branch, open a PR into `main`.

### 2. Merge release branch
- Merge the release PR into `main` (via squash/merge).
- The `merge.yml` workflow will:
  - Verify version consistency (`_verify-release.yml`)
  - Create and push a signed annotated tag (`vX.Y.Z`)
  - Trigger `release.yml` for that tag
  - Open a back-merge PR `main -> develop` (auto-merged if clean)

### 3. Automated release pipeline
- The `release.yml` workflow will:
  - Validate the tag points to `main`
  - Invoke `_build.yml` in release mode to produce:
    - `.deb` packages
    - `SHA256SUMS` + `SHA256SUMS.asc`
    - `RELEASE_NOTES.md` + `CHANGELOG.md`
  - Assemble and upload release assets with `gh release upload`
  - Publish a GitHub Release with the generated notes and assets
  - Invoke `_release.yml` for optional APT publishing

### 4. Back-merge
- The `merge.yml` workflow ensures `develop` is kept in sync with `main`:
  - Auto-merges a back-merge PR if it is conflict-free and CI passes
  - Otherwise opens a PR for manual resolution

---

## Outputs

Each tagged release (`vX.Y.Z`) produces:
- A GitHub Release with:
  - `.deb` package(s)
  - `SHA256SUMS` and detached signature
  - `CHANGELOG.md`
  - `RELEASE_NOTES.md`
- Version bump applied consistently across:
  - Cargo.toml
  - Debian changelog
  - Git tag
- Auto-updated `develop` branch via back-merge PR

---

## Manual release triggers

You can also manually trigger a release workflow:

```bash
gh workflow run release.yml -f tag=vX.Y.Z
```

This runs the same build, verify, and publish steps as an automated tag push.

---

## Troubleshooting

- **No assets on release**  
  Ensure `_build.yml` ran packaging steps. Tag pushes always bypass path filters now.

- **Back-merge PR not auto-merged**  
  Check if conflicts exist or repo auto-merge is disabled. Resolve manually if required.

- **Missing changelog**  
  If `RELEASE_NOTES.md` wasn’t produced by `_build.yml`, `release.yml` regenerates notes via `git-cliff`.
