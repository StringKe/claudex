# Claudex 需求文档

## 项目背景

用户拥有多个 AI 模型提供商的高级套餐（Grok、ChatGPT、DeepSeek、MiniMax、Kimi、GLM、OpenRouter 等），希望通过 Claude Code CLI 统一使用这些模型。

Claude Code 支持以下环境变量，使得代理架构可行：
- `CLAUDE_CONFIG_DIR` — 隔离配置目录
- `ANTHROPIC_BASE_URL` — 自定义 API 地址
- `ANTHROPIC_API_KEY` — API 密钥
- `ANTHROPIC_MODEL` — 模型名
- `ANTHROPIC_CUSTOM_HEADERS` — 自定义请求头

## 核心需求

### 1. 多提供商代理

通过本地 proxy 统一转发请求到不同 AI 提供商。

**提供商兼容性矩阵：**

| 提供商 | API 格式 | Base URL | Anthropic 原生 | 需要翻译 |
|--------|----------|----------|----------------|----------|
| Anthropic | Anthropic | `https://api.anthropic.com` | 是 | 否 |
| MiniMax | 双模式 | `https://api.minimax.io/anthropic` | 是 | 否 |
| OpenRouter | 双模式 | `https://openrouter.ai/api` | 是 | 否 |
| Grok (xAI) | OpenAI | `https://api.x.ai/v1` | 否 | 是 |
| OpenAI | OpenAI | `https://api.openai.com/v1` | 否 | 是 |
| DeepSeek | OpenAI | `https://api.deepseek.com` | 否 | 是 |
| Kimi | OpenAI | `https://api.moonshot.cn/v1` | 否 | 是 |
| GLM (智谱) | OpenAI | `https://open.bigmodel.cn/api/paas/v4` | 否 | 是 |

### 2. 协议翻译

对于 OpenAI 兼容提供商，需要完整实现 Anthropic Messages API ↔ OpenAI Chat Completions API 双向翻译。

**翻译范围：**
- 请求翻译：system prompt、messages、tools、tool_choice、参数映射
- 响应翻译：content blocks、tool calls、usage、stop reason
- 流式翻译：SSE 事件格式转换、tool call 状态机

### 3. 智能路由（可选）

通过本地分类模型（Ollama 等）分析用户请求意图，自动路由到最适合的提供商。

**路由规则示例：**
- 代码生成 → DeepSeek
- 项目分析 → Grok
- 创意写作 → ChatGPT
- 联网搜索 → Kimi
- 数学推理 → DeepSeek

### 4. 上下文引擎（可选）

- **对话压缩**：超过 token 阈值时用本地模型生成摘要
- **跨 Profile 共享**：从 Profile A 对话中提取关键信息注入 Profile B
- **本地 RAG**：对项目文件建立向量索引，检索相关片段注入请求

### 5. 容错与高可用

- 断路器：检测故障自动熔断，定期尝试恢复
- Failover：主提供商失败时自动切换到备用提供商
- 健康检查：后台定期检测所有 Profile 连通性

### 6. TUI 仪表盘

实时展示所有 Profile 的健康状态、延迟、请求量、日志。

### 7. 本地模型支持

通过 `ProviderType::OpenAICompatible` 原生支持任何 OpenAI 兼容的本地推理服务：Ollama、vLLM、llama.cpp server、LM Studio 等。

## 非功能需求

- **性能**：代理延迟 < 10ms（不含上游响应时间）
- **安全**：API 密钥支持 OS keychain 存储
- **可观测性**：结构化日志（tracing）、请求指标
- **配置**：TOML 格式、支持热重载
- **跨平台**：macOS + Linux

## 技术选型

- 语言：Rust
- 异步运行时：Tokio
- Web 框架：Axum
- HTTP 客户端：reqwest
- TUI：ratatui + crossterm
- 配置：toml
- 日志：tracing
