# Release & Versioning Process

This project follows **Semantic Versioning (SemVer)**: `MAJOR.MINOR.PATCH`.

- **MAJOR**: breaking API changes.
- **MINOR**: new functionality in a backwards-compatible manner.
- **PATCH**: backwards-compatible bug fixes.

While the project is `0.x`, breaking changes may occur at any time, but we still
signal intent: breaking changes increment the **MINOR** version and document it
clearly in the changelog.

## Changelog policy

We use **Keep a Changelog** in `CHANGELOG.md`.

- Add every user-facing change under **[Unreleased]** as soon as the change lands.
- Group entries under **Added**, **Changed**, **Deprecated**, **Removed**, **Fixed**, and **Security**.
- Keep entries short, user-focused, and linked to PRs/issues when possible.

## Tagging policy

- Release tags use the format: `vMAJOR.MINOR.PATCH` (e.g., `v0.2.1`).
- Tags must point at the release commit on the default branch.
- Annotated tags are preferred:
  ```bash
  git tag -a v0.2.1 -m "astorion v0.2.1"
  ```

## Release checklist

1. **Prepare release PR**
   - Decide the next version number.
   - Update `Cargo.toml` version.
   - Update `CHANGELOG.md`:
     - Move entries from **[Unreleased]** into a new version section.
     - Add the release date (`YYYY-MM-DD`).
     - Ensure links at the bottom compare against the new tag.
   - Ensure `README.md` or docs mention notable changes if needed.

2. **Validate**
   - Run formatting/lint/test as available:
     ```bash
     cargo fmt --all
     cargo clippy --all-targets --all-features
     cargo test
     ```

3. **Release**
   - Merge the release PR to the default branch.
   - Create an annotated tag for the release:
     ```bash
     git tag -a vX.Y.Z -m "astorion vX.Y.Z"
     git push origin vX.Y.Z
     ```
   - Create a GitHub release using the changelog notes for that version.

4. **Post-release**
   - Add a new **[Unreleased]** section to `CHANGELOG.md`.
   - If publishing to crates.io, follow the publish checklist:
     ```bash
     cargo publish
     ```

## Hotfixes

- For urgent fixes, branch from the release tag (`vX.Y.Z`) and increment **PATCH**.
- Keep the hotfix changes minimal and update `CHANGELOG.md`.
- Tag the hotfix release and forward-merge the fixes back to the default branch.
