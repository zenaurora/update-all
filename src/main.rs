use anyhow::{Context, Result, bail};
use clap::Parser;
use dirs::config_dir;
use serde::Deserialize;
use std::collections::HashSet;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};

#[derive(Debug, Parser)]
#[command(
    name = "update-all",
    version,
    about = "Run common update commands from one place."
)]
struct Cli {
    #[arg(long, help = "Skip the confirmation prompt and run immediately.")]
    yes: bool,

    #[arg(
        long,
        help = "List detected updaters and planned commands without running them."
    )]
    list: bool,

    #[arg(
        long,
        value_name = "PATH",
        help = "Load custom updaters and disabled built-ins from this TOML file."
    )]
    config: Option<PathBuf>,
}

#[derive(Debug, Clone)]
struct Updater {
    name: String,
    detect: String,
    command: String,
    needs_sudo: bool,
    kind: UpdaterKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum UpdaterKind {
    Builtin,
    Custom,
}

#[derive(Debug, Clone)]
struct DetectedUpdater {
    updater: Updater,
    installed: bool,
}

#[derive(Debug, Default, Deserialize)]
struct ConfigFile {
    #[serde(default)]
    disable: Vec<String>,
    #[serde(default)]
    custom: Vec<CustomUpdater>,
}

#[derive(Debug, Deserialize)]
struct CustomUpdater {
    name: String,
    detect: String,
    command: String,
    #[serde(default)]
    needs_sudo: bool,
}

#[derive(Debug, Default)]
struct Summary {
    ok: Vec<String>,
    failed: Vec<String>,
    skipped: Vec<String>,
}

fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(err) => {
            eprintln!("error: {err:#}");
            ExitCode::from(1)
        }
    }
}

fn run() -> Result<ExitCode> {
    let cli = Cli::parse();
    let config_path = cli.config.clone().unwrap_or_else(default_config_path);
    let config = load_config(&config_path, cli.config.is_some())?;
    let updaters = build_updaters(config)?;
    let detected = detect_updaters(updaters);

    print_detected(&detected, &config_path);
    let planned: Vec<Updater> = detected
        .iter()
        .filter(|item| item.installed)
        .map(|item| item.updater.clone())
        .collect();

    if cli.list {
        if planned.is_empty() {
            println!();
            println!("No installed updaters detected.");
        } else {
            println!();
            print_planned(&planned);
        }
        return Ok(ExitCode::SUCCESS);
    }

    if planned.is_empty() {
        println!();
        println!("No installed updaters detected.");
        return Ok(ExitCode::SUCCESS);
    }

    println!();
    print_planned(&planned);

    if !cli.yes && !prompt_for_confirmation()? {
        println!("Aborted.");
        return Ok(ExitCode::SUCCESS);
    }

    let mut summary = Summary {
        skipped: detected
            .iter()
            .filter(|item| !item.installed)
            .map(|item| item.updater.name.clone())
            .collect(),
        ..Summary::default()
    };

    println!();
    execute_updaters(&planned, &mut summary);
    println!();
    print_summary(&summary);

    if summary.failed.is_empty() {
        Ok(ExitCode::SUCCESS)
    } else {
        Ok(ExitCode::from(1))
    }
}

fn default_config_path() -> PathBuf {
    config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("update-all")
        .join("config.toml")
}

fn load_config(path: &Path, explicit: bool) -> Result<ConfigFile> {
    if !path.exists() {
        if explicit {
            bail!("config file not found: {}", path.display());
        }
        return Ok(ConfigFile::default());
    }

    let raw = fs::read_to_string(path)
        .with_context(|| format!("failed to read config file: {}", path.display()))?;
    toml::from_str(&raw).with_context(|| format!("failed to parse config file: {}", path.display()))
}

fn build_updaters(config: ConfigFile) -> Result<Vec<Updater>> {
    let disabled: HashSet<String> = config
        .disable
        .into_iter()
        .map(|name| normalize(&name))
        .collect();
    let mut updaters: Vec<Updater> = builtin_updaters()
        .into_iter()
        .filter(|updater| !disabled.contains(&normalize(&updater.name)))
        .collect();

    for custom in config.custom {
        if disabled.contains(&normalize(&custom.name)) {
            continue;
        }

        updaters.push(Updater {
            name: custom.name,
            detect: custom.detect,
            command: custom.command,
            needs_sudo: custom.needs_sudo,
            kind: UpdaterKind::Custom,
        });
    }

    let mut seen = HashSet::new();
    for updater in &updaters {
        let key = normalize(&updater.name);
        if !seen.insert(key) {
            bail!("duplicate updater name: {}", updater.name);
        }
    }

    Ok(updaters)
}

