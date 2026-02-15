# agent-mutator

Mutation testing CLI for AI coding agents. Validates that tests actually catch bugs.

## Quick Start

```
mutator run <source_file> -t <test_file> --json
```

## Workflow

1. Write or modify code
2. Write tests
3. Run mutator to check test quality:
   ```
   mutator run src/app.py -t tests/test_app.py -f my_function --json
   ```
4. If mutants survive, improve tests to catch them
5. Re-run until score is acceptable

## Commands

| Command | Description |
|---|---|
| `mutator run <file> -t <test> --json` | Run mutation testing, JSON output |
| `mutator run <file> -t <test> -f <fn>` | Scope to a single function |
| `mutator run <file> -t <test> -q` | Exit code only (0 = all killed, 1 = survivors) |
| `mutator show @m1` | Show details for survived mutant m1 |
| `mutator status --json` | Summary of last run |

## Flags

- `--test-cmd <cmd>` -- Override test runner (default: `pytest`). Use `"cargo test"` for Rust, `"npx vitest run"` for JS/TS.
- `--session <id>` -- Named session for isolation. Pass your agent/session ID to avoid conflicts.
- `--timeout-mult <n>` -- Timeout multiplier (default: 3x baseline).
- `--in-place` -- Mutate source directly instead of copying to temp dir. Unsafe for concurrent use.

## Supported Languages

- **Python** (.py) -- default test cmd: `pytest`
- **JavaScript** (.js, .mjs, .cjs) -- use `--test-cmd "npx vitest run"`
- **TypeScript** (.ts, .mts, .cts) -- use `--test-cmd "npx vitest run"`
- **TSX/JSX** (.tsx, .jsx) -- use `--test-cmd "npx vitest run"`
- **Rust** (.rs) -- use `--test-cmd "cargo test"`

## JSON Output Format

```json
{
  "score": 0.85,
  "total": 20,
  "killed": 17,
  "survived": 3,
  "timeout": 0,
  "unviable": 0,
  "duration_ms": 5000,
  "survived_mutants": [
    {
      "ref_id": "m1",
      "file": "src/app.py",
      "line": 10,
      "column": 5,
      "operator": "boundary",
      "original": ">",
      "replacement": ">=",
      "diff": "- x > 0\n+ x >= 0\n",
      "context_before": ["def check(x):"],
      "context_after": ["    return True"]
    }
  ]
}
```

## Tips

- Always use `-f <function>` to scope mutations. Full-file runs are slow.
- Use `--json` for machine-readable output. Parse `score` and `survived_mutants`.
- A score of 1.0 means all mutants were killed. Below 0.8 suggests weak tests.
- Use `mutator show @m1` to inspect specific survivors and understand what to test.
- The `--session` flag prevents temp dir conflicts when multiple agents run concurrently.
