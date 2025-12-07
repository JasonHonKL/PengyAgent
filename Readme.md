# Pengy Agent CLI

A terminal-based chat interface for interacting with AI agents and LLMs.

## Building

Build the CLI binary:

```bash
cargo build --release
```

The binary will be available at `target/release/pengy`.

## Installation

To install the CLI system-wide so you can run `pengy` from anywhere:

### Option 1: Install to user local bin (recommended)

```bash
cargo build --release
cp target/release/pengy ~/.local/bin/
```

Make sure `~/.local/bin` is in your PATH. Add this to your `~/.zshrc` or `~/.bashrc` if needed:

```bash
export PATH="$HOME/.local/bin:$PATH"
```

### Option 2: Install to system bin (requires sudo)

```bash
cargo build --release
sudo cp target/release/pengy /usr/local/bin/
```

### Option 3: Install using Cargo (if ~/.cargo/bin is in PATH)

```bash
cargo install --path . --bin pengy
```

After installation, verify it works:

```bash
pengy
```

## Running

Run the CLI from anywhere after installation:

```bash
pengy
```

Or run directly from the project directory without installation:

```bash
cargo run --bin pengy
```

## Configuration

On first run, configure your API key and select a model:

1. Press `/settings` to set your API key (OpenRouter API key)
2. Press `/models` to select a model
3. Press `/agents` to choose an agent type (Direct Chat, Coder, Code Researcher, Test Agent, or Pengy Agent)

Configuration is saved to `.pengy_config.json` in the current directory. You can also set the `API_KEY` environment variable.

## Usage

- Type messages to chat with the selected agent
- Press `/models` to switch models
- Press `/agents` to switch agent types
- Press `/settings` to update API key
- Press `/help` for available commands
- Press `Esc` to exit

## Requirements

- Rust (latest stable version)
- API key from OpenRouter or compatible API provider