fn builtin_updaters() -> Vec<Updater> {
    vec![
        updater(
            "brew",
            "command -v brew >/dev/null 2>&1",
            "brew update && brew upgrade",
            false,
            UpdaterKind::Builtin,
        ),
        updater(
            "oh-my-zsh",
            "[ -d \"$HOME/.oh-my-zsh\" ] && command -v omz >/dev/null 2>&1",
            "omz update",
            false,
            UpdaterKind::Builtin,
        ),
        updater(
            "rustup",
            "command -v rustup >/dev/null 2>&1",
            "rustup update",
            false,
            UpdaterKind::Builtin,
        ),
        updater(
            "claude",
            "command -v claude >/dev/null 2>&1",
            "claude update",
            false,
            UpdaterKind::Builtin,
        ),
        updater(
            "codex",
            "command -v codex >/dev/null 2>&1",
            "codex update",
            false,
            UpdaterKind::Builtin,
        ),
        updater(
            "uipro",
            "command -v uipro >/dev/null 2>&1",
            "uipro update",
            false,
            UpdaterKind::Builtin,
        ),
        updater(
            "kimi",
            "command -v kimi >/dev/null 2>&1",
            "kimi upgrade",
            false,
            UpdaterKind::Builtin,
        ),
        updater(
            "opencode",
            "command -v opencode >/dev/null 2>&1",
            "opencode upgrade",
            false,
            UpdaterKind::Builtin,
        ),
        updater(
            "bob",
            "command -v bob >/dev/null 2>&1",
            "bob update --all",
            false,
            UpdaterKind::Builtin,
        ),
        updater(
            "mas",
            "command -v mas >/dev/null 2>&1",
            "mas upgrade",
            false,
            UpdaterKind::Builtin,
        ),
        updater(
            "bun",
            "command -v bun >/dev/null 2>&1",
            "bun upgrade",
            false,
            UpdaterKind::Builtin,
        ),
        updater(
            "pnpm",
            "command -v pnpm >/dev/null 2>&1",
            "pnpm self-update",
            false,
            UpdaterKind::Builtin,
        ),
        updater(
            "uv",
            "command -v uv >/dev/null 2>&1",
            "uv self update && uv tool upgrade --all",
            false,
            UpdaterKind::Builtin,
        ),
        updater(
            "gh-extensions",
            "command -v gh >/dev/null 2>&1",
            "gh extension upgrade --all",
            false,
            UpdaterKind::Builtin,
        ),
        updater(
            "mise",
            "command -v mise >/dev/null 2>&1",
            "mise self-update && mise plugins update",
            false,
            UpdaterKind::Builtin,
        ),
        updater(
            "asdf",
            "command -v asdf >/dev/null 2>&1",
            "asdf plugin update --all",
            false,
            UpdaterKind::Builtin,
        ),
        updater(
            "tlmgr",
            "command -v tlmgr >/dev/null 2>&1",
            "tlmgr update --self --all",
            false,
            UpdaterKind::Builtin,
        ),
    ]
}

fn updater(
    name: &str,
    detect: &str,
    command: &str,
    needs_sudo: bool,
    kind: UpdaterKind,
) -> Updater {
    Updater {
        name: name.to_owned(),
        detect: detect.to_owned(),
        command: command.to_owned(),
        needs_sudo,
        kind,
    }
}

fn detect_updaters(updaters: Vec<Updater>) -> Vec<DetectedUpdater> {
    updaters
        .into_iter()
        .map(|updater| DetectedUpdater {
            installed: shell_status(&updater.detect).is_ok_and(|status| status.success()),
            updater,
        })
        .collect()
}

fn print_detected(detected: &[DetectedUpdater], config_path: &Path) {
    println!("Detected updaters:");
    for item in detected {
        let status = if item.installed { "yes" } else { "no " };
        let source = match item.updater.kind {
            UpdaterKind::Builtin => "builtin",
            UpdaterKind::Custom => "custom",
        };
        let mut notes = vec![source.to_owned()];
        if !item.installed {
            notes.push("not installed".to_owned());
        }
        if item.updater.needs_sudo {
            notes.push("requires sudo".to_owned());
        }
        println!("  [{status}] {} ({})", item.updater.name, notes.join(", "));
    }

    if config_path.exists() {
        println!("Config: {}", config_path.display());
    }
}

