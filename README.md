# astorion

Duckling-style parsing engine in Rust.

[![CI](https://img.shields.io/badge/CI-not_configured-lightgrey)](#)
[![MSRV](https://img.shields.io/badge/MSRV-1.85.0-blue)](#)
[![Docs](https://img.shields.io/badge/docs-not_published-lightgrey)](#)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Crate](https://img.shields.io/badge/crates.io-TBD-lightgrey)](#)

## What is it?

Astorion is a Rust port of Duckling’s rule-based entity parsing pipeline.

## Who is it for / why does it exist?

- Teams that want Duckling-like time/numeral parsing but prefer a Rust codebase.
- Contributors who want an engine + rule architecture that’s easy to extend with new dimensions/locales.
- Anyone experimenting with saturation-style parsing (discover nodes → combine them → resolve).

## Quick start

```bash
cargo run -- "from 2:30 - 5:50"
RUSTLING_DEBUG_RULES=1 cargo run -- "tomorrow at 5pm"
cargo run -- --reference 2013-02-12T04:30:00 "tomorrow at 5pm"
```

## Example usage

Run the built-in CLI (prints a saturation summary + resolved tokens):

```bash
cargo run -- "tomorrow at 5pm"
```

Or build a release binary:

```bash
cargo build --release
./target/release/astorion "from 2:30 - 5:50"
```

## Status & guarantees

- **Status:** alpha.
- **Stability:** a minimal public API is stabilized; see "Public API" below.
- **MSRV:** 1.85.0 (see `rust-version` in `Cargo.toml`).
- **Breaking changes:** allowed at any time while `0.x`.

## Roadmap

- Define a small stable public API (e.g. `parse(...) -> Vec<Entity>`).
- Improve parity with Duckling semantics (resolution, span/ranking behavior).
- Add locale scaffolding and additional dimensions.
- Add CI, docs, and (eventually) publish a crate.

## Public API

Astorion now exposes a deliberately small, stable API surface intended for early adopters:

- `parse(text) -> ParseResult`
- `parse_with(text, &Context, &Options) -> ParseResult`
- `Context`, `Options`, `Entity`, and `ParseResult`

These items are re-exported at the crate root (`crate::time_expr::parse`, `crate::time_expr::ParseResult`, etc.).
All other modules, types, and debug/verbose entry points are considered internal and may change
without notice while the crate is in `0.x`.

## Contributing

See `CONTRIBUTING.md`.

## Release process

See `docs/release-process.md` and `CHANGELOG.md` for versioning and release guidance.

## License

MIT. See `LICENSE`.

---

## Features

- Duckling-style rule engine: regex/predicate patterns, production closures, saturation to a fixed point.
- Span-based results with rule provenance (`rule_name`) and an evidence chain.
- Built-in CLI debug report (saturation passes, tokens, timings).

## Installation

Astorion is not published to crates.io yet.

To use it locally as a dependency, add a path dependency:

```toml
[dependencies]
astorion = { path = "../astorion" }
```

## CLI usage

The CLI is the primary interface and ships with usage, flags, and exit codes:

```bash
cargo run -- --help
```

### Options

| Option                    | Description                                                                                        |
| ------------------------- | -------------------------------------------------------------------------------------------------- |
| `-i, --input <text>`      | Input text to parse. If omitted, Astorion reads remaining args or stdin when no args are provided. |
| `--reference <timestamp>` | Reference time in `YYYY-MM-DDTHH:MM:SS` (default: `2013-02-12T04:30:00`).                          |
| `--color`                 | Force ANSI color output.                                                                           |
| `--no-color`              | Disable ANSI color output.                                                                         |
| `-h, --help`              | Show help text.                                                                                    |
| `-V, --version`           | Print version information.                                                                         |

### Exit codes

| Code | Meaning                             |
| ---- | ----------------------------------- |
| `0`  | Success.                            |
| `1`  | Internal error.                     |
| `2`  | Invalid arguments or missing input. |

Set `RUSTLING_DEBUG_RULES=1` to print rule filtering/production diagnostics.

## How it works

At a high level, the engine repeatedly applies rules to grow a stash of `Node`s, then resolves and filters the results.

```mermaid
flowchart TD
  subgraph Inputs
    A(["Raw input string"])
    B(["Rule set<br/>Pattern + production"])
  end

  subgraph ParserLifecycle
    C(["Parser::new / new_compiled"])
    C1(["TriggerInfo::scan<br/>buckets + phrases"])
    C2(["Select active rules<br/>bucket + phrase gating"])
    C3(["Split rules<br/>regex_rules / predicate_rules"])

    D(["run_rule_set(regex_rules)<br/>seed pass"])
    E(["Deduplicate<br/>node_key + seen"])
    F(["stash = stash.union(new)"])

    subgraph SaturationLoop
      direction TB
      G(["Filter rules by deps<br/>dimensions_in_stash"])
      H(["run_rule_set(predicate + regex)"])
      I(["Deduplicate + union"])
      J{New nodes?}
    end

    K(["resolve_filtered<br/>resolve_node then drop subsumed spans"])
    L(["ResolvedToken<br/>value + span + rule"])
  end

  A --> C
  B --> C
  C --> C1 --> C2 --> C3 --> D --> E --> F --> G --> H --> I --> J
  J -- yes --> G
  J -- no --> K --> L

  %% Styling
  classDef input fill:transparent,stroke:#06B6D4,stroke-width:1.5px;
  classDef setup fill:transparent,stroke:#6366F1,stroke-width:1.5px;
  classDef loop fill:transparent,stroke:#10B981,stroke-width:1.5px;
  classDef resolve fill:transparent,stroke:#F97316,stroke-width:1.5px;
  classDef decision fill:transparent,stroke:#94A3B8,stroke-width:1.5px;

  class A,B input;
  class C,C1,C2,C3,D,E,F setup;
  class G,H,I loop;
  class K,L resolve;
  class J decision;
```

Key implementation touchpoints:

- `Parser::new_compiled` performs trigger scanning and rule activation.
- `Parser::saturate` runs an initial regex pass, then loops predicate-first until a fixed point.
- `Parser::node_key` + `seen` prevent unbounded growth from duplicate nodes.
- `Parser::resolve_filtered` resolves nodes, sorts by dimension/span, and drops spans contained by a wider match.

## Contributing

See `CONTRIBUTING.md`.

## License

MIT. See `LICENSE`.
