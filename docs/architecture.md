# Claudex 架构文档

## 系统架构

```
claudex run <profile>
    │
    ├─ 启动 proxy（如未运行）→ 127.0.0.1:13456
    │
    └─ exec claude 并设置环境变量：
        CLAUDE_CONFIG_DIR=~/.config/claude-<profile>
        ANTHROPIC_BASE_URL=http://127.0.0.1:13456/proxy/<profile>
        ANTHROPIC_API_KEY=claudex-passthrough
        ANTHROPIC_MODEL=<default_model>
```

## 请求处理流程

```
Claude Code → HTTP POST /proxy/{profile}/v1/messages
    │
    ├─ 从 URL path 提取 profile 名
    ├─ 查找 profile 配置
    │
    ├─ DirectAnthropic（Anthropic/MiniMax）
    │   └─ 直接转发（替换 header + base_url）
    │
    └─ OpenAICompatible（Grok/OpenAI/DeepSeek/Kimi/GLM/Ollama）
        ├─ Anthropic → OpenAI 请求翻译
        ├─ 转发到目标提供商
        └─ OpenAI → Anthropic 响应翻译
```

## 模块划分

| 模块 | 职责 |
|------|------|
| `cli.rs` | 命令行参数定义 |
| `config.rs` | 配置解析、keyring 集成 |
| `profile.rs` | Profile CRUD、连通性测试 |
| `launch.rs` | 启动 Claude Code 进程 |
| `daemon.rs` | PID 文件、进程管理 |
| `metrics.rs` | 请求指标收集 |
| `proxy/handler.rs` | 请求路由与转发 |
| `proxy/translation.rs` | Anthropic ↔ OpenAI 协议翻译 |
| `proxy/streaming.rs` | SSE 流式翻译 |
| `proxy/fallback.rs` | 断路器 |
| `proxy/health.rs` | 后台健康检查 |
| `proxy/models.rs` | 模型列表聚合 |
| `router/` | 智能路由（意图分类） |
| `context/` | 上下文引擎（压缩/共享/RAG） |
| `tui/` | TUI 仪表盘 |

## 数据流

### 非流式请求

```
Anthropic Request (JSON)
    → anthropic_to_openai() 翻译
    → POST upstream /chat/completions
    → OpenAI Response (JSON)
    → openai_to_anthropic() 翻译
    → Anthropic Response (JSON)
```

### 流式请求

```
Anthropic Request (stream: true)
    → anthropic_to_openai() 翻译
    → POST upstream /chat/completions (stream: true)
    → OpenAI SSE stream
    → translate_sse_stream() 逐事件翻译
        - message_start
        - content_block_start / content_block_delta / content_block_stop
        - message_delta / message_stop
    → Anthropic SSE stream
```

## 断路器状态机

```
Closed ──[failure >= threshold]──→ Open
  ↑                                  │
  │                         [recovery_timeout]
  │                                  ↓
  └──────[probe success]──── HalfOpen
                               │
                    [probe failure]──→ Open
```

## 配置结构

```
~/.config/claudex/
└── config.toml          # 主配置文件

~/.config/claude-{profile}/  # 每个 profile 独立的 Claude 配置目录
```
