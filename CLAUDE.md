# again

A CLI tool that analyzes your shell history to surface the commands you run again and again.

## Agent Behavior

- **Implement without asking**: When making suggestions or improvements, implement them directly without asking for permission. Just do it.
- **Always add tests**: Add unit tests for all modified code and all new code. No exceptions.

## Quick Reference

```bash
cargo build --release      # Build optimized binary
cargo test                 # Run all tests
cargo clippy               # Lint (must pass with no warnings)
cargo fmt -- --check       # Check formatting
cargo fmt                  # Apply formatting
```

## Architecture

```
src/
└── main.rs    # Single-file CLI application
statusbar/
├── Package.swift
└── Sources/AgainStatusBar   # Swift AppKit menu bar wrapper
```

The codebase is intentionally minimal. Key components:

- **CLI parsing**: Uses `clap` with derive macros (`Cli` struct at line ~8)
- **History parsing**: Supports both zsh extended format (`: timestamp:0;cmd`) and plain format
- **Filtering**: Time-based, pattern matching, and noise exclusion
- **Output**: Flat list or grouped by command prefix

## Code Conventions

### Rust Style
- Follow standard Rust idioms and the [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- Use `clippy` warnings as errors in CI
- Format with `rustfmt` before committing
- Prefer `if let` and `match` over `.unwrap()` where errors are possible
- Use `expect()` only when the panic message adds context

### Error Handling
- User-facing errors: print to stderr with `eprintln!` and exit with non-zero code
- Internal invariants: use `expect()` with descriptive messages
- Avoid `.unwrap()` on user input or file operations

### CLI Design
- Keep flags intuitive and Unix-like
- Provide sensible defaults (see `Cli` struct)
- Support both short and long flag variants

## Dependencies

| Crate | Purpose |
|-------|---------|
| `clap` | Command-line argument parsing with derive macros |
| `chrono` | Timestamp parsing and date arithmetic |
| `dirs` | Cross-platform home directory resolution |

Keep dependencies minimal. Justify any new additions.

## Testing

**Required workflow:**
1. After modifying, adding, or deleting any logic, write or update corresponding tests. This is mandatory—no code changes without corresponding test changes.
2. All new functions must have unit tests covering typical cases and edge cases
3. All modified functions must have their tests updated to reflect the changes
4. After committing changes, run `cargo test` locally to verify everything passes

When adding features, include tests:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_since_variants() {
        // Test "today", "week", "month", "all", and numeric days
    }
}
```

## Pull Request Guidelines

1. Run the full check suite: `cargo fmt -- --check && cargo clippy && cargo test`
2. Keep commits atomic and well-described
3. Update this file if adding new components or changing architecture
4. Maintain backward compatibility for CLI flags when possible

## Common Tasks

### Adding a new CLI flag
1. Add field to `Cli` struct with appropriate `#[arg(...)]` attributes
2. Use the field in filtering/output logic
3. Update `--help` text via doc comments

### Supporting a new shell history format
1. Extend `parse_history()` with format detection
2. Add to `get_history_path()` if the file location differs
3. Document the format in code comments

### Adding output formats (e.g., JSON)
1. Add `--format` flag to `Cli`
2. Create separate print function (e.g., `print_json()`)
3. Dispatch in `main()` based on format selection

### Shipping the Swift status bar wrapper
1. Build via `cd statusbar && swift build`
2. Run with `swift run again-statusbar`
3. Build `.app` bundle: `cd statusbar && ./build-app.sh`
4. Install to Applications: `cp -r statusbar/AgainStatusBar.app /Applications/`
5. The wrapper shells out to the installed `again` binary (`cargo install --path .`)

## Gitignore

The repository uses a combined `.gitignore` for both Rust and Swift:

```
# Rust
/target          # Cargo build output
Cargo.lock       # Lock file (for binaries, not libraries)

# Swift
.build/          # Swift Package Manager build output
.swiftpm/        # SPM workspace data
*.xcodeproj/     # Xcode project files
DerivedData/     # Xcode build cache
*.app/           # Built application bundles

# macOS
.DS_Store        # Finder metadata
*.dSYM/          # Debug symbols
```

**When adding new build artifacts or tooling, update `.gitignore` accordingly.**
