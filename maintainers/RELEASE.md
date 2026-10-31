# Release policy

- Use Conventional Commits with one required scope: `package`, `docs`, or `repo`.
- Use `feat(package): ...` for package features, `fix(package): ...` for fixes, and `perf(package): ...` for performance changes.
- Use `docs(docs): ...` for documentation-only work and `ci(repo): ...` for repository automation.
- Only release-bearing `package` commits trigger a package version; documentation and repository maintenance commits are explicitly non-releasing.
- Releases run through Semantic Release in `package-release.yml` after its validation job.
- npm Trusted Publishing/OIDC is preferred over long-lived publish tokens. The npm account's publisher configuration is an external maintainer action.
- Never force-push tags, overwrite an existing version, or bypass required checks.
