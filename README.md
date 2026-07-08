# update-all

Small native CLI for running the update commands you care about from one place.

## What it does

- Detects built-in updaters automatically
- Prints the commands it will run
- Prompts once before execution unless you pass `--yes`
- Continues after failures and prints a final summary
- Supports optional custom updaters from TOML

Built-in updaters:

- `brew`
- `oh-my-zsh`
- `rustup`
- `claude`
- `codex`
- `uipro`
- `kimi`
- `opencode`
- `bob`
- `mas`
- `bun`
- `uv`
- `mise`
- `asdf`

Notes:

- `bob` runs `bob update --all`, which updates Bob-managed Neovim installs.
- That is separate from a Homebrew-installed `nvim`.
- This tool is aimed at concrete CLIs, not just package managers themselves.
  For example, `codex update` is a better built-in than a generic `npm update`.

## Usage

```bash
cargo run -- --list
cargo run
cargo run -- --yes
```

Build a release binary:

```bash
cargo build --release
```

Then place `target/release/update-all` somewhere in your `PATH`, such as `~/.local/bin/update-all`.

## Optional config

No config file is required. If present, the default path is:

```text
~/.config/update-all/config.toml
```

You can also point to a file explicitly:

```bash
update-all --config /path/to/config.toml
```

Supported keys:

- `disable = ["mas"]` to disable built-in updaters by name
- `[[custom]]` to add your own updater entries

Example:

```toml
disable = ["mas"]

[[custom]]
name = "cargo-nextest"
detect = "command -v cargo-nextest >/dev/null 2>&1"
command = "cargo install cargo-nextest --locked"
needs_sudo = false
```

More tool-level examples:

```toml
[[custom]]
name = "uipro"
detect = "command -v uipro >/dev/null 2>&1"
command = "uipro update"
needs_sudo = false

[[custom]]
name = "codex-npm"
detect = "command -v codex >/dev/null 2>&1"
command = "npm install -g @openai/codex@latest"
needs_sudo = false

[[custom]]
name = "mdbook"
detect = "command -v mdbook >/dev/null 2>&1"
command = "cargo install mdbook --locked"
needs_sudo = false

[[custom]]
name = "kimi-uv"
detect = "command -v kimi >/dev/null 2>&1"
command = "uv tool upgrade kimi-cli"
needs_sudo = false
```
# update-all
