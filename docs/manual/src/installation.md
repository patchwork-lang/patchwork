# Installation

Patchwork runs inside your IDE's AI assistant. This guide covers setup for **Zed with Claude Code**.

## Prerequisites

You'll need:
- [Rust](https://rustup.rs/) (for installing Patchwork tools)
- [Node.js](https://nodejs.org/) (for Claude Code)

## Install the Tools

Install the Patchwork runtime and conductor:

```bash
cargo install sacp-conductor patchwork-acp
```

This adds two executables to your Cargo bin directory (typically `~/.cargo/bin/`).

## Configure Zed

Open your Zed settings file at `~/.config/zed/settings.json` and add the `agent_servers` configuration:

```json
{
  "agent_servers": {
    "Patchwork": {
      "default_mode": "bypassPermissions",
      "command": "/Users/yourname/.cargo/bin/sacp-conductor",
      "args": [
        "--debug",
        "agent",
        "/Users/yourname/.cargo/bin/patchwork-acp",
        "npx -y '@zed-industries/claude-code-acp'"
      ],
      "env": {}
    }
  }
}
```

> **Note:** Replace `/Users/yourname` with your actual home directory. You can find the correct path with `which sacp-conductor`.

## Start a Session

1. Reload Zed (or restart it)
2. Open the Agent Panel
3. Click the **+** button
4. Choose **Patchwork** from the menu

You're ready to start agentic scripting with Patchwork!
