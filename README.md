# Claudex

Multi-instance Claude Code manager with intelligent translation proxy.

通过统一代理管理多个 AI 模型提供商，让 Claude Code 无缝使用 Grok、ChatGPT、DeepSeek、MiniMax、Kimi、GLM 等模型。

## Features

- **Multi-provider proxy** — Anthropic-native providers (Anthropic, MiniMax, OpenRouter) direct passthrough; OpenAI-compatible providers (Grok, OpenAI, DeepSeek, Kimi, GLM) automatic Anthropic ↔ OpenAI translation
- **Streaming translation** — Full SSE stream translation with tool call support
- **Smart routing** — Optional intent-based routing via local classifier model (Ollama)
- **Context engine** — Conversation compression, cross-profile sharing, local RAG
- **Circuit breaker + failover** — Automatic fallback to backup providers on failure
- **TUI dashboard** — Real-time profile health, metrics, and logs
- **Local model support** — Any OpenAI-compatible local service (Ollama, vLLM, LM Studio)
- **Keyring integration** — Secure API key storage via OS keychain

## Quick Start

```bash
# Build
cargo build --release

# Copy and edit config
cp config.example.toml ~/.config/claudex/config.toml
# Edit with your API keys

# List profiles
claudex profile list

# Test connectivity
claudex profile test all

# Run Claude Code with a specific provider
claudex run grok
claudex run deepseek
claudex run chatgpt

# Override model for a session
claudex run grok -m grok-3-mini-beta

# Start proxy manually
claudex proxy start

# Launch TUI dashboard
claudex dashboard
```

## How It Works

```
claudex run grok
    │
    ├─ Start proxy (if not running) → 127.0.0.1:13456
    │
    └─ exec claude with env vars:
        CLAUDE_CONFIG_DIR=~/.config/claude-grok
        ANTHROPIC_BASE_URL=http://127.0.0.1:13456/proxy/grok
        ANTHROPIC_API_KEY=claudex-passthrough
        ANTHROPIC_MODEL=grok-3-beta
```

Proxy receives request → extracts profile from URL path → routes:
- **DirectAnthropic**: Forward with correct headers + base URL
- **OpenAICompatible**: Translate Anthropic → OpenAI → forward → translate response back

## Configuration

See [`config.example.toml`](./config.example.toml) for full documentation.

### Provider Types

| Provider | Type | Translation |
|----------|------|-------------|
| Anthropic | `DirectAnthropic` | None |
| MiniMax | `DirectAnthropic` | None |
| OpenRouter | `DirectAnthropic` | None |
| Grok (xAI) | `OpenAICompatible` | Anthropic ↔ OpenAI |
| OpenAI | `OpenAICompatible` | Anthropic ↔ OpenAI |
| DeepSeek | `OpenAICompatible` | Anthropic ↔ OpenAI |
| Kimi | `OpenAICompatible` | Anthropic ↔ OpenAI |
| GLM | `OpenAICompatible` | Anthropic ↔ OpenAI |
| Ollama/vLLM | `OpenAICompatible` | Anthropic ↔ OpenAI |

### Keyring

Store API keys securely:

```toml
[[profiles]]
name = "grok"
api_key_keyring = "grok-api-key"  # reads from OS keychain
```

## License

MIT
