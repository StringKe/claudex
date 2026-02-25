# Claudex - Claude Code 项目指南

## 项目概述

Claudex 是一个 Rust 实现的多实例 Claude Code 管理器，内置智能翻译代理。它通过本地 proxy 将 Claude Code 的 Anthropic API 请求翻译并转发到多种 AI 提供商。

## 技术栈

- **语言**: Rust (Edition 2021)
- **异步运行时**: Tokio
- **Web 框架**: Axum 0.8
- **HTTP 客户端**: reqwest (rustls-tls)
- **TUI**: ratatui + crossterm
- **配置**: TOML (toml crate)
- **日志**: tracing + tracing-subscriber
- **错误处理**: anyhow + thiserror

## 项目结构

```
src/
├── main.rs              # 入口 + CLI dispatch
├── cli.rs               # clap 子命令定义
├── config.rs            # 配置解析（API key 直接存 config，不自动读 keyring）
├── profile.rs           # Profile 管理
├── launch.rs            # 启动 claude 进程（含 Claude OAuth 特殊处理）
├── oauth/               # OAuth 订阅认证
│   ├── mod.rs           # AuthType, OAuthProvider, OAuthToken 类型
│   ├── token.rs         # 外部 CLI token 读取（Codex/Claude/Gemini）
│   ├── server.rs        # 本地回调服务器 + Device Code 轮询
│   └── providers.rs     # 各平台登录/刷新/状态逻辑
├── daemon.rs            # PID 文件 + 进程管理
├── metrics.rs           # 请求指标
├── proxy/               # 翻译代理
│   ├── mod.rs           # Axum server
│   ├── handler.rs       # 请求处理
│   ├── translation.rs   # Anthropic ↔ OpenAI 翻译
│   ├── streaming.rs     # SSE 流式翻译
│   ├── fallback.rs      # 断路器
│   ├── health.rs        # 健康检查
│   └── models.rs        # /v1/models 端点
├── router/              # 智能路由
│   ├── mod.rs
│   └── classifier.rs    # 意图分类
├── context/             # 上下文引擎
│   ├── mod.rs
│   ├── compression.rs   # 对话压缩
│   ├── sharing.rs       # 跨 profile 共享
│   └── rag.rs           # 本地 RAG
└── tui/                 # TUI 仪表盘
    ├── mod.rs
    ├── dashboard.rs
    ├── widgets.rs
    └── input.rs
```

## 构建与运行

```bash
# 开发构建
cargo build

# Release 构建
cargo build --release

# 检查
cargo check

# Clippy 检查
cargo clippy

# 运行
cargo run -- profile list
cargo run -- run grok
cargo run -- proxy start
```

## 核心概念

### Provider Types

- `DirectAnthropic`: 原生 Anthropic API，直接转发（Anthropic、MiniMax）
- `OpenAICompatible`: OpenAI 兼容 API，需要协议翻译（OpenRouter、Grok、OpenAI、DeepSeek、Kimi、GLM、Ollama）

### 翻译层

`proxy/translation.rs` 实现完整的 Anthropic ↔ OpenAI 双向翻译：
- 请求翻译：system prompt、messages（含图片、tool_use）、tools、tool_choice
- 响应翻译：content blocks、tool calls、usage、stop_reason
- 流式翻译（`proxy/streaming.rs`）：SSE 事件转换、tool call 状态机

### 认证方式

- **API Key**（默认）：配置 `api_key` 或 `api_key_keyring`
- **OAuth 订阅**：配置 `auth_type = "oauth"` + `oauth_provider`，通过 `claudex auth login` 完成
  - Claude subscription 特殊处理：跳过代理，让 Claude Code 直接使用自身 OAuth
  - 其他 provider：OAuth token 存入 keyring，代理自动加载和刷新

### 配置

配置文件位于 `~/.config/claudex/config.toml`，参考 `config.example.toml`。

## 开发规范

- 编译检查：修改后运行 `cargo check` 确认通过
- 代码检查：提交前运行 `cargo clippy` 确认无 warning
- 错误处理：使用 `anyhow::Result` + `?` 传播，不要 unwrap 生产代码
- 日志：使用 `tracing::info!` / `tracing::warn!` / `tracing::error!`
- 格式化：运行 `cargo fmt` 保持统一风格

## 关键文件说明

| 文件 | 修改频率 | 说明 |
|------|---------|------|
| `config.rs` | 中 | 新增配置字段时需要同步更新 |
| `translation.rs` | 高 | 翻译逻辑是核心，新提供商可能需要特殊处理 |
| `streaming.rs` | 高 | 流式翻译复杂度高，需要仔细处理状态机 |
| `handler.rs` | 中 | 新增路由或中间件时修改，含 OAuth token 懒刷新 |
| `cli.rs` | 低 | 新增子命令时修改 |
| `oauth/` | 低 | OAuth 认证模块，新增 provider 时修改 |
| `launch.rs` | 低 | Claude OAuth subscription 特殊处理逻辑在此 |
