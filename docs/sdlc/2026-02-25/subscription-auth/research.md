# 调研报告：AI 平台订阅制认证支持

## 背景

当前 Claudex 仅支持 API Key 认证。各 AI 平台已推出订阅制计划，用户通过月费获得更高性价比。Claudex 作为 Claude Code 的启动器和代理，应支持用户直接使用订阅凭证。

**核心价值**：Claude Max $200/月 ≈ API 方式 $3,650/月 的同等用量，差距约 18 倍。

**TOS 澄清**：Claudex 是 Claude Code 的代理/启动器，OAuth token 最终由 Claude Code 进程使用，不属于"第三方工具使用"，不违反消费者 TOS。对于 Codex、Gemini CLI 等同理——Claudex 翻译后转发到对应平台的官方 API，本质上是一个带翻译功能的代理。

---

## 全平台订阅 & OAuth 认证调研

### 第一类：有官方 CLI Agent + OAuth 登录的平台

这些平台有自己的 CLI 工具，支持 OAuth 登录获取 token，Claudex 可以复用这些 token。

#### 1. Claude Code（Anthropic）

| 项目 | 详情 |
|------|------|
| 订阅计划 | Pro $20/月、Max 5x $100/月、Max 20x $200/月 |
| 认证方式 | 浏览器 OAuth 登录 claude.ai |
| Token 存储 | `~/.claude/.credentials.json` |
| Token 格式 | `sk-ant-oat01-...`（access）、`sk-ant-ort01-...`（refresh） |
| Token 有效期 | Access 8-12h，Refresh 单次使用 |
| API 端点 | `https://api.claude.ai/` |
| Device Code | ❌ 不支持（社区有 feature request） |

```json
{
  "claudeAiOauth": {
    "accessToken": "sk-ant-oat01-...",
    "refreshToken": "sk-ant-ort01-...",
    "expiresAt": 1748658860401,
    "scopes": ["user:inference", "user:profile"]
  }
}
```

**Claudex 集成方式**：Claudex 启动 Claude Code 时不覆盖认证环境变量，让 Claude Code 使用自己的 OAuth token 直接连接 Anthropic。对于 DirectAnthropic 的 subscription profile，proxy 只做 passthrough 不做翻译。

---

#### 2. OpenAI Codex CLI

| 项目 | 详情 |
|------|------|
| 订阅计划 | ChatGPT Plus $20/月、Pro $200/月、Business、Enterprise |
| 认证方式 | 浏览器 OAuth + Device Code (`codex login --device-auth`) |
| Token 存储 | `~/.codex/auth.json`（明文）或 OS credential store |
| Token 格式 | JWT（含 `chatgpt_account_id`、`organization_id`） |
| OAuth Client ID | `app_EMoamEEZ73f0CkXaXp7hrann`（公开） |
| Auth Endpoint | `https://auth.openai.com/oauth/authorize` |
| Token Endpoint | `https://auth.openai.com/oauth/token` |
| 第三方可用性 | ✅ Cline、OpenCode 等已官方集成 |

**Claudex 集成方式**：
- 方案 A：读取 `~/.codex/auth.json` 中已有 token
- 方案 B：自建 OAuth Flow（使用公开 Client ID）
- Proxy 读取 OAuth token → 作为 Bearer token 转发到 OpenAI API

---

#### 3. Google Gemini CLI

| 项目 | 详情 |
|------|------|
| 订阅计划 | 免费 60req/min、Google AI Pro、Google AI Ultra |
| 认证方式 | Google OAuth 浏览器登录 |
| Auth Type 支持 | `oauth`、`oauth-personal`、`api-key`、`vertex-ai` |
| API 端点 | `https://generativelanguage.googleapis.com/` |
| Token 格式 | Google OAuth2 标准 |
| 第三方可用性 | ✅ 开源 CLI（Apache 2.0） |

**Claudex 集成方式**：读取 Gemini CLI 的 OAuth 缓存 token，或自建 Google OAuth Flow。

---

#### 4. Kimi Code CLI（Moonshot）

| 项目 | 详情 |
|------|------|
| 订阅计划 | Adagio（免费）、Andante ¥49/月、更高档次 |
| 认证方式 | OAuth 浏览器登录（v1.1 新增） |
| API 端点 | `https://api.moonshot.cn/v1`（OpenAI 兼容） |
| Token 格式 | OAuth token |
| 第三方可用性 | ✅ 有 opencode 插件 |

