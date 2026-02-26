<p align="center">
  <h1 align="center">Claudex</h1>
  <p align="center">多实例 Claude Code 管理器，内置智能翻译代理</p>
</p>

<p align="center">
  <a href="https://github.com/StringKe/claudex/actions/workflows/ci.yml"><img src="https://github.com/StringKe/claudex/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
  <a href="https://github.com/StringKe/claudex/releases"><img src="https://github.com/StringKe/claudex/actions/workflows/release.yml/badge.svg" alt="Release"></a>
  <a href="https://github.com/StringKe/claudex/blob/main/LICENSE"><img src="https://img.shields.io/github/license/StringKe/claudex" alt="License"></a>
  <a href="https://github.com/StringKe/claudex/releases"><img src="https://img.shields.io/github/v/release/StringKe/claudex" alt="Latest Release"></a>
</p>

<p align="center">
  <a href="https://stringke.github.io/claudex/zh-CN/">文档</a>
</p>

<p align="center">
  <a href="./README.md">English</a> |
  简体中文 |
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

Claudex 是一个统一代理，通过自动协议翻译让 [Claude Code](https://docs.anthropic.com/en/docs/claude-code) 无缝对接多种 AI 提供商。

## 功能特性

- **多提供商代理** — DirectAnthropic 直通 + Anthropic <-> OpenAI Chat Completions 翻译 + Anthropic <-> Responses API 翻译
- **20+ 提供商** — Anthropic、OpenRouter、Grok、OpenAI、DeepSeek、Kimi、GLM、Groq、Mistral、Together AI、Perplexity、Cerebras、Azure OpenAI、Google Vertex AI、Ollama、LM Studio 等
- **流式翻译** — 完整 SSE 流式翻译，支持工具调用
- **断路器 + 故障转移** — 自动切换备用提供商，阈值可配置
- **智能路由** — 基于意图的自动路由，使用本地分类器
- **上下文引擎** — 对话压缩、跨 profile 共享、带向量嵌入的本地 RAG
- **OAuth 订阅** — ChatGPT/Codex、Claude Max、GitHub Copilot、GitLab Duo、Google Gemini、Qwen、Kimi
- **配置集** — 从 git 仓库安装和管理可复用的 Claude Code 配置集
- **TUI 仪表盘** — 实时 profile 健康状态、指标、日志及快速启动
- **自动更新** — `claudex update` 从 GitHub 下载最新版本

## 安装

```bash
# 一键安装（Linux / macOS）
curl -fsSL https://raw.githubusercontent.com/StringKe/claudex/main/install.sh | bash

# 从源码构建
cargo install --git https://github.com/StringKe/claudex

# 或从 GitHub Releases 下载
# https://github.com/StringKe/claudex/releases
```

### 系统要求

- macOS（Intel / Apple Silicon）或 Linux（x86_64 / ARM64）
- 已安装 [Claude Code](https://docs.anthropic.com/en/docs/claude-code)
- Windows：从 [Releases](https://github.com/StringKe/claudex/releases) 下载预构建二进制文件

## 快速上手

```bash
# 1. 初始化配置
claudex config init

# 2. 交互式添加提供商 profile
claudex profile add

# 3. 测试连通性
claudex profile test all

# 4. 使用指定提供商运行 Claude Code
claudex run grok

# 5. 或使用智能路由自动选择最佳提供商
claudex run auto
```

## 工作原理

```
claudex run openrouter-claude
    │
    ├── 启动代理（如未运行）→ 127.0.0.1:13456
    │
    └── 携带环境变量执行 claude：
        ANTHROPIC_BASE_URL=http://127.0.0.1:13456/proxy/openrouter-claude
        ANTHROPIC_AUTH_TOKEN=claudex-passthrough
        ANTHROPIC_MODEL=anthropic/claude-sonnet-4
        ANTHROPIC_DEFAULT_HAIKU_MODEL=...
        ANTHROPIC_DEFAULT_SONNET_MODEL=...
        ANTHROPIC_DEFAULT_OPUS_MODEL=...
```

代理拦截请求并处理协议翻译：

- **DirectAnthropic**（Anthropic、MiniMax、Vertex AI）→ 附加正确请求头后直接转发
- **OpenAICompatible**（Grok、OpenAI、DeepSeek 等）→ Anthropic → OpenAI Chat Completions → 翻译响应返回
- **OpenAIResponses**（ChatGPT/Codex 订阅）→ Anthropic → Responses API → 翻译响应返回

## 提供商兼容性

| 提供商 | 类型 | 翻译方式 | 认证 | 示例模型 |
|--------|------|----------|------|----------|
| Anthropic | DirectAnthropic | 无 | API Key | `claude-sonnet-4-20250514` |
| MiniMax | DirectAnthropic | 无 | API Key | `claude-sonnet-4-20250514` |
| OpenRouter | OpenAICompatible | Anthropic <-> OpenAI | API Key | `anthropic/claude-sonnet-4` |
| Grok (xAI) | OpenAICompatible | Anthropic <-> OpenAI | API Key | `grok-3-beta` |
| OpenAI | OpenAICompatible | Anthropic <-> OpenAI | API Key | `gpt-4o` |
| DeepSeek | OpenAICompatible | Anthropic <-> OpenAI | API Key | `deepseek-chat` |
| Kimi | OpenAICompatible | Anthropic <-> OpenAI | API Key | `kimi-k2-0905-preview` |
| GLM（智谱）| OpenAICompatible | Anthropic <-> OpenAI | API Key | `glm-4-plus` |
| Groq | OpenAICompatible | Anthropic <-> OpenAI | API Key | `llama-3.3-70b` |
| Mistral | OpenAICompatible | Anthropic <-> OpenAI | API Key | `mistral-large-latest` |
| Together AI | OpenAICompatible | Anthropic <-> OpenAI | API Key | `meta-llama/...` |
| Perplexity | OpenAICompatible | Anthropic <-> OpenAI | API Key | `sonar-pro` |
| Cerebras | OpenAICompatible | Anthropic <-> OpenAI | API Key | `llama-3.3-70b` |
| Azure OpenAI | OpenAICompatible | Anthropic <-> OpenAI | api-key header | `gpt-4o` |
| Google Vertex AI | DirectAnthropic | 无 | Bearer (gcloud) | `claude-sonnet-4@...` |
| Ollama | OpenAICompatible | Anthropic <-> OpenAI | 无 | `qwen2.5:72b` |
| LM Studio | OpenAICompatible | Anthropic <-> OpenAI | 无 | 本地模型 |
| ChatGPT/Codex 订阅 | OpenAIResponses | Anthropic <-> Responses | OAuth (PKCE/Device) | `gpt-5.3-codex` |
| Claude Max 订阅 | DirectAnthropic | 无 | OAuth (file) | `claude-sonnet-4` |
| GitHub Copilot | OpenAICompatible | Anthropic <-> OpenAI | OAuth (Device+Bearer) | `gpt-4o` |
| GitLab Duo | OpenAICompatible | Anthropic <-> OpenAI | GITLAB_TOKEN | `claude-sonnet-4` |

## 配置

Claudex 按以下顺序查找配置文件：

1. `$CLAUDEX_CONFIG` 环境变量
2. `./claudex.toml` 或 `./claudex.yaml`（当前目录）
3. `./.claudex/config.toml`
4. 父级目录（最多向上 10 层）
5. `~/.config/claudex/config.toml`（全局，推荐）

支持 TOML 和 YAML 格式。完整参考见 [`config.example.toml`](./config.example.toml)。

## CLI 命令参考

| 命令 | 说明 |
|------|------|
| `claudex run <profile>` | 使用指定提供商运行 Claude Code |
| `claudex run auto` | 智能路由，自动选择最佳提供商 |
| `claudex run <profile> -m <model>` | 为本次会话覆盖模型 |
| `claudex profile list` | 列出所有已配置的 profile |
| `claudex profile add` | 交互式 profile 配置向导 |
| `claudex profile show <name>` | 显示 profile 详情 |
| `claudex profile remove <name>` | 删除 profile |
| `claudex profile test <name\|all>` | 测试提供商连通性 |
| `claudex proxy start [-p port] [-d]` | 启动代理（可选后台守护进程模式）|
| `claudex proxy stop` | 停止代理守护进程 |
| `claudex proxy status` | 显示代理状态 |
| `claudex dashboard` | 启动 TUI 仪表盘 |
| `claudex config show [--raw] [--json]` | 显示已加载的配置 |
| `claudex config init [--yaml]` | 在当前目录创建配置文件 |
| `claudex config edit [--global]` | 用 $EDITOR 打开配置文件 |
| `claudex config validate [--connectivity]` | 校验配置 |
| `claudex config get <key>` | 获取配置值 |
| `claudex config set <key> <value>` | 设置配置值 |
| `claudex config export --format <fmt>` | 导出配置（json/toml/yaml）|
| `claudex update [--check]` | 从 GitHub Releases 自动更新 |
| `claudex auth login <provider>` | OAuth 登录 |
| `claudex auth login github --enterprise-url <domain>` | GitHub Enterprise Copilot |
| `claudex auth status` | 显示 OAuth token 状态 |
| `claudex auth logout <profile>` | 删除 OAuth token |
| `claudex auth refresh <profile>` | 强制刷新 OAuth token |
| `claudex sets add <source> [--global]` | 安装配置集 |
| `claudex sets remove <name>` | 删除配置集 |
| `claudex sets list [--global]` | 列出已安装的配置集 |
| `claudex sets update [name]` | 将配置集更新到最新版本 |

## OAuth 订阅

使用现有订阅替代 API Key：

```bash
# ChatGPT 订阅（自动检测已有 Codex CLI 凭证）
claudex auth login chatgpt --profile codex-sub

# ChatGPT 强制浏览器登录
claudex auth login chatgpt --profile codex-sub --force

# ChatGPT 无头模式（SSH / 无浏览器环境）
claudex auth login chatgpt --profile codex-sub --force --headless

# GitHub Copilot
claudex auth login github --profile copilot

# GitHub Copilot Enterprise
claudex auth login github --profile copilot-ent --enterprise-url company.ghe.com

# GitLab Duo（读取 GITLAB_TOKEN 环境变量）
claudex auth login gitlab --profile gitlab-duo

# 查看状态
claudex auth status

# 使用订阅运行
claudex run codex-sub
```

支持的提供商：`claude`、`chatgpt`/`openai`、`google`、`qwen`、`kimi`、`github`/`copilot`、`gitlab`

## 模型槽位映射

将 Claude Code 的 `/model` 切换器（haiku/sonnet/opus）映射到任意提供商的模型：

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

## 架构

```
src/
├── main.rs
├── cli.rs
├── update.rs
├── util.rs
├── config/
│   ├── mod.rs          # 配置发现与解析（figment）
│   ├── cmd.rs          # config get/set/export/validate 子命令
│   └── profile.rs      # Profile CRUD + 连通性测试
├── process/
│   ├── mod.rs
│   ├── launch.rs       # Claude 进程启动器
│   └── daemon.rs       # PID 文件 + 进程管理
├── oauth/
│   ├── mod.rs          # AuthType、OAuthProvider、OAuthToken
│   ├── source.rs       # 第一层：凭证来源（env/file/keyring）
│   ├── exchange.rs     # 第二层：token 换取（PKCE/device code/refresh）
│   ├── manager.rs      # 第三层：缓存 + 并发去重 + 401 重试
│   ├── handler.rs      # OAuthProviderHandler trait
│   ├── providers.rs    # 登录/刷新/状态 CLI 逻辑
│   ├── server.rs       # OAuth 回调服务器 + device code 轮询
│   └── token.rs        # 重导出
├── proxy/
│   ├── mod.rs          # Axum 服务器 + ProxyState
│   ├── handler.rs      # 请求路由 + 断路器 + 401 重试
│   ├── adapter/        # 各提供商适配器
│   │   ├── mod.rs      # ProviderAdapter trait + 工厂
│   │   ├── direct.rs   # DirectAnthropic（直通）
│   │   ├── chat_completions.rs  # OpenAI Chat Completions
│   │   └── responses.rs         # OpenAI Responses API
│   ├── translate/      # 协议翻译
│   │   ├── chat_completions.rs
│   │   ├── chat_completions_stream.rs
│   │   ├── responses.rs
│   │   └── responses_stream.rs
│   ├── context_engine.rs
│   ├── fallback.rs     # 断路器
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
├── sets/               # 配置集管理
│   ├── mod.rs
│   ├── schema.rs
│   ├── source.rs
│   ├── install.rs
│   ├── lock.rs
│   ├── conflict.rs
│   └── mcp.rs
├── terminal/           # 终端检测 + 超链接
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

## 许可证

[MIT](./LICENSE)
