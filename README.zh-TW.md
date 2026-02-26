<p align="center">
  <h1 align="center">Claudex</h1>
  <p align="center">多實例 Claude Code 管理器，內建智慧翻譯代理</p>
</p>

<p align="center">
  <a href="https://github.com/StringKe/claudex/actions/workflows/ci.yml"><img src="https://github.com/StringKe/claudex/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
  <a href="https://github.com/StringKe/claudex/releases"><img src="https://github.com/StringKe/claudex/actions/workflows/release.yml/badge.svg" alt="Release"></a>
  <a href="https://github.com/StringKe/claudex/blob/main/LICENSE"><img src="https://img.shields.io/github/license/StringKe/claudex" alt="License"></a>
  <a href="https://github.com/StringKe/claudex/releases"><img src="https://img.shields.io/github/v/release/StringKe/claudex" alt="Latest Release"></a>
</p>

<p align="center">
  <a href="https://stringke.github.io/claudex/zh-TW/">說明文件</a>
</p>

<p align="center">
  <a href="./README.md">English</a> |
  <a href="./README.zh-CN.md">简体中文</a> |
  繁體中文 |
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

Claudex 是一個統一的代理層，讓 [Claude Code](https://docs.anthropic.com/en/docs/claude-code) 透過自動協定翻譯，無縫對接多種 AI 提供商。

## 功能特色

- **多提供商代理** — DirectAnthropic 直通 + Anthropic <-> OpenAI Chat Completions 翻譯 + Anthropic <-> Responses API 翻譯
- **20+ 提供商** — Anthropic、OpenRouter、Grok、OpenAI、DeepSeek、Kimi、GLM、Groq、Mistral、Together AI、Perplexity、Cerebras、Azure OpenAI、Google Vertex AI、Ollama、LM Studio 等
- **串流翻譯** — 完整 SSE 串流翻譯，支援 tool call
- **斷路器 + 容錯移轉** — 可設定閾值，自動切換至備用提供商
- **智慧路由** — 透過本地分類器進行意圖感知的自動路由
- **上下文引擎** — 對話壓縮、跨 profile 共享、本地 RAG 向量檢索
- **OAuth 訂閱** — ChatGPT/Codex、Claude Max、GitHub Copilot、GitLab Duo、Google Gemini、Qwen、Kimi
- **設定集** — 從 git 倉庫安裝並管理可重用的 Claude Code 設定集
- **TUI 儀表板** — 即時 profile 健康狀態、指標、日誌與快速啟動
- **自動更新** — `claudex update` 從 GitHub 下載最新版本

## 安裝

```bash
# 一鍵安裝（Linux / macOS）
curl -fsSL https://raw.githubusercontent.com/StringKe/claudex/main/install.sh | bash

# 從原始碼編譯
cargo install --git https://github.com/StringKe/claudex

# 或從 GitHub Releases 下載預編譯二進位檔
# https://github.com/StringKe/claudex/releases
```

### 系統需求

- macOS（Intel / Apple Silicon）或 Linux（x86_64 / ARM64）
- 已安裝 [Claude Code](https://docs.anthropic.com/en/docs/claude-code)
- Windows：請從 [Releases](https://github.com/StringKe/claudex/releases) 下載預編譯二進位檔

## 快速開始

```bash
# 1. 初始化設定
claudex config init

# 2. 以互動方式新增提供商 profile
claudex profile add

# 3. 測試連線
claudex profile test all

# 4. 以指定提供商執行 Claude Code
claudex run grok

# 5. 或使用智慧路由自動選擇最佳提供商
claudex run auto
```

## 運作原理

```
claudex run openrouter-claude
    │
    ├── 啟動代理（若尚未執行）→ 127.0.0.1:13456
    │
    └── 以環境變數執行 claude：
        ANTHROPIC_BASE_URL=http://127.0.0.1:13456/proxy/openrouter-claude
        ANTHROPIC_AUTH_TOKEN=claudex-passthrough
        ANTHROPIC_MODEL=anthropic/claude-sonnet-4
        ANTHROPIC_DEFAULT_HAIKU_MODEL=...
        ANTHROPIC_DEFAULT_SONNET_MODEL=...
        ANTHROPIC_DEFAULT_OPUS_MODEL=...
```

代理攔截請求並處理協定翻譯：

- **DirectAnthropic**（Anthropic、MiniMax、Vertex AI）→ 加入正確標頭後直接轉發
- **OpenAICompatible**（Grok、OpenAI、DeepSeek 等）→ Anthropic → OpenAI Chat Completions → 翻譯回應
- **OpenAIResponses**（ChatGPT/Codex 訂閱）→ Anthropic → Responses API → 翻譯回應

## 提供商相容性

| 提供商 | 類型 | 翻譯 | 認證方式 | 範例模型 |
|--------|------|------|---------|---------|
| Anthropic | DirectAnthropic | 無 | API Key | `claude-sonnet-4-20250514` |
| MiniMax | DirectAnthropic | 無 | API Key | `claude-sonnet-4-20250514` |
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
| Google Vertex AI | DirectAnthropic | 無 | Bearer (gcloud) | `claude-sonnet-4@...` |
| Ollama | OpenAICompatible | Anthropic <-> OpenAI | 無 | `qwen2.5:72b` |
| LM Studio | OpenAICompatible | Anthropic <-> OpenAI | 無 | local model |
| ChatGPT/Codex 訂閱 | OpenAIResponses | Anthropic <-> Responses | OAuth (PKCE/Device) | `gpt-5.3-codex` |
| Claude Max 訂閱 | DirectAnthropic | 無 | OAuth (file) | `claude-sonnet-4` |
| GitHub Copilot | OpenAICompatible | Anthropic <-> OpenAI | OAuth (Device+Bearer) | `gpt-4o` |
| GitLab Duo | OpenAICompatible | Anthropic <-> OpenAI | GITLAB_TOKEN | `claude-sonnet-4` |

## 設定

Claudex 依下列順序搜尋設定檔：

1. `$CLAUDEX_CONFIG` 環境變數
2. `./claudex.toml` 或 `./claudex.yaml`（當前目錄）
3. `./.claudex/config.toml`
4. 向上層目錄搜尋（最多 10 層）
5. `~/.config/claudex/config.toml`（全域設定，建議使用）

支援 TOML 與 YAML 格式。完整設定參考請見 [`config.example.toml`](./config.example.toml)。

## CLI 指令參考

| 指令 | 說明 |
|------|------|
| `claudex run <profile>` | 以指定提供商執行 Claude Code |
| `claudex run auto` | 智慧路由，自動選擇最佳提供商 |
| `claudex run <profile> -m <model>` | 覆寫本次工作階段使用的模型 |
| `claudex profile list` | 列出所有已設定的 profile |
| `claudex profile add` | 互動式 profile 設定精靈 |
| `claudex profile show <name>` | 顯示 profile 詳細資訊 |
| `claudex profile remove <name>` | 移除 profile |
| `claudex profile test <name\|all>` | 測試提供商連線 |
| `claudex proxy start [-p port] [-d]` | 啟動代理（可選擇以常駐程式模式執行） |
| `claudex proxy stop` | 停止代理常駐程式 |
| `claudex proxy status` | 顯示代理狀態 |
| `claudex dashboard` | 啟動 TUI 儀表板 |
| `claudex config show [--raw] [--json]` | 顯示已載入的設定 |
| `claudex config init [--yaml]` | 在當前目錄建立設定檔 |
| `claudex config edit [--global]` | 以 $EDITOR 開啟設定檔 |
| `claudex config validate [--connectivity]` | 驗證設定 |
| `claudex config get <key>` | 取得設定值 |
| `claudex config set <key> <value>` | 設定設定值 |
| `claudex config export --format <fmt>` | 匯出設定（json/toml/yaml） |
| `claudex update [--check]` | 從 GitHub Releases 自動更新 |
| `claudex auth login <provider>` | OAuth 登入 |
| `claudex auth login github --enterprise-url <domain>` | GitHub Enterprise Copilot |
| `claudex auth status` | 顯示 OAuth token 狀態 |
| `claudex auth logout <profile>` | 移除 OAuth token |
| `claudex auth refresh <profile>` | 強制刷新 OAuth token |
| `claudex sets add <source> [--global]` | 安裝設定集 |
| `claudex sets remove <name>` | 移除設定集 |
| `claudex sets list [--global]` | 列出已安裝的設定集 |
| `claudex sets update [name]` | 更新設定集至最新版本 |

## OAuth 訂閱

使用現有訂閱帳號而非 API Key：

```bash
# ChatGPT 訂閱（自動偵測現有 Codex CLI 憑證）
claudex auth login chatgpt --profile codex-sub

# ChatGPT 強制瀏覽器登入
claudex auth login chatgpt --profile codex-sub --force

# ChatGPT 無介面模式（SSH / 無瀏覽器環境）
claudex auth login chatgpt --profile codex-sub --force --headless

# GitHub Copilot
claudex auth login github --profile copilot

# GitHub Copilot Enterprise
claudex auth login github --profile copilot-ent --enterprise-url company.ghe.com

# GitLab Duo（讀取 GITLAB_TOKEN 環境變數）
claudex auth login gitlab --profile gitlab-duo

# 查看狀態
claudex auth status

# 以訂閱帳號執行
claudex run codex-sub
```

支援的提供商：`claude`、`chatgpt`/`openai`、`google`、`qwen`、`kimi`、`github`/`copilot`、`gitlab`

## 模型槽位對應

將 Claude Code 的 `/model` 切換器（haiku/sonnet/opus）對應到任意提供商的模型：

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

## 架構

```
src/
├── main.rs
├── cli.rs
├── update.rs
├── util.rs
├── config/
│   ├── mod.rs          # 設定探索 + 解析（figment）
│   ├── cmd.rs          # config get/set/export/validate 子指令
│   └── profile.rs      # Profile CRUD + 連線測試
├── process/
│   ├── mod.rs
│   ├── launch.rs       # Claude 程序啟動器
│   └── daemon.rs       # PID 檔案 + 程序管理
├── oauth/
│   ├── mod.rs          # AuthType、OAuthProvider、OAuthToken
│   ├── source.rs       # 第一層：憑證來源（env/file/keyring）
│   ├── exchange.rs     # 第二層：token 交換（PKCE/device code/refresh）
│   ├── manager.rs      # 第三層：快取 + 並發去重 + 401 重試
│   ├── handler.rs      # OAuthProviderHandler trait
│   ├── providers.rs    # 登入/刷新/狀態 CLI 邏輯
│   ├── server.rs       # OAuth 回呼伺服器 + device code 輪詢
│   └── token.rs        # Re-exports
├── proxy/
│   ├── mod.rs          # Axum 伺服器 + ProxyState
│   ├── handler.rs      # 請求路由 + 斷路器 + 401 重試
│   ├── adapter/        # 提供商專用介面卡
│   │   ├── mod.rs      # ProviderAdapter trait + factory
│   │   ├── direct.rs   # DirectAnthropic（直通）
│   │   ├── chat_completions.rs  # OpenAI Chat Completions
│   │   └── responses.rs         # OpenAI Responses API
│   ├── translate/      # 協定翻譯
│   │   ├── chat_completions.rs
│   │   ├── chat_completions_stream.rs
│   │   ├── responses.rs
│   │   └── responses_stream.rs
│   ├── context_engine.rs
│   ├── fallback.rs     # 斷路器
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
├── sets/               # 設定集管理
│   ├── mod.rs
│   ├── schema.rs
│   ├── source.rs
│   ├── install.rs
│   ├── lock.rs
│   ├── conflict.rs
│   └── mcp.rs
├── terminal/           # 終端機偵測 + 超連結
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

## 授權條款

[MIT](./LICENSE)
