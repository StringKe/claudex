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
  <a href="https://stringke.github.io/claudex/">Documentation</a>
</p>

<p align="center">
  English |
  <a href="./README.zh-CN.md">简体中文</a> |
  <a href="./README.zh-TW.md">繁體中文</a> |
  <a href="./README.ja.md">日本語</a> |
  <a href="./README.ko.md">한국어</a> |
  <a href="./README.ru.md">Русский</a> |
  <a href="./README.fr.md">Français</a> |
  <a href="./README.pt-BR.md">Português do Brasil</a> |
  <a href="./README.es.md">Español</a> |
  <a href="./README.it.md">Italiano</a> |
  <a href="./README.de.md">Deutsch</a> |
  <a href="./README.pl.md">Polski</a>
</p>

---

Claudex is a unified proxy that lets [Claude Code](https://docs.anthropic.com/en/docs/claude-code) seamlessly work with multiple AI providers through automatic protocol translation.

## Features

- **Multi-provider proxy** — DirectAnthropic passthrough + Anthropic <-> OpenAI Chat Completions translation + Anthropic <-> Responses API translation
- **20+ providers** — Anthropic, OpenRouter, Grok, OpenAI, DeepSeek, Kimi, GLM, Groq, Mistral, Together AI, Perplexity, Cerebras, Azure OpenAI, Google Vertex AI, Ollama, LM Studio, and more
- **Streaming translation** — Full SSE stream translation with tool call support
- **Circuit breaker + failover** — Automatic fallback to backup providers with configurable thresholds
- **Smart routing** — Intent-based auto-routing via local classifier
- **Context engine** — Conversation compression, cross-profile sharing, local RAG with embeddings
- **OAuth subscriptions** — ChatGPT/Codex, Claude Max, GitHub Copilot, GitLab Duo, Google Gemini, Qwen, Kimi
- **Configuration sets** — Install and manage reusable Claude Code configuration sets from git repos
- **TUI dashboard** — Real-time profile health, metrics, logs, and quick-launch
- **Self-update** — `claudex update` downloads the latest release from GitHub

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
claudex config init

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
        ANTHROPIC_AUTH_TOKEN=claudex-passthrough
        ANTHROPIC_MODEL=anthropic/claude-sonnet-4
        ANTHROPIC_DEFAULT_HAIKU_MODEL=...
        ANTHROPIC_DEFAULT_SONNET_MODEL=...
        ANTHROPIC_DEFAULT_OPUS_MODEL=...
```

The proxy intercepts requests and handles protocol translation:

- **DirectAnthropic** (Anthropic, MiniMax, Vertex AI) → forward with correct headers
- **OpenAICompatible** (Grok, OpenAI, DeepSeek, etc.) → Anthropic → OpenAI Chat Completions → translate response back
- **OpenAIResponses** (ChatGPT/Codex subscriptions) → Anthropic → Responses API → translate response back

## Provider Compatibility

| Provider | Type | Translation | Auth | Example Model |
|----------|------|-------------|------|---------------|
| Anthropic | DirectAnthropic | None | API Key | `claude-sonnet-4-20250514` |
| MiniMax | DirectAnthropic | None | API Key | `claude-sonnet-4-20250514` |
| OpenRouter | OpenAICompatible | Anthropic <-> OpenAI | API Key | `anthropic/claude-sonnet-4` |
| Grok (xAI) | OpenAICompatible | Anthropic <-> OpenAI | API Key | `grok-3-beta` |
| OpenAI | OpenAICompatible | Anthropic <-> OpenAI | API Key | `gpt-4o` |
| DeepSeek | OpenAICompatible | Anthropic <-> OpenAI | API Key | `deepseek-chat` |
| Kimi | OpenAICompatible | Anthropic <-> OpenAI | API Key | `kimi-k2-0905-preview` |
| GLM (Zhipu) | OpenAICompatible | Anthropic <-> OpenAI | API Key | `glm-4-plus` |
| Groq | OpenAICompatible | Anthropic <-> OpenAI | API Key | `llama-3.3-70b` |
| Mistral | OpenAICompatible | Anthropic <-> OpenAI | API Key | `mistral-large-latest` |
| Together AI | OpenAICompatible | Anthropic <-> OpenAI | API Key | `meta-llama/...` |
| Perplexity | OpenAICompatible | Anthropic <-> OpenAI | API Key | `sonar-pro` |
| Cerebras | OpenAICompatible | Anthropic <-> OpenAI | API Key | `llama-3.3-70b` |
| Azure OpenAI | OpenAICompatible | Anthropic <-> OpenAI | api-key header | `gpt-4o` |
| Google Vertex AI | DirectAnthropic | None | Bearer (gcloud) | `claude-sonnet-4@...` |
| Ollama | OpenAICompatible | Anthropic <-> OpenAI | None | `qwen2.5:72b` |
| LM Studio | OpenAICompatible | Anthropic <-> OpenAI | None | local model |
| ChatGPT/Codex sub | OpenAIResponses | Anthropic <-> Responses | OAuth (PKCE/Device) | `gpt-5.3-codex` |
| Claude Max sub | DirectAnthropic | None | OAuth (file) | `claude-sonnet-4` |
| GitHub Copilot | OpenAICompatible | Anthropic <-> OpenAI | OAuth (Device+Bearer) | `gpt-4o` |
| GitLab Duo | OpenAICompatible | Anthropic <-> OpenAI | GITLAB_TOKEN | `claude-sonnet-4` |

## Configuration

Claudex searches for config files in this order:

1. `$CLAUDEX_CONFIG` environment variable
2. `./claudex.toml` or `./claudex.yaml` (current directory)
3. `./.claudex/config.toml`
4. Parent directories (up to 10 levels)
5. `~/.config/claudex/config.toml` (global, recommended)

Supports TOML and YAML formats. See [`config.example.toml`](./config.example.toml) for the full reference.

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
| `claudex proxy start [-p port] [-d]` | Start proxy (optionally as daemon) |
| `claudex proxy stop` | Stop proxy daemon |
| `claudex proxy status` | Show proxy status |
| `claudex dashboard` | Launch TUI dashboard |
| `claudex config show [--raw] [--json]` | Show loaded config |
| `claudex config init [--yaml]` | Create config in current directory |
| `claudex config edit [--global]` | Open config in $EDITOR |
| `claudex config validate [--connectivity]` | Validate config |
| `claudex config get <key>` | Get a config value |
| `claudex config set <key> <value>` | Set a config value |
| `claudex config export --format <fmt>` | Export config (json/toml/yaml) |
| `claudex update [--check]` | Self-update from GitHub Releases |
| `claudex auth login <provider>` | OAuth login |
| `claudex auth login github --enterprise-url <domain>` | GitHub Enterprise Copilot |
| `claudex auth status` | Show OAuth token status |
| `claudex auth logout <profile>` | Remove OAuth token |
| `claudex auth refresh <profile>` | Force refresh OAuth token |
| `claudex sets add <source> [--global]` | Install a configuration set |
| `claudex sets remove <name>` | Remove a configuration set |
| `claudex sets list [--global]` | List installed sets |
| `claudex sets update [name]` | Update sets to latest |

## OAuth Subscriptions

Use existing subscriptions instead of API keys:

```bash
# ChatGPT subscription (auto-detects existing Codex CLI credentials)
claudex auth login chatgpt --profile codex-sub

# ChatGPT force browser login
claudex auth login chatgpt --profile codex-sub --force

# ChatGPT headless (SSH/no-browser)
claudex auth login chatgpt --profile codex-sub --force --headless

# GitHub Copilot
claudex auth login github --profile copilot

# GitHub Copilot Enterprise
claudex auth login github --profile copilot-ent --enterprise-url company.ghe.com

# GitLab Duo (reads GITLAB_TOKEN env)
claudex auth login gitlab --profile gitlab-duo

# Check status
claudex auth status

# Run with subscription
claudex run codex-sub
```

Supported: `claude`, `chatgpt`/`openai`, `google`, `qwen`, `kimi`, `github`/`copilot`, `gitlab`

## Model Slot Mapping

Map Claude Code's `/model` switcher (haiku/sonnet/opus) to any provider's models:

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
├── main.rs
├── cli.rs
├── update.rs
├── util.rs
├── config/
│   ├── mod.rs          # Config discovery + parsing (figment)
│   ├── cmd.rs          # config get/set/export/validate subcommands
│   └── profile.rs      # Profile CRUD + connectivity test
├── process/
│   ├── mod.rs
│   ├── launch.rs       # Claude process launcher
│   └── daemon.rs       # PID file + process management
├── oauth/
│   ├── mod.rs          # AuthType, OAuthProvider, OAuthToken
│   ├── source.rs       # Layer 1: credential sources (env/file/keyring)
│   ├── exchange.rs     # Layer 2: token exchange (PKCE/device code/refresh)
│   ├── manager.rs      # Layer 3: cache + concurrent dedup + 401 retry
│   ├── handler.rs      # OAuthProviderHandler trait
│   ├── providers.rs    # Login/refresh/status CLI logic
│   ├── server.rs       # OAuth callback server + device code polling
│   └── token.rs        # Re-exports
├── proxy/
│   ├── mod.rs          # Axum server + ProxyState
│   ├── handler.rs      # Request routing + circuit breaker + 401 retry
│   ├── adapter/        # Provider-specific adapters
│   │   ├── mod.rs      # ProviderAdapter trait + factory
│   │   ├── direct.rs   # DirectAnthropic (passthrough)
│   │   ├── chat_completions.rs  # OpenAI Chat Completions
│   │   └── responses.rs         # OpenAI Responses API
│   ├── translate/      # Protocol translation
│   │   ├── chat_completions.rs
│   │   ├── chat_completions_stream.rs
│   │   ├── responses.rs
│   │   └── responses_stream.rs
│   ├── context_engine.rs
│   ├── fallback.rs     # Circuit breaker
│   ├── health.rs
│   ├── metrics.rs
│   ├── models.rs
│   ├── error.rs
│   └── util.rs
├── router/
│   ├── mod.rs
│   └── classifier.rs
├── context/
│   ├── mod.rs
│   ├── compression.rs
│   ├── sharing.rs
│   └── rag.rs
├── sets/               # Configuration sets management
│   ├── mod.rs
│   ├── schema.rs
│   ├── source.rs
│   ├── install.rs
│   ├── lock.rs
│   ├── conflict.rs
│   └── mcp.rs
├── terminal/           # Terminal detection + hyperlinks
│   ├── mod.rs
│   ├── detect.rs
│   ├── osc8.rs
│   └── pty.rs
└── tui/
    ├── mod.rs
    ├── dashboard.rs
    ├── input.rs
    └── widgets.rs
```

## License

[MIT](./LICENSE)
