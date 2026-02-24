# Claudex

[![CI](https://github.com/StringKe/claudex/actions/workflows/ci.yml/badge.svg)](https://github.com/StringKe/claudex/actions/workflows/ci.yml)
[![Release](https://github.com/StringKe/claudex/actions/workflows/release.yml/badge.svg)](https://github.com/StringKe/claudex/releases)

Multi-instance Claude Code manager with intelligent translation proxy.

通过统一代理管理多个 AI 模型提供商，让 Claude Code 无缝使用 Grok、ChatGPT、DeepSeek、MiniMax、Kimi、GLM 等模型。

## Installation

```bash
# One-liner (Linux / macOS)
curl -fsSL https://raw.githubusercontent.com/StringKe/claudex/main/install.sh | bash

# From GitHub Releases
# Download the binary for your platform from:
# https://github.com/StringKe/claudex/releases

# From source
cargo install --git https://github.com/StringKe/claudex
```

## Features

- **Multi-provider proxy** — Anthropic-native providers (Anthropic, MiniMax, OpenRouter) direct passthrough; OpenAI-compatible providers (Grok, OpenAI, DeepSeek, Kimi, GLM) automatic Anthropic <-> OpenAI translation
- **Streaming translation** — Full SSE stream translation with tool call support
- **Circuit breaker + failover** — Automatic fallback to backup providers on failure with configurable thresholds
- **Smart routing** — Intent-based auto-routing via local classifier (Ollama), maps code/analysis/creative/search/math to optimal profiles
- **Context engine** — Conversation compression, cross-profile context sharing, local RAG with embeddings
- **TUI dashboard** — Real-time profile health, metrics, logs, and quick-launch
- **Interactive profile add** — Guided CLI wizard with connectivity test and keyring integration
- **Config discovery** — Automatic config file search from current directory up to global config
- **Self-update** — `claudex update` downloads latest release from GitHub
- **Local model support** — Any OpenAI-compatible local service (Ollama, vLLM, LM Studio)
- **Keyring integration** — Secure API key storage via OS keychain

## Quick Start

```bash
# Initialize config in current directory (or auto-discover existing config)
claudex config --init

# Or let claudex create a global config on first run
claudex profile list

# Add a profile interactively
claudex profile add

# Test connectivity
claudex profile test all

# Run Claude Code with a specific provider
claudex run grok
claudex run deepseek
claudex run chatgpt

# Override model for a session
claudex run grok -m grok-3-mini-beta

# Smart routing (auto-selects best provider)
claudex run auto

# Launch TUI dashboard
claudex dashboard

# Self-update
claudex update
claudex update --check
```

## How It Works

```
claudex run grok
    |
    +-- Start proxy (if not running) -> 127.0.0.1:13456
    |
    +-- exec claude with env vars:
        CLAUDE_CONFIG_DIR=~/.config/claude-grok
        ANTHROPIC_BASE_URL=http://127.0.0.1:13456/proxy/grok
        ANTHROPIC_API_KEY=claudex-passthrough
        ANTHROPIC_MODEL=grok-3-beta
```

Proxy receives request -> extracts profile from URL path -> routes:
- **DirectAnthropic**: Forward with correct headers + base URL
- **OpenAICompatible**: Translate Anthropic -> OpenAI -> forward -> translate response back

### Request Flow with All Features Enabled

```
Request -> Smart Router (if profile=auto) -> Profile resolved
        -> Context Engine:
            1. RAG: search local code index -> inject relevant snippets
            2. Sharing: gather context from other profiles -> inject
            3. Compression: summarize old messages if over threshold
        -> Circuit Breaker check
        -> Forward to provider
        -> On failure: try backup providers (with their own circuit breakers)
        -> Record metrics + store response context
```

## Configuration

### Config Discovery

Claudex searches for config files in this order:

1. `$CLAUDEX_CONFIG` environment variable
2. `./claudex.toml` (current directory)
3. `./.claudex/config.toml` (current directory)
4. Parent directories (up to 10 levels), checking `claudex.toml` and `.claudex/config.toml`
5. `~/.config/claudex/config.toml` (global)

If no config is found, a default global config is created from the built-in template.

```bash
# Show which config is loaded and search paths
claudex config

# Create a local config in the current directory
claudex config --init
```

### Provider Types

| Provider | Type | Translation |
|----------|------|-------------|
| Anthropic | `DirectAnthropic` | None |
| MiniMax | `DirectAnthropic` | None |
| OpenRouter | `DirectAnthropic` | None |
| Grok (xAI) | `OpenAICompatible` | Anthropic <-> OpenAI |
| OpenAI | `OpenAICompatible` | Anthropic <-> OpenAI |
| DeepSeek | `OpenAICompatible` | Anthropic <-> OpenAI |
| Kimi | `OpenAICompatible` | Anthropic <-> OpenAI |
| GLM | `OpenAICompatible` | Anthropic <-> OpenAI |
| Ollama/vLLM | `OpenAICompatible` | Anthropic <-> OpenAI |

### Keyring

Store API keys securely in your OS keychain:

```toml
[[profiles]]
name = "grok"
api_key_keyring = "grok-api-key"  # reads from OS keychain
```

Or use the interactive wizard which offers keyring storage:

```bash
claudex profile add
```

### Smart Router

Route requests to different providers based on intent classification:

```toml
[router]
enabled = true
classifier_url = "http://localhost:11434/v1"
classifier_model = "qwen2.5:3b"

[router.rules]
code = "deepseek"
analysis = "grok"
creative = "chatgpt"
search = "kimi"
math = "deepseek"
default = "grok"
```

Use with `claudex run auto`.

### Context Engine

```toml
[context.compression]
enabled = true
threshold_tokens = 50000
keep_recent = 10
summarizer_url = "http://localhost:11434/v1"
summarizer_model = "qwen2.5:3b"

[context.sharing]
enabled = true
max_context_size = 2000

[context.rag]
enabled = true
index_paths = ["./src", "./docs"]
embedding_url = "http://localhost:11434/v1"
embedding_model = "nomic-embed-text"
```

### Self-Update

```bash
# Check for updates
claudex update --check

# Download and install latest version
claudex update
```

See [`config.example.toml`](./config.example.toml) for full configuration reference.

## Architecture

```
src/
+-- main.rs              # Entry point + CLI dispatch
+-- cli.rs               # clap command definitions
+-- config.rs            # Config discovery + parsing + keyring
+-- profile.rs           # Profile CRUD + interactive add + connectivity test
+-- launch.rs            # Claude process launcher
+-- daemon.rs            # PID file + process management
+-- metrics.rs           # Request metrics (atomic counters)
+-- update.rs            # Self-update via GitHub Releases
+-- proxy/
|   +-- mod.rs           # Axum server + state
|   +-- handler.rs       # Request routing + circuit breaker + failover
|   +-- middleware.rs     # Context engine (RAG, sharing, compression)
|   +-- translation.rs   # Anthropic <-> OpenAI protocol translation
|   +-- streaming.rs     # SSE stream translation (state machine)
|   +-- fallback.rs      # Circuit breaker implementation
|   +-- health.rs        # Background health checker
|   +-- models.rs        # /v1/models endpoint
+-- router/
|   +-- mod.rs           # Router config
|   +-- classifier.rs    # Intent classification via local LLM
+-- context/
|   +-- mod.rs           # Context engine config
|   +-- compression.rs   # Conversation compression
|   +-- sharing.rs       # Cross-profile context sharing
|   +-- rag.rs           # Local RAG with embeddings
+-- tui/
    +-- mod.rs           # TUI app + event loop
    +-- dashboard.rs     # Dashboard rendering
    +-- input.rs         # Keyboard input handling
    +-- widgets.rs       # Help popup
```

## License

MIT