**Claudex 集成方式**：读取 Kimi CLI 的 OAuth token 或自建 OAuth Flow。

---

#### 5. Qwen Code（阿里通义）

| 项目 | 详情 |
|------|------|
| 订阅计划 | 免费 2000 req/天（OAuth）、Coding Plan 月费套餐 |
| 认证方式 | Qwen OAuth 浏览器登录 |
| API 端点 | `https://chat.qwen.ai/`（OAuth）、DashScope API |
| Token 格式 | OAuth token |
| Device Code | ✅ 支持 Device Flow |
| 第三方可用性 | ✅ 有 opencode 插件 |

**Claudex 集成方式**：自建 Qwen OAuth Flow 或读取已有 token。

---

#### 6. GitHub Copilot CLI

| 项目 | 详情 |
|------|------|
| 订阅计划 | Individual $10/月、Business $19/用户/月、Enterprise $39/用户/月 |
| 认证方式 | OAuth Device Code Flow |
| 认证优先级 | `COPILOT_GITHUB_TOKEN → GITHUB_TOKEN → gh CLI → Device Flow` |
| 第三方可用性 | ✅ 通过 GitHub App |

**Claudex 集成方式**：通过 `gh` CLI 获取 token 或自建 Device Code Flow。

---

### 第二类：有订阅但 CLI 仅支持 API Key 的平台

#### 7. Grok / xAI

| 项目 | 详情 |
|------|------|
| 订阅计划 | X Premium+ $40/月、SuperGrok $30/月 |
| CLI 认证 | ❌ 仅 API Key（`xai-` 前缀） |
| API 端点 | `https://api.x.ai/v1`（OpenAI 兼容） |
| OAuth | 仅 X 账号登录 console.x.ai 获取 API Key |

**现状**：Grok 没有官方 CLI Agent，也没有 OAuth token 可以直接用于 API 调用。SuperGrok 订阅只能在 grok.com Web 界面使用，无法导出 token 用于 API。Claudex 继续使用 API Key 方式。

---

#### 8. DeepSeek

| 项目 | 详情 |
|------|------|
| 定价模式 | 纯 Pay-per-token，新账号赠送 500 万 token |
| CLI 认证 | ❌ 仅 API Key |
| API 端点 | `https://api.deepseek.com`（OpenAI 兼容） |

**现状**：DeepSeek 无订阅计划、无 OAuth。Claudex 继续使用 API Key。

---

### 第三类：IDE 内嵌 Agent（非 CLI）

#### 9. Windsurf（原 Codeium，现归 Cognition AI）

| 项目 | 详情 |
|------|------|
| 订阅计划 | Free 25 credits/月、Pro $15/月、Teams $30/月 |
| 认证方式 | IDE 内 Token 认证 |
| CLI 支持 | ❌ 无独立 CLI |

**现状**：IDE 内嵌，与 Claudex 使用场景不同。不纳入支持范围。

---

#### 10. Cursor

| 项目 | 详情 |
|------|------|
| 订阅计划 | Pro $20/月、Pro+ $60/月、Ultra $200/月 |
| 认证方式 | Cursor 账号登录 |
| CLI | ✅ 2026 年 1 月新推出 CLI + Agent 模式 |

**现状**：Cursor CLI 较新，认证机制尚未公开。观望。

---

#### 11. Google Antigravity

| 项目 | 详情 |
|------|------|
| 认证方式 | Google OAuth |
| 支持模型 | Gemini 3 Pro、Claude Opus 4.5 等 |
| 第三方可用性 | ⚠️ TOS 明确禁止第三方使用，已有封号案例 |

**现状**：风险太高，不建议支持。

---

#### 12. Augment Code（Auggie CLI）

| 项目 | 详情 |
|------|------|
| 认证方式 | OAuth 浏览器登录 |
| CLI | ✅ Auggie CLI |

**现状**：较新平台，文档不完善，观望。

---

## 实现优先级矩阵

按 **用户需求 × 技术可行性 × 安全性** 排序：

