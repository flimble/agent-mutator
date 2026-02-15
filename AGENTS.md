# agent-mutator

Mutation testing CLI for AI coding agents. Supports Python, JavaScript, TypeScript, and Rust.

## Architecture

Rust binary. Key modules:

- `parser.rs` / `parser_js.rs` / `parser_rust.rs` -- tree-sitter based mutation discovery per language
- `operators.rs` -- mutation operator definitions (arithmetic, comparison, logical, boolean, return, string, block removal)
- `runner.rs` -- test execution, baseline timing, isolated tree-copy mutation runs
- `copy_tree.rs` -- project tree copying with smart filtering (.git, node_modules, __pycache__, etc.)
- `state.rs` -- JSON state persistence for `status` and `show` commands
- `safety.rs` -- backup/restore for legacy in-place mode
- `output.rs` -- human-readable terminal output with colors
- `main.rs` -- CLI entry point (clap)

Default mode copies the project to a temp dir and mutates there. Original source is never touched.

## Commands

```
mutator run <file> -t <test_file>                    # full run
mutator run <file> -t <test_file> -f <function>      # scope to function
mutator run <file> -t <test_file> --json             # JSON output
mutator run <file> -t <test_file> -q                 # exit code only
mutator run <file> -t <test_file> --test-cmd "cargo test"  # custom test command
mutator run <file> -t <test_file> --session my-agent       # named session for isolation
mutator run <file> -t <test_file> --in-place                # legacy: mutate in-place
mutator show @m1                                     # show survived mutant details
mutator status                                       # summary of last run
```

## Testing

```
cargo test              # all 252 tests (includes e2e, needs pytest installed)
cargo test --lib        # unit tests only
cargo test --test test_e2e  # e2e tests only (spawns real mutator binary + pytest)
```

## Releasing

Tag and push:

```
git tag v0.2.0
git push origin v0.2.0
```

This automatically:

1. Runs tests
2. Bumps the version in `Cargo.toml` on main
3. Creates a GitHub Release with auto-generated notes
4. Updates the Homebrew formula in `flimble/homebrew-tap`

Do NOT manually edit the version in `Cargo.toml`.

## Development Rules

- All subprocess spawns must set `OBJC_DISABLE_INITIALIZE_FORK_SAFETY=YES` (macOS fork safety)
- Isolated mode is the default; in-place mode exists for backward compatibility
- Test commands are resolved: absolute paths pass through, relative paths with `/` are resolved from CWD, bare commands use PATH
- Parser modules must skip docstrings, logging calls, and string concatenation
- Every mutation needs: line, column, start_byte, end_byte, operator, original, replacement, context
