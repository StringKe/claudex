<p align="center">
  <h1 align="center">Claudex</h1>
  <p align="center">Multi-instance Claude Code manager with intelligent translation proxy</p>
</p>

<p align="center">
  <a href="https://github.com/StringKe/claudex/actions/workflows/ci.yml"><img src="https://github.com/StringKe/claudex/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
  <a href="https://github.com/StringKe/claudex/releases"><img src="https://github.com/StringKe/claudex/actions/workflows/release.yml/badge.svg" alt="Release"></a>
  <a href="https://github.com/StringKe/claudex/blob/main/LICENSE"><img src="https://img.shields.io/github/license/StringKe/claudex" alt="License"></a>
  <a href="https://github.com/StringKe/claudex/releases"><img src="https://img.shields.io/github/v/release/StringKe/claudex" alt="Latest Release"></a>
</p>

<p align="center">
  <a href="https://stringke.github.io/claudex/">Documentation</a> |
  <a href="./README.zh-CN.md">中文</a>
</p>

---

Claudex is a unified proxy that lets [Claude Code](https://docs.anthropic.com/en/docs/claude-code) seamlessly work with multiple AI providers — Grok, ChatGPT, DeepSeek, MiniMax, Kimi, GLM, Ollama, and more — through automatic Anthropic-to-OpenAI protocol translation.

## Features

- **Multi-provider proxy** — DirectAnthropic passthrough (Anthropic, MiniMax, OpenRouter) + automatic Anthropic <-> OpenAI translation (Grok, OpenAI, DeepSeek, Kimi, GLM)
- **Streaming translation** — Full SSE stream translation with tool call support
- **Circuit breaker + failover** — Automatic fallback to backup providers with configurable thresholds
- **Smart routing** — Intent-based auto-routing via local classifier, maps code/analysis/creative/search/math to optimal profiles
- **Context engine** — Conversation compression, cross-profile sharing, local RAG with embeddings
- **TUI dashboard** — Real-time profile health, metrics, logs, and quick-launch
- **Config discovery** — Automatic config file search from current directory up to global config
- **Self-update** — `claudex update` downloads the latest release from GitHub
- **Local model support** — Ollama, vLLM, LM Studio, or any OpenAI-compatible service
- **OAuth subscription support** — Use AI subscriptions (ChatGPT Plus via Codex CLI, Claude Max, etc.) via `claudex auth login`
- **Model slot mapping** — Map Claude Code's `/model` switcher (haiku/sonnet/opus) to any provider's models
- **Non-interactive mode** — `claudex run <profile> "prompt" --print` for one-shot execution
- **Tool name compatibility** — Auto-truncates tool names exceeding OpenAI's 64-char limit with transparent roundtrip restoration

## Installation

```bash
# One-liner (Linux / macOS)
curl -fsSL https://raw.githubusercontent.com/StringKe/claudex/main/install.sh | bash

# From source
cargo install --git https://github.com/StringKe/claudex

# Or download from GitHub Releases
# https://github.com/StringKe/claudex/releases
```

### System Requirements

- macOS (Intel / Apple Silicon) or Linux (x86_64 / ARM64)
- [Claude Code](https://docs.anthropic.com/en/docs/claude-code) installed
- Windows: download pre-built binary from [Releases](https://github.com/StringKe/claudex/releases)

## Quick Start

```bash
# 1. Initialize config
claudex config --init

# 2. Add a provider profile interactively
claudex profile add

# 3. Test connectivity
claudex profile test all

# 4. Run Claude Code with a specific provider
claudex run grok

# 5. Or use smart routing to auto-select the best provider
claudex run auto
```

## How It Works

```
claudex run openrouter-claude
    │
    ├── Start proxy (if not running) → 127.0.0.1:13456
    │
    └── exec claude with env vars:
        ANTHROPIC_BASE_URL=http://127.0.0.1:13456/proxy/openrouter-claude
        ANTHROPIC_AUTH_TOKEN=claudex-passthrough   (gateway mode, no auth conflict)
        ANTHROPIC_MODEL=anthropic/claude-sonnet-4
        ANTHROPIC_DEFAULT_HAIKU_MODEL=...          (from profile models config)
        ANTHROPIC_DEFAULT_SONNET_MODEL=...
        ANTHROPIC_DEFAULT_OPUS_MODEL=...
```

The proxy intercepts requests and handles protocol translation transparently:

- **DirectAnthropic** providers (Anthropic, MiniMax, OpenRouter) → forward with correct headers
- **OpenAICompatible** providers (Grok, OpenAI, DeepSeek, etc.) → translate Anthropic → OpenAI → forward → translate response back

## Provider Compatibility

| Provider | Type | Translation | Example Model |
|----------|------|-------------|---------------|
| Anthropic | `DirectAnthropic` | None | `claude-sonnet-4-20250514` |
| MiniMax | `DirectAnthropic` | None | `claude-sonnet-4-20250514` |
| OpenRouter | `DirectAnthropic` | None | `anthropic/claude-sonnet-4` |
| Grok (xAI) | `OpenAICompatible` | Anthropic <-> OpenAI | `grok-3-beta` |
| OpenAI | `OpenAICompatible` | Anthropic <-> OpenAI | `gpt-4o` |
| DeepSeek | `OpenAICompatible` | Anthropic <-> OpenAI | `deepseek-chat` |
| Kimi | `OpenAICompatible` | Anthropic <-> OpenAI | `moonshot-v1-128k` |
| GLM (Zhipu) | `OpenAICompatible` | Anthropic <-> OpenAI | `glm-4-plus` |
| Ollama | `OpenAICompatible` | Anthropic <-> OpenAI | `qwen2.5:72b` |

## Configuration

Claudex searches for config files in this order:

1. `$CLAUDEX_CONFIG` environment variable
2. `./claudex.toml` (current directory)
3. `./.claudex/config.toml`
4. Parent directories (up to 10 levels)
5. `~/.config/claudex/config.toml` (XDG, recommended)
6. Platform config dir (macOS `~/Library/Application Support/claudex/config.toml`)

```bash
# Show loaded config and search paths
claudex config

# Create a local config
claudex config --init
```

See [`config.example.toml`](./config.example.toml) for the full configuration reference, or visit the [documentation site](https://stringke.github.io/claudex/) for detailed guides.

## CLI Reference

| Command | Description |
|---------|-------------|
| `claudex run <profile>` | Run Claude Code with a specific provider |
| `claudex run auto` | Smart routing — auto-select best provider |
| `claudex run <profile> -m <model>` | Override model for a session |
| `claudex profile list` | List all configured profiles |
| `claudex profile add` | Interactive profile setup wizard |
| `claudex profile show <name>` | Show profile details |
| `claudex profile remove <name>` | Remove a profile |
| `claudex profile test <name\|all>` | Test provider connectivity |
| `claudex proxy start [-p port] [-d]` | Start proxy server (optionally as daemon) |
| `claudex proxy stop` | Stop proxy daemon |
| `claudex proxy status` | Show proxy status |
| `claudex dashboard` | Launch TUI dashboard |
| `claudex config [--init]` | Show or initialize config |
| `claudex update [--check]` | Self-update from GitHub Releases |
| `claudex auth login <provider>` | OAuth login (claude/openai/google/qwen/kimi/github) |
| `claudex auth status` | Show OAuth token status |
| `claudex auth logout <profile>` | Remove OAuth token |
| `claudex auth refresh <profile>` | Force refresh OAuth token |

## Non-interactive Mode

Run Claude Code in one-shot mode (no TUI, outputs result and exits):

```bash
claudex run openrouter-claude "Explain this codebase" --print --dangerously-skip-permissions
```

## OAuth Subscriptions

Use existing CLI subscriptions (e.g., ChatGPT Plus via Codex CLI) instead of API keys:

```bash
# Read token from Codex CLI (~/.codex/auth.json)
claudex auth login openai --profile codex-sub

# Check token status
claudex auth status

# Run with subscription
claudex run codex-sub
```

Supported providers: `claude` (reads `~/.claude`), `openai` (reads `~/.codex`), `google`, `kimi`

## Model Slot Mapping

Map Claude Code's `/model` switcher to any provider's models via `[profiles.models]`:

```toml
[[profiles]]
name = "openrouter-deepseek"
provider_type = "OpenAICompatible"
base_url = "https://openrouter.ai/api/v1"
api_key = "sk-or-..."
default_model = "deepseek/deepseek-chat-v3-0324"

[profiles.models]
haiku = "deepseek/deepseek-chat-v3-0324"
sonnet = "deepseek/deepseek-chat-v3-0324"
opus = "deepseek/deepseek-r1"
```

## Architecture

```
src/
├── main.rs              # Entry + CLI dispatch
├── cli.rs               # clap command definitions
├── config.rs            # Config discovery + parsing
├── profile.rs           # Profile CRUD + connectivity test
├── launch.rs            # Claude process launcher
├── daemon.rs            # PID file + process management
├── metrics.rs           # Request metrics (atomic counters)
├── update.rs            # Self-update via GitHub Releases
├── oauth/               # OAuth subscription auth
│   ├── mod.rs           # AuthType, OAuthProvider, OAuthToken types
│   ├── token.rs         # Keyring CRUD + external CLI token readers
│   ├── server.rs        # Local callback server + Device Code polling
│   └── providers.rs     # Per-platform login/refresh/status logic
├── proxy/               # Translation proxy
│   ├── handler.rs       # Request routing + circuit breaker + failover
│   ├── translation.rs   # Anthropic <-> OpenAI protocol translation
│   ├── streaming.rs     # SSE stream translation (state machine)
│   ├── fallback.rs      # Circuit breaker implementation
│   ├── health.rs        # Background health checker
│   └── models.rs        # /v1/models aggregation
├── router/              # Smart routing
│   └── classifier.rs    # Intent classification via local LLM
├── context/             # Context engine
│   ├── compression.rs   # Conversation compression
│   ├── sharing.rs       # Cross-profile context sharing
│   └── rag.rs           # Local RAG with embeddings
└── tui/                 # TUI dashboard
    ├── dashboard.rs     # Dashboard rendering
    ├── input.rs         # Keyboard input
    └── widgets.rs       # UI components
```

## License

[MIT](./LICENSE)
