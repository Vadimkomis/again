# Again Status Bar Wrapper

This Swift package hosts `again` inside a native macOS menu bar app so you can view command-usage stats without relying on xbar.

## Prerequisites

1. Install the Rust CLI (needed because the Swift app shells out to `again`):
   ```bash
   cargo install --path .
   ```
2. Ensure `~/.cargo/bin` is on your `PATH`. The app prepends it automatically, but keeping it in your shell profile helps when you run `again` manually.

## Run

```bash
cd statusbar
swift run again-statusbar
```

The binary sets its activation policy to `.accessory`, so it appears as a menu bar icon labeled with the top command (`Again: …`). Click the icon to see the sections for top commands, usage snapshot, and actions. It refreshes automatically every five minutes; use the “Refresh” menu item to trigger a manual update.

### Customizing arguments

By default the wrapper calls:

```
again --since week --min 2 -n 15 --status-bar
```

If you want a different window (e.g., show only today’s commands), edit `commandArguments()` inside `StatusBarController.swift` to add or remove flags, then rebuild via `swift run`.