fn print_planned(updaters: &[Updater]) {
    println!("Planned commands:");
    for updater in updaters {
        println!("  {} -> {}", updater.name, display_command(updater));
    }
}

fn prompt_for_confirmation() -> Result<bool> {
    print!("Continue? [y/N] ");
    io::stdout().flush().context("failed to flush stdout")?;

    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .context("failed to read confirmation")?;

    let normalized = input.trim().to_ascii_lowercase();
    Ok(matches!(normalized.as_str(), "y" | "yes"))
}

fn execute_updaters(updaters: &[Updater], summary: &mut Summary) {
    for (index, updater) in updaters.iter().enumerate() {
        println!("[{}/{}] {}", index + 1, updaters.len(), updater.name);
        if updater.needs_sudo {
            println!("requires sudo");
        }
        println!("running: {}", display_command(updater));

        match run_command(updater) {
            Ok(true) => {
                println!("status: ok");
                summary.ok.push(updater.name.clone());
            }
            Ok(false) => {
                println!("status: failed");
                summary.failed.push(updater.name.clone());
            }
            Err(err) => {
                eprintln!("status: failed to start: {err:#}");
                summary.failed.push(updater.name.clone());
            }
        }

        println!();
    }
}

fn run_command(updater: &Updater) -> Result<bool> {
    let status = if updater.needs_sudo {
        Command::new("sudo")
            .arg("sh")
            .arg("-lc")
            .arg(&updater.command)
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()
            .with_context(|| format!("failed to start updater {}", updater.name))?
    } else {
        shell_status_inherit(&updater.command)
            .with_context(|| format!("failed to start updater {}", updater.name))?
    };

    Ok(status.success())
}

fn print_summary(summary: &Summary) {
    println!("Summary:");
    println!("  ok: {}", join_names(&summary.ok));
    println!("  failed: {}", join_names(&summary.failed));
    println!("  skipped: {}", join_names(&summary.skipped));
}

fn join_names(items: &[String]) -> String {
    if items.is_empty() {
        "none".to_owned()
    } else {
        items.join(", ")
    }
}

fn display_command(updater: &Updater) -> String {
    if updater.needs_sudo {
        format!("sudo {}", updater.command)
    } else {
        updater.command.clone()
    }
}

fn normalize(name: &str) -> String {
    name.trim().to_ascii_lowercase()
}

fn shell_status(command: &str) -> io::Result<std::process::ExitStatus> {
    Command::new("sh")
        .arg("-lc")
        .arg(command)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
}

fn shell_status_inherit(command: &str) -> io::Result<std::process::ExitStatus> {
    Command::new("sh")
        .arg("-lc")
        .arg(command)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_command_prefixes_sudo() {
        let updater = updater(
            "codex",
            "command -v codex >/dev/null 2>&1",
            "npm install -g @openai/codex",
            true,
            UpdaterKind::Custom,
        );

        assert_eq!(
            display_command(&updater),
            "sudo npm install -g @openai/codex"
        );
    }

    #[test]
    fn build_updaters_rejects_duplicate_names() {
        let config = ConfigFile {
            disable: Vec::new(),
            custom: vec![CustomUpdater {
                name: "brew".to_owned(),
                detect: "command -v brew >/dev/null 2>&1".to_owned(),
                command: "brew update".to_owned(),
                needs_sudo: false,
            }],
        };

        let result = build_updaters(config);

        assert!(result.is_err());
    }

    #[test]
    fn build_updaters_applies_disabled_names_case_insensitively() {
        let config = ConfigFile {
            disable: vec!["BrEw".to_owned()],
            custom: Vec::new(),
        };

        let updaters = build_updaters(config).expect("config should build");

        assert!(!updaters.iter().any(|item| item.name == "brew"));
    }

    #[test]
    fn builtin_updaters_include_recent_package_managers() {
        let updaters = builtin_updaters();

        assert!(updaters.iter().any(|item| item.name == "pnpm"));
        assert!(updaters.iter().any(|item| item.name == "gh-extensions"));
        assert!(updaters.iter().any(|item| item.name == "tlmgr"));
        assert!(updaters.iter().any(|item| item.name == "uipro"));
    }

    #[test]
    fn uv_updates_self_and_installed_tools() {
        let uv = builtin_updaters()
            .into_iter()
            .find(|item| item.name == "uv")
            .expect("uv updater should exist");

        assert_eq!(uv.command, "uv self update && uv tool upgrade --all");
    }
}
