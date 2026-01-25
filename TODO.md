# Astorion TODOs for open-source readiness

## Open-source readiness

- [ ] Configure CI (tests, formatting, linting) and replace the `CI not_configured` badge with a real workflow badge. 【F:README.md†L5-L63】
- [ ] Publish documentation (e.g., docs.rs) and update the docs badge. 【F:README.md†L5-L63】
- [x] Declare an MSRV by setting `rust-version` in `Cargo.toml` and updating the MSRV badge. 【F:README.md†L5-L53】
- [ ] Stabilize and document a minimal public API surface (README notes instability). 【F:README.md†L49-L63】

## CLI + library readiness

- [x] Document the library API usage (examples for `parse`, `parse_with`, etc.) so the README doesn’t treat the CLI as the only “real” interface. 【F:README.md†L116-L128】【F:src/api.rs†L99-L148】
- [ ] Specify CLI behavior/flags and consider making the CLI a first-class binary (documented usage, options, exit codes). 【F:src/main.rs†L1-L21】

## Publishing + version control

- [x] Add full crate metadata in `Cargo.toml` (e.g., `description`, `license`, `repository`, `readme`, `keywords`, `categories`, `rust-version`). 【F:Cargo.toml†L1-L9】
- [x] Define a release/versioning process (e.g., `CHANGELOG.md`, tagging policy) consistent with “breaking changes allowed while 0.x.” 【F:README.md†L49-L63】
- [ ] Publish the crate to crates.io once API/docs/metadata are ready and update README accordingly. 【F:README.md†L116-L121】