| 优先级 | 平台 | auth_type | 实现方式 | 难度 |
|--------|------|-----------|---------|------|
| P0 | Claude Code | `claude-oauth` | 读取 `~/.claude/.credentials.json` | 低 |
| P0 | OpenAI Codex | `codex-oauth` | 读取 `~/.codex/auth.json` 或自建 OAuth | 中 |
| P0 | Gemini CLI | `google-oauth` | 读取 Gemini 缓存或自建 Google OAuth | 中 |
| P1 | Qwen Code | `qwen-oauth` | 自建 OAuth Device Flow | 中 |
| P1 | Kimi Code | `kimi-oauth` | 读取 Kimi CLI token | 低 |
| P1 | Copilot CLI | `github-oauth` | 通过 gh CLI 获取 | 低 |
| P2 | Grok / xAI | 保持 `api-key` | 无订阅 OAuth 可用 | — |
| P2 | DeepSeek | 保持 `api-key` | 无订阅 OAuth 可用 | — |
| — | Antigravity | 不支持 | TOS 风险，已有封号案例 | — |
| — | Windsurf | 不支持 | IDE 内嵌，非 CLI 场景 | — |

---

## 关键架构决策

### 1. Claude Code OAuth 的特殊处理

Claudex 启动 Claude Code，Claude Code 自己做 OAuth 登录。关键问题：

- **当前行为**：Claudex 设置 `ANTHROPIC_BASE_URL` 指向代理，`ANTHROPIC_API_KEY=claudex-passthrough`
- **订阅模式**：不能设置这些环境变量，否则 Claude Code 不会使用 OAuth
- **解决方案**：subscription profile 不设置 `ANTHROPIC_BASE_URL` 和 `ANTHROPIC_API_KEY`，让 Claude Code 直连 Anthropic。Claudex 的翻译代理功能对这类 profile 不可用（本来也不需要翻译）。

### 2. 非 Claude 平台的 OAuth Token 作为 API Key 使用

对于 Codex、Gemini、Qwen 等平台：
- 读取/获取 OAuth token → 作为 Bearer token 设置到 profile 的 `api_key` 字段
- Proxy 正常翻译和转发，只是 token 来源从手动配置变为自动获取
- 需要后台 token 刷新机制

### 3. Token 竞争问题

- Claude OAuth refresh token 是**单次使用**的
- 如果 Claudex 和 Claude Code 同时刷新，会导致一方失效
- 解决：Claudex 只**读取** Claude Code 的 token，不自己刷新

---

## 现有开源参考

| 项目 | 语言 | 说明 |
|------|------|------|
| [CLIProxyAPI](https://github.com/router-for-me/CLIProxyAPI) | Go | 最完整的多平台 OAuth 代理 |
| [opencode-openai-codex-auth](https://github.com/numman-ali/opencode-openai-codex-auth) | JS | OpenAI Codex OAuth 插件 |
| [opencode-qwencode-auth](https://libraries.io/npm/opencode-kimi-code-auth) | JS | Qwen OAuth 插件 |
| [opencode-antigravity-auth](https://github.com/NoeFabris/opencode-antigravity-auth) | JS | Google OAuth 插件 |
| [grll/claude-code-login](https://github.com/grll/claude-code-login) | — | Claude Code OAuth 登录工具 |

---

## Sources

- [Claude Code Authentication](https://code.claude.com/docs/en/authentication)
- [Claude Max Pricing](https://claude.com/pricing/max)
- [OpenAI Codex Authentication](https://developers.openai.com/codex/auth/)
- [OpenAI Codex CLI Reference](https://developers.openai.com/codex/cli/reference/)
- [Gemini CLI Authentication](https://google-gemini.github.io/gemini-cli/docs/get-started/authentication.html)
- [Qwen Code Authentication](https://qwenlm.github.io/qwen-code-docs/en/users/configuration/auth/)
- [Kimi CLI GitHub](https://github.com/MoonshotAI/kimi-cli)
- [GitHub Copilot CLI](https://github.com/features/copilot/cli)
- [xAI Grok Pricing](https://docs.x.ai/developers/models)
- [SuperGrok Plans](https://grok.com/plans)
- [CLIProxyAPI](https://github.com/router-for-me/CLIProxyAPI)
- [Cline Codex OAuth](https://cline.bot/blog/introducing-openai-codex-oauth)
- [2026 CLI Coding Tools Comparison](https://www.tembo.io/blog/coding-cli-tools-comparison)
- [Alibaba Coding Plan](https://www.alibabacloud.com/help/en/model-studio/coding-plan)
