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

    /// Output BitBar/xbar compatible text for use in a status bar app
    #[arg(long)]
    status_bar: bool,
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

    if cli.status_bar {
        print_status_bar(&counts, &cli);
        return;
    }

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
            s.parse::<i64>().ok().map(|days| now - Duration::days(days))
        }
    }
}

const NOISE_COMMANDS: &[&str] = &[
    "ls", "cd", "clear", "pwd", "exit", "history", "which", "echo", "cat", "less", "more", "head",
    "tail", "man", "again",
];

fn filter_entries(
    entries: Vec<HistoryEntry>,
    cutoff: Option<DateTime<Local>>,
    cli: &Cli,
) -> Vec<String> {
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
                if let Some(prefix) = normalized_command_prefix(cmd) {
                    if NOISE_COMMANDS.contains(&prefix.as_str()) {
                        return false;
                    }
                } else {
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

        let (prefix, rest) = command_name_and_rest(&cmd);
        groups.entry(prefix).or_default().push((rest, count));
    }

    // Sort groups by total count
    let mut group_totals: Vec<_> = groups
        .iter()
        .map(|(prefix, cmds)| {
            let total: u32 = cmds.iter().map(|(_, c)| *c).sum();
            (prefix.clone(), total)
        })
        .collect();
    group_totals.sort_by(|a, b| b.1.cmp(&a.1));

    for (prefix, total) in group_totals.into_iter().take(cli.limit) {
        println!("\n{} ({}x)", prefix, total);

        if let Some(cmds) = groups.get(&prefix) {
            for (rest, count) in cmds.iter().take(5) {
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

fn print_status_bar(counts: &[(String, u32)], cli: &Cli) {
    for line in build_status_bar_lines(counts, cli) {
        println!("{}", line);
    }
}

fn build_status_bar_lines(counts: &[(String, u32)], cli: &Cli) -> Vec<String> {
    let filtered: Vec<_> = counts
        .iter()
        .filter(|(_, count)| *count >= cli.min)
        .take(cli.limit)
        .map(|(command, count)| (command.as_str(), *count))
        .collect();

    if filtered.is_empty() {
        return vec![
            "Again: no data".to_string(),
            "---".to_string(),
            "No commands found matching criteria.".to_string(),
        ];
    }

    let (top_command, top_count) = filtered[0];
    let total_executions: u32 = counts.iter().map(|(_, count)| *count).sum();
    let mut lines = Vec::new();
    lines.push(format!(
        "Again: {} ({}x)",
        truncate_for_status_bar(top_command, 40),
        top_count
    ));

    lines.push("---".to_string());
    lines.push(status_bar_header("Top commands"));
    for &(command, count) in &filtered {
        lines.push(format!(
            "{} ({}x)",
            truncate_for_status_bar(command, 60),
            count
        ));
    }

    lines.push("---".to_string());
    lines.push(status_bar_header("Usage snapshot"));
    lines.push(format!("Unique commands tracked: {}", counts.len()));
    lines.push(format!("Total executions tallied: {}", total_executions));
    lines.push(format!(
        "Showing top {} of {} possible",
        filtered.len(),
        cli.limit
    ));
    lines.push(status_bar_filters_summary(cli));

    lines.push("---".to_string());
    lines.push(status_bar_header("Actions"));
    lines.push(build_status_bar_refresh_line(cli));
    lines.push(build_status_bar_repo_line());

    lines
}

fn status_bar_header(title: &str) -> String {
    format!("{} | color=#888888", title)
}

fn status_bar_filters_summary(cli: &Cli) -> String {
    let pattern = cli
        .pattern
        .as_deref()
        .map(|p| {
            if p.contains(' ') {
                format!("\"{}\"", p)
            } else {
                p.to_string()
            }
        })
        .unwrap_or_else(|| "any".to_string());
    let noise = if cli.no_noise { "noise off" } else { "noise on" };
    let grouping = if cli.group { "grouped" } else { "flat" };
    format!(
        "Filters: since={} | min={} | pattern={} | noise={} | output={}",
        cli.since, cli.min, pattern, noise, grouping
    )
}

fn build_status_bar_refresh_line(cli: &Cli) -> String {
    let args = status_bar_command_arguments(cli);
    let params = args
        .into_iter()
        .enumerate()
        .map(|(idx, value)| format!("param{}={}", idx + 1, encode_status_bar_param(&value)))
        .collect::<Vec<_>>()
        .join(" ");
    format!(
        "Refresh | bash=again {} terminal=false refresh=true",
        params
    )
}

fn build_status_bar_repo_line() -> String {
    let repo = env!("CARGO_PKG_REPOSITORY");
    format!("Open again repo | href={}", repo)
}

fn status_bar_command_arguments(cli: &Cli) -> Vec<String> {
    let mut args = Vec::new();
    if let Some(pattern) = &cli.pattern {
        args.push(pattern.clone());
    }
    args.push("--min".to_string());
    args.push(cli.min.to_string());
    args.push("--since".to_string());
    args.push(cli.since.clone());
    args.push("-n".to_string());
    args.push(cli.limit.to_string());
    if cli.group {
        args.push("--group".to_string());
    }
    if cli.no_noise {
        args.push("--no-noise".to_string());
    }
    args.push("--status-bar".to_string());
    args
}

fn encode_status_bar_param(value: &str) -> String {
    let escaped = value.replace('\\', "\\\\").replace('"', "\\\"");
    format!("\"{}\"", escaped)
}

fn truncate_for_status_bar(command: &str, limit: usize) -> String {
    if command.chars().count() <= limit {
        return command.to_string();
    }

    if limit <= 3 {
        return "...".to_string();
    }

    let truncated: String = command.chars().take(limit - 3).collect();
    format!("{}...", truncated)
}

fn normalized_command_prefix(command: &str) -> Option<String> {
    command
        .split_whitespace()
        .next()
        .map(|token| normalize_command_token(token))
}

fn normalize_command_token(token: &str) -> String {
    let trimmed = token.trim_matches(|c| c == '"' || c == '\'');
    if trimmed.is_empty() {
        return "other".to_string();
    }

    trimmed
        .rsplit(|c| c == '/' || c == '\\')
        .next()
        .unwrap_or(trimmed)
        .to_string()
}

fn command_name_and_rest(command: &str) -> (String, String) {
    let mut parts = command.split_whitespace();
    if let Some(raw_prefix) = parts.next() {
        let normalized = normalize_command_token(raw_prefix);
        let rest = parts.collect::<Vec<_>>().join(" ");
        (normalized, rest)
    } else {
        ("other".to_string(), String::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_cli() -> Cli {
        Cli {
            pattern: None,
            min: 2,
            since: "week".to_string(),
            limit: 15,
            group: false,
            no_noise: false,
            status_bar: false,
        }
    }

    #[test]
    fn normalize_command_token_removes_paths() {
        assert_eq!(normalize_command_token("/usr/bin/git"), "git");
        assert_eq!(
            normalize_command_token("C:\\Program Files\\Git\\bin\\git.exe"),
            "git.exe"
        );
        assert_eq!(normalize_command_token("./scripts/deploy"), "deploy");
    }

    #[test]
    fn command_name_and_rest_handles_path_prefix() {
        let (prefix, rest) = command_name_and_rest("/usr/local/bin/python manage.py runserver");
        assert_eq!(prefix, "python");
        assert_eq!(rest, "manage.py runserver");

        let (prefix, rest) = command_name_and_rest("git status -sb");
        assert_eq!(prefix, "git");
        assert_eq!(rest, "status -sb");
    }

    #[test]
    fn noise_filter_ignores_commands_with_path_prefix() {
        let entries = vec![
            HistoryEntry {
                command: "/bin/ls -la".to_string(),
                timestamp: None,
            },
            HistoryEntry {
                command: "/usr/bin/cd projects".to_string(),
                timestamp: None,
            },
            HistoryEntry {
                command: "git status".to_string(),
                timestamp: None,
            },
        ];

        let mut cli = base_cli();
        cli.no_noise = true;

        let filtered = filter_entries(entries, None, &cli);
        assert_eq!(filtered, vec!["git status".to_string()]);
    }

    #[test]
    fn status_bar_lines_include_top_entries() {
        let counts = vec![
            ("git status".to_string(), 5),
            ("npm start".to_string(), 3),
            ("ls".to_string(), 1),
        ];
        let cli = base_cli();
        let lines = build_status_bar_lines(&counts, &cli);

        assert_eq!(lines[0], "Again: git status (5x)");
        assert_eq!(lines[1], "---");
        assert_eq!(lines[2], "Top commands | color=#888888");
        assert!(lines.contains(&"git status (5x)".to_string()));
        assert!(lines.contains(&"npm start (3x)".to_string()));
        assert!(lines.iter().any(|l| l.starts_with("Usage snapshot |")));
        assert!(lines.iter().any(|l| l.contains("Filters: since=week")));
        assert!(lines
            .iter()
            .any(|l| l.starts_with("Refresh | bash=again")));
        assert!(!lines.iter().any(|l| l.contains("ls (1x)")));
    }

    #[test]
    fn status_bar_lines_handle_empty_results() {
        let counts = vec![("git status".to_string(), 2)];
        let mut cli = base_cli();
        cli.min = 5;

        let lines = build_status_bar_lines(&counts, &cli);
        assert_eq!(
            lines,
            vec![
                "Again: no data".to_string(),
                "---".to_string(),
                "No commands found matching criteria.".to_string()
            ]
        );
    }
}
