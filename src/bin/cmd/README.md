# Pengy Command-Line Interface (Non-Interactive)

This is a non-interactive command-line interface for running Pengy agents without the TUI.

## Usage

```bash
pengy-cmd --apikey=<key> --model=<model> --prompt="<prompt>" [options]
```

## Required Arguments

- `--apikey=<key>` or `--api-key=<key>`: API key for the model provider
- `--model=<model>`: Model name (e.g., `openai/gpt-4o`)
- `--prompt="<prompt>"`: The prompt/question for the agent

## Optional Arguments

- `--agent=<type>`: Agent type (default: `coder`)
  - Available: `coder`, `code-researcher`, `test-agent`, `pengy-agent`, `control-agent`, `issue-agent`, `chat-agent`
- `--base-url=<url>`: Custom base URL (default: `https://openrouter.ai/api/v1`)
- `--yolo`: Auto-approve all actions (always yes mode)

## Examples

```bash
# Basic usage with coder agent
pengy-cmd --apikey=sk-... --model=openai/gpt-4o --prompt="Write a hello world function"

# Use code researcher agent
pengy-cmd --apikey=sk-... --model=openai/gpt-4o --prompt="Research the codebase" --agent=code-researcher

# Use with yolo mode (auto-approve)
pengy-cmd --apikey=sk-... --model=openai/gpt-4o --prompt="Make changes" --yolo

# Use custom base URL
pengy-cmd --apikey=sk-... --model=custom/model --prompt="Do something" --base-url=https://api.example.com/v1
```

## Building

```bash
cargo build --release --bin pengy-cmd
```

The binary will be available at `target/release/pengy-cmd`.

