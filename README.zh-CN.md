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
  <a href="https://stringke.github.io/claudex/">文档站</a> |
  <a href="./README.md">English</a>
</p>

---

Claudex 是一个统一代理，让 [Claude Code](https://docs.anthropic.com/en/docs/claude-code) 无缝接入多个 AI 提供商 — Grok、ChatGPT、DeepSeek、MiniMax、Kimi、GLM、Ollama 等 — 通过自动 Anthropic-to-OpenAI 协议翻译实现透明转发。

## 功能特性

- **多提供商代理** — DirectAnthropic 直通（Anthropic、MiniMax、OpenRouter）+ 自动 Anthropic <-> OpenAI 翻译（Grok、OpenAI、DeepSeek、Kimi、GLM）
- **流式翻译** — 完整 SSE 流翻译，支持 tool call
- **断路器 + 故障转移** — 失败时自动切换到备用提供商，可配置阈值
- **智能路由** — 基于意图的自动路由，通过本地分类器将 code/analysis/creative/search/math 映射到最优 profile
- **上下文引擎** — 对话压缩、跨 profile 上下文共享、本地 RAG 向量检索
- **TUI 仪表盘** — 实时 profile 健康状态、指标、日志和快速启动
- **配置发现** — 从当前目录向上自动搜索配置文件
- **自更新** — `claudex update` 从 GitHub 下载最新版本
- **本地模型支持** — Ollama、vLLM、LM Studio 或任何 OpenAI 兼容服务
- **OAuth 订阅认证** — 通过 `claudex auth login` 使用已有 CLI 订阅（ChatGPT Plus via Codex CLI、Claude Max 等）
- **模型 slot 映射** — 通过 `[profiles.models]` 配置 Claude Code `/model` 切换器对应的模型名
- **非交互模式** — `claudex run <profile> "prompt" --print` 一次性执行
- **工具名兼容** — 自动截断超过 OpenAI 64 字符限制的工具名并透明还原

## 安装

```bash
# 一键安装（Linux / macOS）
curl -fsSL https://raw.githubusercontent.com/StringKe/claudex/main/install.sh | bash

# 从源码安装
cargo install --git https://github.com/StringKe/claudex

# 或从 GitHub Releases 下载
# https://github.com/StringKe/claudex/releases
```

### 系统要求

- macOS（Intel / Apple Silicon）或 Linux（x86_64 / ARM64）
- 已安装 [Claude Code](https://docs.anthropic.com/en/docs/claude-code)
- Windows：从 [Releases](https://github.com/StringKe/claudex/releases) 下载预编译二进制文件

## 快速开始

```bash
# 1. 初始化配置
claudex config --init

# 2. 交互式添加提供商 Profile
claudex profile add

# 3. 测试连通性
claudex profile test all

# 4. 使用指定提供商运行 Claude Code
claudex run grok

# 5. 或使用智能路由自动选择最优提供商
claudex run auto
```

## 工作原理

```
claudex run openrouter-claude
    │
    ├── 启动代理（如果未运行）→ 127.0.0.1:13456
    │
    └── 执行 claude，设置环境变量：
        ANTHROPIC_BASE_URL=http://127.0.0.1:13456/proxy/openrouter-claude
        ANTHROPIC_AUTH_TOKEN=claudex-passthrough   (Gateway 模式，不与 claude.ai 冲突)
        ANTHROPIC_MODEL=anthropic/claude-sonnet-4
```

代理拦截请求并透明处理协议翻译：

- **DirectAnthropic** 提供商（Anthropic、MiniMax、OpenRouter）→ 替换 header 直接转发
- **OpenAICompatible** 提供商（Grok、OpenAI、DeepSeek 等）→ Anthropic → OpenAI 翻译 → 转发 → 翻译响应

## 提供商兼容性

| 提供商 | 类型 | 翻译 | 示例模型 |
|--------|------|------|----------|
| Anthropic | `DirectAnthropic` | 无 | `claude-sonnet-4-20250514` |
| MiniMax | `DirectAnthropic` | 无 | `claude-sonnet-4-20250514` |
| OpenRouter | `DirectAnthropic` | 无 | `anthropic/claude-sonnet-4` |
| Grok (xAI) | `OpenAICompatible` | Anthropic <-> OpenAI | `grok-3-beta` |
| OpenAI | `OpenAICompatible` | Anthropic <-> OpenAI | `gpt-4o` |
| DeepSeek | `OpenAICompatible` | Anthropic <-> OpenAI | `deepseek-chat` |
| Kimi | `OpenAICompatible` | Anthropic <-> OpenAI | `moonshot-v1-128k` |
| GLM（智谱） | `OpenAICompatible` | Anthropic <-> OpenAI | `glm-4-plus` |
| Ollama | `OpenAICompatible` | Anthropic <-> OpenAI | `qwen2.5:72b` |

## 配置

Claudex 按以下顺序搜索配置文件：

1. `$CLAUDEX_CONFIG` 环境变量
2. `./claudex.toml`（当前目录）
3. `./.claudex/config.toml`
4. 父目录（最多向上 10 级）
5. `~/.config/claudex/config.toml`（XDG，推荐）
6. 平台配置目录（macOS `~/Library/Application Support/claudex/config.toml`）

```bash
# 查看加载的配置和搜索路径
claudex config

# 在当前目录创建本地配置
claudex config --init
```

完整配置参考请查看 [`config.example.toml`](./config.example.toml)，或访问[文档站](https://stringke.github.io/claudex/)获取详细指南。

## CLI 命令参考

| 命令 | 说明 |
|------|------|
| `claudex run <profile>` | 使用指定提供商运行 Claude Code |
| `claudex run auto` | 智能路由 — 自动选择最优提供商 |
| `claudex run <profile> -m <model>` | 临时覆盖模型 |
| `claudex profile list` | 列出所有已配置的 profile |
| `claudex profile add` | 交互式 profile 设置向导 |
| `claudex profile show <name>` | 查看 profile 详情 |
| `claudex profile remove <name>` | 删除 profile |
| `claudex profile test <name\|all>` | 测试提供商连通性 |
| `claudex proxy start [-p port] [-d]` | 启动代理服务器（可选守护进程模式） |
| `claudex proxy stop` | 停止代理守护进程 |
| `claudex proxy status` | 查看代理状态 |
| `claudex dashboard` | 启动 TUI 仪表盘 |
| `claudex config [--init]` | 查看或初始化配置 |
| `claudex update [--check]` | 从 GitHub Releases 自更新 |
| `claudex auth login <provider>` | OAuth 登录（claude/openai/google/qwen/kimi/github） |
| `claudex auth status` | 查看 OAuth token 状态 |
| `claudex auth logout <profile>` | 删除 OAuth token |
| `claudex auth refresh <profile>` | 强制刷新 OAuth token |

## 非交互模式

一次性执行，输出结果后退出：

```bash
claudex run openrouter-claude "解释这段代码" --print --dangerously-skip-permissions
```

## OAuth 订阅认证

使用已有 CLI 订阅（如 Codex CLI 的 ChatGPT Plus）：

```bash
# 从 Codex CLI 读取 token（~/.codex/auth.json）
claudex auth login openai --profile codex-sub

# 查看状态
claudex auth status

# 使用订阅运行
claudex run codex-sub
```

支持的提供商：`claude`（读 `~/.claude`）、`openai`（读 `~/.codex`）、`google`、`kimi`

## 模型 Slot 映射

通过 `[profiles.models]` 配置 Claude Code `/model` 切换器：

```toml
[[profiles]]
name = "openrouter-deepseek"
default_model = "deepseek/deepseek-chat-v3-0324"

[profiles.models]
haiku = "deepseek/deepseek-chat-v3-0324"
sonnet = "deepseek/deepseek-chat-v3-0324"
opus = "deepseek/deepseek-r1"
```

## 架构

```
src/
├── main.rs              # 入口 + CLI 分发
├── cli.rs               # clap 命令定义
├── config.rs            # 配置发现 + 解析 + keyring
├── profile.rs           # Profile 增删改查 + 连通性测试
├── launch.rs            # Claude 进程启动器
├── daemon.rs            # PID 文件 + 进程管理
├── metrics.rs           # 请求指标（原子计数器）
├── update.rs            # 通过 GitHub Releases 自更新
├── oauth/               # OAuth 订阅认证
│   ├── mod.rs           # AuthType、OAuthProvider、OAuthToken 类型
│   ├── token.rs         # 外部 CLI token 读取（Codex/Claude/Gemini）
│   ├── server.rs        # 本地回调服务器 + Device Code 轮询
│   └── providers.rs     # 各平台登录/状态逻辑
├── proxy/               # 翻译代理
│   ├── handler.rs       # 请求路由 + 断路器 + OAuth token 刷新
│   ├── translation.rs   # Anthropic <-> OpenAI 协议翻译（含工具名截断）
│   ├── streaming.rs     # SSE 流式翻译（状态机）
│   ├── fallback.rs      # 断路器实现
│   ├── health.rs        # 后台健康检查
│   └── models.rs        # /v1/models 聚合端点
├── router/              # 智能路由
│   └── classifier.rs    # 通过本地 LLM 进行意图分类
├── context/             # 上下文引擎
│   ├── compression.rs   # 对话压缩
│   ├── sharing.rs       # 跨 profile 上下文共享
│   └── rag.rs           # 本地 RAG 向量检索
└── tui/                 # TUI 仪表盘
    ├── dashboard.rs     # 仪表盘渲染
    ├── input.rs         # 键盘输入
    └── widgets.rs       # UI 组件
```

## 许可证

[MIT](./LICENSE)
