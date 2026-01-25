# Contributing

Thanks for your interest in Astorion!

## Development setup

```bash
cargo test
cargo run -- "tomorrow at 5pm"
RUSTLING_DEBUG_RULES=1 cargo run -- "from 2:30 - 5:50"
```

## Formatting and linting

```bash
cargo fmt --all
cargo clippy --all-targets --all-features
```

## Where to make changes

- Engine/runtime: `src/engine.rs`
- Rules:
  - Time: `src/rules/time/*`
  - Numerals: `src/rules/numeral/*`
- Public types (currently minimal): `src/lib.rs`

## Bugfix vs feature

- **Bugfix:** add/adjust a focused test that reproduces the issue, then fix the smallest surface area needed.
- **Feature:** describe the intended Duckling semantics and add tests/examples that lock in behavior.

## Tests

Add tests alongside the relevant dimension rules:

- Time rules: `src/rules/time/tests.rs`
- Numeral rules: `src/rules/numeral/tests.rs`

## Pull requests

- Keep diffs small and focused (prefer incremental PRs over one large change).
- Prefer behavior-parity and tests over refactors.
- Include a short note describing intended semantics and example inputs.

## Proposing bigger changes

If the change touches core engine architecture, rule activation, or output semantics, start with an issue/discussion first so we can align on direction before you invest significant time.
