## What changed / why

Describe the change and the motivation.

## How to test

```bash
cargo test
# optionally
cargo run -- "tomorrow at 5pm"
RUSTLING_DEBUG_RULES=1 cargo run -- "from 2:30 - 5:50"
```

## Checklist

- [ ] Tests added/updated (or not needed)
- [ ] `cargo fmt --all` run
- [ ] `cargo clippy --all-targets --all-features` run (or explain why not)
- [ ] Docs/README updated if relevant

## Discussions (optional)

If this is a larger change, link the issue/discussion that aligned on the approach.
