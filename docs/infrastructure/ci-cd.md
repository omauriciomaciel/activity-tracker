---
type: Infrastructure
title: CI/CD Pipeline
description: GitHub Actions release pipeline using python-semantic-release for versioning/changelog, a reusable matrix build workflow producing .deb/.tar.gz/.dmg artifacts, and a checksums job.
tags: [ci, cd, github-actions, semantic-release, build, release, matrix]
status: stable
---

# CI/CD Pipeline

**Workflows**: `.github/workflows/release.yml`, `.github/workflows/build.yml`
**Release config**: `pyproject.toml` (`[tool.semantic_release]`)

## Trigger

`release.yml` runs on every push to `main`. It is the single source of truth for versioning,
changelog, and artifact publication.

## Release Job (`release`)

1. Checkout with `fetch-depth: 0` and `GITHUB_TOKEN`.
2. Setup Python 3.12, `pip install python-semantic-release==10.5.3`.
3. Run `semantic-release version` then `semantic-release publish`.
4. Detect a `v*` tag pointing at HEAD; export `released=true|false` and `tag`.

### Semantic Release Config (`pyproject.toml`)

- Version source: `Cargo.toml:package.version` (`version_toml`).
- `major_on_zero = true`, tag format `v{version}`.
- Commit message: `chore(release): {version}`.
- `allowed_tags`: feat, fix, perf, refactor, chore, docs, style, test, ci, build, revert.
- `minor_tags`: feat. `patch_tags`: fix, perf, refactor.
- Changelog written to `CHANGELOG.md`, excluding `chore(release)`, version-number, and merge
  commits.
- Remote: GitHub. Publishes to VCS release.

## Build Job (`build`)

A reusable workflow (`workflow_call`) invoked by `release.yml` only when `released == 'true'`.
Matrix builds for four targets:

| Target | Runner | Cross |
|---|---|---|
| `x86_64-unknown-linux-gnu` | ubuntu-22.04 | no |
| `aarch64-unknown-linux-gnu` | ubuntu-22.04 | yes (`cross`) |
| `x86_64-apple-darwin` | macos-15-intel | no |
| `aarch64-apple-darwin` | macos-latest | no |

Steps per target: checkout at the tag, `dtolnay/rust-toolchain` (stable, with target),
`Swatinem/rust-cache`, build (`cargo` or `cross`), then package.

### Linux Packaging

- **.deb**: builds a Debian package (`activity-tracker_{ver}_{arch}`) with the binary in
  `/usr/bin`, a control file, and `/etc/profile.d/activity-tracker.sh` containing the `at`/`ats`
  aliases. Built with `fakeroot dpkg-deb`.
- **.tar.gz**: `activity-tracker-{ver}-linux-{arch}.tar.gz` containing the binary,
  `README.md`, and `install.sh`.

### macOS Packaging

- **.dmg**: via `create-dmg` (with `hdiutil` fallback), staging the binary + `install.sh`.
- **.tar.gz**: `activity-tracker-{ver}-macos-{arch}.tar.gz` containing the binary, `README.md`,
  and `install.sh`.

### Upload

All artifacts (`.deb`, `.tar.gz`, `.dmg`, `.zip`) are attached to the GitHub Release via
`softprops/action-gh-release`.

## Checksums Job (`checksums`)

Runs after `release` + `build`. Downloads all release assets via
`robinraju/release-downloader`, generates `SHA256SUMS.txt` (`sha256sum` over all archives),
and uploads it back to the release.

## Related

- Artifacts consumed by [Installer](installer.md) and [Self-Update Pipeline](../pipeline/self-update.md)
- Versions the `Cargo.toml` consumed by [Updater Service](../services/updater.md) (`CARGO_PKG_VERSION`)
- CHANGELOG.md is auto-generated (do not edit manually)
