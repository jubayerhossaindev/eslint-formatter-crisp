# Publishing the npm package

Releases are managed by Semantic Release from `.github/workflows/package-release.yml`; do not publish from a developer workstation.

## Before enabling releases

1. Configure the npm Trusted Publisher for owner `jubayerhossaindev`, repository `eslint-formatter-crisp`, and workflow filename `package-release.yml`.
2. Ensure the repository is public if npm provenance is required. Trusted Publishing can authenticate without a long-lived npm token; provenance has repository visibility requirements.
3. Enable the required GitHub Actions permissions and protect the default branch and release tags.
4. Confirm the release workflow's required validation job passes.

## Release behavior

Only package commits using the `package` scope and release-bearing Conventional Commit types can create an npm release. Docs-only and repository-only changes do not increment the package version. Semantic Release updates the package manifest, lockfile, package changelog and GitHub release when run on the configured branch.

The initial version remains the existing package version until Semantic Release can determine a release from real Git history. Do not manually set a 1.0.0 version or overwrite existing tags.

No release, npm publish, or GitHub Release was performed as part of this workspace task.
