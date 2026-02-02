# Regex Profiling Guide

Astorion can help you spot slow or overly chatty regexes while parsing. Just run the CLI with the`--regex-profile` flag:

```bash
cargo run -- --regex-profile --input "on friday at 5"
```

When profiling is enabled, the CLI report includes an extra section that breaks down regex activity:

```
━━━ Regex Profiling ━━━
  Total regex time: 13.706386ms  │  Matches: 488
  from <time-of-day> - <time-of-day> on <weekday> 339.141µs  evals: 18  matches: 60
  <weekday> 265.942µs  evals: 4  matches: 4
  <weekday> <time-of-day> 238.549µs  evals: 4  matches: 0
  this|last|next qtr 235.802µs  evals: 4  matches: 0
  <time>'s <weekday> 216.708µs  evals: 44  matches: 22
```

## How to read the summary

- **Total regex time** — The total wall-clock time spent evaluating regexes during this run. Compare this with the overall saturation time to see whether regexes are a major contributor to parsing cost.
- **Matches** — The total number of capture hits across all regexes. A high match count with low total time usually means cheap literal matches. Fewer matches with high time, on the other hand, often point to expensive or overly broad rules.
- **Per-rule rows** contain:
  - `rule name`
  - `total_time` — how much time was spent running regexes for this rule
  - `evals` — how many times those regexes were evaluated
  - `matches` — how many capture hits they produced

## Using the data to optimize

1. **Target the heaviest rules first.** Rules at the top of the list cost the most time overall. Simplifying their regexes or reducing how often they run usually gives the biggest payoff.
2. **Watch for noisy rules that never match.** A high evals count with matches: 0 means the rule scans the input frequently but never contributes anything. Tighten its triggers or replace broad patterns with more specific ones.
3. **Check match density.** If `matches` is large but `total_time` is also large, consider breaking the regex into smaller pieces, pre-filtering the input, or caching intermediate results.
4. **Compare against saturation time.** When `Total regex time` is close to the overall saturation time, regex evaluation is your bottleneck. Predicate-first rules, pre-tokenization, or better triggers can help avoid full input scans.
5. **Re-measure after changes.** Re-run `cargo run -- --regex-profile ...` after refactors to confirm the targeted rules dropped in the ranking and the total regex time decreased.

The profiler is intentionally opt-in so normal runs stay fast. Use it on representative inputs before and after changes to make sure your optimizations actually improve real-world workloads.
