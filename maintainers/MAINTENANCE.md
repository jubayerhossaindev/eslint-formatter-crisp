# Maintenance

## Toolchains

Root `.prototools` pins Moon, Node.js/npm and Rust. The package's `rust-toolchain.toml` matches the workspace Rust pin. The docs application has an independent `package-lock.json`; the npm package keeps its own lockfile and Cargo lockfile.

## Routine checks

- `moon run eslint-formatter-crisp:verify`
- `moon run docs:verify`
- Compare native binary checksums before and after any package build or relocation.
- Run a packed-tarball consumer test for every change to exports, declarations, or packaging.
- Run target-specific CI on real hosts before claiming native runtime support.

## Dependency updates

Review Dependabot changes individually. Do not use blanket upgrades; keep npm lockfiles and `Cargo.lock` committed with the manifests they resolve.
