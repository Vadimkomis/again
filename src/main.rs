use chrono::{DateTime, Duration, Local, TimeZone};
use clap::Parser;
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "again")]
#[command(about = "See what commands you run again and again")]
struct Cli {
    /// Filter commands containing this pattern
    pattern: Option<String>,

    /// Only show commands run at least N times
    #[arg(short, long, default_value = "2")]
    min: u32,

    /// Time period: "today", "week", "month", or number of days
    #[arg(short, long, default_value = "week")]
    since: String,

    /// Number of results to show
    #[arg(short = 'n', long, default_value = "15")]
    limit: usize,

    /// Group commands by prefix (git, npm, etc.)
    #[arg(short, long)]
    group: bool,

    /// Exclude common noise commands (ls, cd, clear, etc.)
    #[arg(long)]
    no_noise: bool,
}

struct HistoryEntry {
    command: String,
    timestamp: Option<DateTime<Local>>,
}

fn main() {
    let cli = Cli::parse();

    let history_path = get_history_path();
    let content = match read_file_lossy(&history_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Failed to read history file {:?}: {}", history_path, e);
            std::process::exit(1);
        }
    };

    let entries = parse_history(&content);
    let cutoff = parse_since(&cli.since);
    let filtered = filter_entries(entries, cutoff, &cli);
    let counts = count_commands(filtered);

    if cli.group {
        print_grouped(counts, &cli);
    } else {
        print_flat(counts, &cli);
    }
}

fn read_file_lossy(path: &PathBuf) -> std::io::Result<String> {
    let mut file = File::open(path)?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)?;
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

fn get_history_path() -> PathBuf {
    let home = dirs::home_dir().expect("Could not find home directory");

    // Try zsh first, then bash
    let zsh_history = home.join(".zsh_history");
    if zsh_history.exists() {
        return zsh_history;
    }

    let bash_history = home.join(".bash_history");
    if bash_history.exists() {
        return bash_history;
    }

    // Default to zsh
    zsh_history
}

fn parse_history(content: &str) -> Vec<HistoryEntry> {
    let mut entries = Vec::new();

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        // zsh extended history format: `: timestamp:0;command`
        if line.starts_with(": ") && line.contains(";") {
            if let Some((meta, cmd)) = line[2..].split_once(';') {
                let timestamp = meta
                    .split(':')
                    .next()
                    .and_then(|ts| ts.trim().parse::<i64>().ok())
                    .and_then(|ts| Local.timestamp_opt(ts, 0).single());

                entries.push(HistoryEntry {
                    command: cmd.to_string(),
                    timestamp,
                });
            }
        } else {
            // Plain format (no timestamp)
            entries.push(HistoryEntry {
                command: line.to_string(),
                timestamp: None,
            });
        }
    }

    entries
}

fn parse_since(since: &str) -> Option<DateTime<Local>> {
    let now = Local::now();

    match since.to_lowercase().as_str() {
        "today" => Some(now - Duration::days(1)),
        "week" => Some(now - Duration::days(7)),
        "month" => Some(now - Duration::days(30)),
        "all" => None,
        s => {
            // Try parsing as number of days
            s.parse::<i64>()
                .ok()
                .map(|days| now - Duration::days(days))
        }
    }
}

fn filter_entries(entries: Vec<HistoryEntry>, cutoff: Option<DateTime<Local>>, cli: &Cli) -> Vec<String> {
    let noise = vec![
        "ls", "cd", "clear", "pwd", "exit", "history", "which", "echo",
        "cat", "less", "more", "head", "tail", "man", "again",
    ];

    entries
        .into_iter()
        .filter(|e| {
            // Filter by time if we have both cutoff and timestamp
            if let (Some(cutoff), Some(ts)) = (cutoff, e.timestamp) {
                if ts < cutoff {
                    return false;
                }
            }
            true
        })
        .map(|e| e.command)
        .filter(|cmd| {
            // Filter by pattern
            if let Some(ref pattern) = cli.pattern {
                if !cmd.to_lowercase().contains(&pattern.to_lowercase()) {
                    return false;
                }
            }

            // Filter noise
            if cli.no_noise {
                let first_word = cmd.split_whitespace().next().unwrap_or("");
                if noise.contains(&first_word) {
                    return false;
                }
            }

            true
        })
        .collect()
}

fn count_commands(commands: Vec<String>) -> Vec<(String, u32)> {
    let mut counts: HashMap<String, u32> = HashMap::new();

    for cmd in commands {
        *counts.entry(cmd).or_insert(0) += 1;
    }

    let mut sorted: Vec<_> = counts.into_iter().collect();
    sorted.sort_by(|a, b| b.1.cmp(&a.1));
    sorted
}

fn print_flat(counts: Vec<(String, u32)>, cli: &Cli) {
    let max_count_width = counts
        .first()
        .map(|(_, c)| c.to_string().len())
        .unwrap_or(1);

    let filtered: Vec<_> = counts
        .into_iter()
        .filter(|(_, count)| *count >= cli.min)
        .take(cli.limit)
        .collect();

    if filtered.is_empty() {
        println!("No commands found matching criteria.");
        return;
    }

    for (cmd, count) in filtered {
        let truncated = if cmd.len() > 60 {
            format!("{}...", &cmd[..57])
        } else {
            cmd
        };
        println!("{:>width$}x  {}", count, truncated, width = max_count_width);
    }
}

fn print_grouped(counts: Vec<(String, u32)>, cli: &Cli) {
    let mut groups: HashMap<String, Vec<(String, u32)>> = HashMap::new();

    for (cmd, count) in counts {
        if count < cli.min {
            continue;
        }

        let prefix = cmd
            .split_whitespace()
            .next()
            .unwrap_or("other")
            .to_string();

        groups.entry(prefix).or_default().push((cmd, count));
    }

    // Sort groups by total count
    let mut group_totals: Vec<_> = groups
        .iter()
        .map(|(prefix, cmds)| {
            let total: u32 = cmds.iter().map(|(_, c)| c).sum();
            (prefix.clone(), total)
        })
        .collect();
    group_totals.sort_by(|a, b| b.1.cmp(&a.1));

    for (prefix, total) in group_totals.into_iter().take(cli.limit) {
        println!("\n{} ({}x)", prefix, total);

        if let Some(cmds) = groups.get(&prefix) {
            for (cmd, count) in cmds.iter().take(5) {
                let rest = cmd.strip_prefix(&prefix).unwrap_or(cmd).trim();
                let display = if rest.is_empty() { "(bare)" } else { rest };
                let truncated = if display.len() > 50 {
                    format!("{}...", &display[..47])
                } else {
                    display.to_string()
                };
                println!("    {:>3}x  {}", count, truncated);
            }
        }
    }
}
