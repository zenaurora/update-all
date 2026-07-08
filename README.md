# update-all

Small native CLI for running the updater commands you care about from one place.

## Why this exists

This tool is aimed at concrete CLIs, not generic package-manager maintenance.

Good fits:

- `codex update`
- `claude update`
- `kimi upgrade`
- `uipro update`
- `cargo install mdbook --locked`

Less useful as built-ins:

- generic `npm update`
- generic `cargo update`
- project-local dependency updates

If a package manager installed a global CLI, the thing you usually want to update is that CLI itself.

## What it does

- Detects built-in updaters automatically
- Prints the commands it will run
- Prompts once before execution unless you pass `--yes`
- Continues after failures and prints a final summary
- Supports optional custom updaters from TOML

## Built-in updaters

- `brew`
- `rustup`
- `claude`
- `codex`
- `uipro`
- `kimi`
- `opencode`
- `bob`
- `bun`
- `pnpm`
- `uv`
- `cargo-edit`
- `cargo-nextest`
- `cargo-semver-checks`
- `mdbook`
- `mdbook-toc`
- `tlmgr`

Notes:

- `bob` runs `bob update --all`, which updates Bob-managed Neovim installs.
- That is separate from a Homebrew-installed `nvim`.
- `uipro` is provided by the global npm package `uipro-cli`; it updates the UI/UX Pro Max skill installer.
- `uv` runs both `uv self update` and `uv tool upgrade --all`.
- The Cargo-based entries reinstall globally installed Rust CLIs with `cargo install ... --locked`.

## Install

Development use:

```bash
cargo run -- --list
cargo run
cargo run -- --yes
```

Why the extra `--`:

- `cargo run` starts the project through Cargo
- the first `--` separates Cargo's own arguments from your program's arguments
- `cargo run -- --list` means "run `update-all` and pass `--list` to it"

Normal use:

```bash
cargo build --release
install -m 755 target/release/update-all ~/.local/bin/update-all
```

Then make sure `~/.local/bin` is in your `PATH`, and run:

```bash
update-all --list
update-all
update-all --yes
```

## Command reference

```text
update-all [OPTIONS]

--yes            Skip the confirmation prompt and run immediately
--list           List detected updaters and planned commands without running them
--config <PATH>  Load custom updaters and disabled built-ins from this TOML file
--help           Print help
--version        Print version
```

## Config

No config file is required. If present, the default path is:

```text
~/.config/update-all/config.toml
```

You can also point to a file explicitly:

```bash
update-all --config /path/to/config.toml
```

Supported keys:

- `disable = ["tlmgr"]` disables built-in updaters by name
- `[[custom]]` adds your own updater entries

Config schema:

```toml
disable = ["tlmgr"]

[[custom]]
name = "some-tool"
detect = "command -v some-tool >/dev/null 2>&1"
command = "some-tool update"
needs_sudo = false
```

Field meanings:

- `name` is the display name and unique identifier
- `detect` is a shell command used to decide whether the tool is installed
- `command` is the updater command that will actually run
- `needs_sudo = true` runs the command through `sudo sh -lc`

## Examples

Tool-level examples:

```toml
disable = ["tlmgr"]

[[custom]]
name = "some-npm-cli"
detect = "command -v some-npm-cli >/dev/null 2>&1"
command = "npm install -g some-npm-cli@latest"
needs_sudo = false

[[custom]]
name = "some-cargo-cli"
detect = "command -v some-cargo-cli >/dev/null 2>&1"
command = "cargo install some-cargo-cli --locked"
needs_sudo = false

[[custom]]
name = "some-uv-tool"
detect = "command -v some-uv-tool >/dev/null 2>&1"
command = "uv tool upgrade some-uv-tool"
needs_sudo = false
```

Examples of things you may want to add yourself if they exist on your machine:

- CLIs installed with `cargo install`
- CLIs installed with `npm -g`
- CLIs installed with `uv tool install`
- internal company tools with a custom upgrade command

## Output behavior

`update-all --list` only prints detection results and planned commands.

`update-all`:

- shows detected updaters
- shows planned commands
- asks once for confirmation
- runs each updater in order
- continues even if one updater fails
- prints a final summary of `ok`, `failed`, and `skipped`

## Caveats

- Detection and execution are shell-based, using `sh -lc`.
- Built-ins are intentionally conservative. Not every global CLI should be hardcoded.
- Some tools are better represented as custom config entries because they are machine-specific.
