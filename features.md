# Features

This document lists the capabilities that `again` already provides and the levers you can pull to extend or customize them.

## Run From Any Folder

You can invoke `again` from any directory in your terminal session by installing the binary into a directory that is on your `PATH`.

Implementation steps:

1. Build and install the binary into your Cargo bin directory: `cargo install --path .`.
2. Ensure `~/.cargo/bin` (or the bin directory you targeted) is on your shell `PATH`.
3. Optionally create a shell alias or wrapper function if you want to pin default flags.

Because the CLI always resolves the shell history file relative to your home directory (see `get_history_path()` in `src/main.rs`), the output is identical regardless of which working directory you are currently in.

## History Analysis

- Reads from zsh extended history and the plain format used by bash.
- Parses timestamps so you can slice history by relative time ranges (`--since` flag).

## Filtering Tools

- `pattern` positional argument filters commands by substring match (case-insensitive).
- `--min` enforces a minimum number of occurrences.
- `--since` accepts friendly ranges (`today`, `week`, `month`, `all`, or numeric days).
- `--no-noise` strips very common commands like `ls`, `cd`, `clear`, etc.

## Output Modes

- Flat list sorted by frequency (`print_flat`).
- Grouped output (`--group`) that aggregates commands by their first token.
