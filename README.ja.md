<p align="center">
  <h1 align="center">Claudex</h1>
  <p align="center">マルチインスタンス Claude Code マネージャー（インテリジェント翻訳プロキシ内蔵）</p>
</p>

<p align="center">
  <a href="https://github.com/StringKe/claudex/actions/workflows/ci.yml"><img src="https://github.com/StringKe/claudex/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
  <a href="https://github.com/StringKe/claudex/releases"><img src="https://github.com/StringKe/claudex/actions/workflows/release.yml/badge.svg" alt="Release"></a>
  <a href="https://github.com/StringKe/claudex/blob/main/LICENSE"><img src="https://img.shields.io/github/license/StringKe/claudex" alt="License"></a>
  <a href="https://github.com/StringKe/claudex/releases"><img src="https://img.shields.io/github/v/release/StringKe/claudex" alt="Latest Release"></a>
</p>

<p align="center">
  <a href="https://stringke.github.io/claudex/ja/">ドキュメント</a>
</p>

<p align="center">
  <a href="./README.md">English</a> |
  <a href="./README.zh-CN.md">简体中文</a> |
  <a href="./README.zh-TW.md">繁體中文</a> |
  日本語 |
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

Claudex は、[Claude Code](https://docs.anthropic.com/en/docs/claude-code) が自動プロトコル変換を通じて複数の AI プロバイダーとシームレスに連携できる統合プロキシです。

## 機能

- **マルチプロバイダープロキシ** — DirectAnthropic パススルー + Anthropic <-> OpenAI Chat Completions 翻訳 + Anthropic <-> Responses API 翻訳
- **20 以上のプロバイダー対応** — Anthropic、OpenRouter、Grok、OpenAI、DeepSeek、Kimi、GLM、Groq、Mistral、Together AI、Perplexity、Cerebras、Azure OpenAI、Google Vertex AI、Ollama、LM Studio など
- **ストリーミング翻訳** — ツールコール対応の完全な SSE ストリーム翻訳
- **サーキットブレーカー + フェイルオーバー** — 設定可能なしきい値によるバックアッププロバイダーへの自動フォールバック
- **スマートルーティング** — ローカル分類器によるインテントベースの自動プロバイダー選択
- **コンテキストエンジン** — 会話の圧縮、プロファイル間共有、埋め込みを使ったローカル RAG
- **OAuth サブスクリプション** — ChatGPT/Codex、Claude Max、GitHub Copilot、GitLab Duo、Google Gemini、Qwen、Kimi
- **設定セット** — git リポジトリから再利用可能な Claude Code 設定セットをインストール・管理
- **TUI ダッシュボード** — プロファイルの健全性、メトリクス、ログ、クイック起動をリアルタイム表示
- **セルフアップデート** — `claudex update` で GitHub から最新リリースをダウンロード

## インストール

```bash
# ワンライナー（Linux / macOS）
curl -fsSL https://raw.githubusercontent.com/StringKe/claudex/main/install.sh | bash

# ソースからビルド
cargo install --git https://github.com/StringKe/claudex

# または GitHub Releases からダウンロード
# https://github.com/StringKe/claudex/releases
```

### システム要件

- macOS（Intel / Apple Silicon）または Linux（x86_64 / ARM64）
- [Claude Code](https://docs.anthropic.com/en/docs/claude-code) がインストール済みであること
- Windows: [Releases](https://github.com/StringKe/claudex/releases) からビルド済みバイナリをダウンロード

## クイックスタート

```bash
# 1. 設定を初期化
claudex config init

# 2. プロバイダープロファイルを対話形式で追加
claudex profile add

# 3. 接続テスト
claudex profile test all

# 4. 特定のプロバイダーで Claude Code を起動
claudex run grok

# 5. または、スマートルーティングで最適なプロバイダーを自動選択
claudex run auto
```

## 仕組み

```
claudex run openrouter-claude
    │
    ├── プロキシを起動（未起動の場合）→ 127.0.0.1:13456
    │
    └── 以下の環境変数を設定して claude を実行:
        ANTHROPIC_BASE_URL=http://127.0.0.1:13456/proxy/openrouter-claude
        ANTHROPIC_AUTH_TOKEN=claudex-passthrough
        ANTHROPIC_MODEL=anthropic/claude-sonnet-4
        ANTHROPIC_DEFAULT_HAIKU_MODEL=...
        ANTHROPIC_DEFAULT_SONNET_MODEL=...
        ANTHROPIC_DEFAULT_OPUS_MODEL=...
```

プロキシはリクエストをインターセプトし、プロトコル変換を処理します:

- **DirectAnthropic**（Anthropic、MiniMax、Vertex AI）→ 正しいヘッダーで転送
- **OpenAICompatible**（Grok、OpenAI、DeepSeek など）→ Anthropic → OpenAI Chat Completions → レスポンスを逆変換
- **OpenAIResponses**（ChatGPT/Codex サブスクリプション）→ Anthropic → Responses API → レスポンスを逆変換

## プロバイダー互換性

| プロバイダー | タイプ | 翻訳 | 認証 | モデル例 |
|-------------|--------|------|------|---------|
| Anthropic | DirectAnthropic | なし | API Key | `claude-sonnet-4-20250514` |
| MiniMax | DirectAnthropic | なし | API Key | `claude-sonnet-4-20250514` |
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
| Google Vertex AI | DirectAnthropic | なし | Bearer (gcloud) | `claude-sonnet-4@...` |
| Ollama | OpenAICompatible | Anthropic <-> OpenAI | なし | `qwen2.5:72b` |
| LM Studio | OpenAICompatible | Anthropic <-> OpenAI | なし | ローカルモデル |
| ChatGPT/Codex サブ | OpenAIResponses | Anthropic <-> Responses | OAuth (PKCE/Device) | `gpt-5.3-codex` |
| Claude Max サブ | DirectAnthropic | なし | OAuth (file) | `claude-sonnet-4` |
| GitHub Copilot | OpenAICompatible | Anthropic <-> OpenAI | OAuth (Device+Bearer) | `gpt-4o` |
| GitLab Duo | OpenAICompatible | Anthropic <-> OpenAI | GITLAB_TOKEN | `claude-sonnet-4` |

## 設定

Claudex は以下の順序で設定ファイルを検索します:

1. `$CLAUDEX_CONFIG` 環境変数
2. `./claudex.toml` または `./claudex.yaml`（カレントディレクトリ）
3. `./.claudex/config.toml`
4. 親ディレクトリ（最大 10 階層）
5. `~/.config/claudex/config.toml`（グローバル設定、推奨）

TOML と YAML 形式に対応しています。完全なリファレンスは [`config.example.toml`](./config.example.toml) を参照してください。

## CLI リファレンス

| コマンド | 説明 |
|---------|------|
| `claudex run <profile>` | 特定のプロバイダーで Claude Code を起動 |
| `claudex run auto` | スマートルーティング — 最適なプロバイダーを自動選択 |
| `claudex run <profile> -m <model>` | セッションのモデルを上書き |
| `claudex profile list` | 設定済みのプロファイルを一覧表示 |
| `claudex profile add` | 対話形式のプロファイル設定ウィザード |
| `claudex profile show <name>` | プロファイルの詳細を表示 |
| `claudex profile remove <name>` | プロファイルを削除 |
| `claudex profile test <name\|all>` | プロバイダーの接続テスト |
| `claudex proxy start [-p port] [-d]` | プロキシを起動（オプションでデーモンとして起動） |
| `claudex proxy stop` | プロキシデーモンを停止 |
| `claudex proxy status` | プロキシの状態を表示 |
| `claudex dashboard` | TUI ダッシュボードを起動 |
| `claudex config show [--raw] [--json]` | 読み込まれた設定を表示 |
| `claudex config init [--yaml]` | カレントディレクトリに設定を作成 |
| `claudex config edit [--global]` | $EDITOR で設定を開く |
| `claudex config validate [--connectivity]` | 設定を検証 |
| `claudex config get <key>` | 設定値を取得 |
| `claudex config set <key> <value>` | 設定値を変更 |
| `claudex config export --format <fmt>` | 設定をエクスポート（json/toml/yaml） |
| `claudex update [--check]` | GitHub Releases からセルフアップデート |
| `claudex auth login <provider>` | OAuth ログイン |
| `claudex auth login github --enterprise-url <domain>` | GitHub Enterprise Copilot |
| `claudex auth status` | OAuth トークンの状態を表示 |
| `claudex auth logout <profile>` | OAuth トークンを削除 |
| `claudex auth refresh <profile>` | OAuth トークンを強制更新 |
| `claudex sets add <source> [--global]` | 設定セットをインストール |
| `claudex sets remove <name>` | 設定セットを削除 |
| `claudex sets list [--global]` | インストール済みの設定セットを一覧表示 |
| `claudex sets update [name]` | 設定セットを最新版に更新 |

## OAuth サブスクリプション

API キーの代わりに既存のサブスクリプションを使用できます:

```bash
# ChatGPT サブスクリプション（既存の Codex CLI 認証情報を自動検出）
claudex auth login chatgpt --profile codex-sub

# ChatGPT ブラウザログインを強制
claudex auth login chatgpt --profile codex-sub --force

# ChatGPT ヘッドレス（SSH / ブラウザなし環境）
claudex auth login chatgpt --profile codex-sub --force --headless

# GitHub Copilot
claudex auth login github --profile copilot

# GitHub Copilot Enterprise
claudex auth login github --profile copilot-ent --enterprise-url company.ghe.com

# GitLab Duo（GITLAB_TOKEN 環境変数を読み取り）
claudex auth login gitlab --profile gitlab-duo

# 状態確認
claudex auth status

# サブスクリプションで起動
claudex run codex-sub
```

対応プロバイダー: `claude`、`chatgpt`/`openai`、`google`、`qwen`、`kimi`、`github`/`copilot`、`gitlab`

## モデルスロットマッピング

Claude Code の `/model` スイッチャー（haiku/sonnet/opus）を任意のプロバイダーのモデルにマッピングできます:

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

## アーキテクチャ

```
src/
├── main.rs
├── cli.rs
├── update.rs
├── util.rs
├── config/
│   ├── mod.rs          # 設定の検索・解析（figment）
│   ├── cmd.rs          # config get/set/export/validate サブコマンド
│   └── profile.rs      # プロファイル CRUD + 接続テスト
├── process/
│   ├── mod.rs
│   ├── launch.rs       # Claude プロセスランチャー
│   └── daemon.rs       # PID ファイル + プロセス管理
├── oauth/
│   ├── mod.rs          # AuthType、OAuthProvider、OAuthToken
│   ├── source.rs       # レイヤー 1: 認証情報ソース（env/file/keyring）
│   ├── exchange.rs     # レイヤー 2: トークン交換（PKCE/device code/refresh）
│   ├── manager.rs      # レイヤー 3: キャッシュ + 並行重複排除 + 401 リトライ
│   ├── handler.rs      # OAuthProviderHandler トレイト
│   ├── providers.rs    # ログイン/更新/状態確認の CLI ロジック
│   ├── server.rs       # OAuth コールバックサーバー + device code ポーリング
│   └── token.rs        # 再エクスポート
├── proxy/
│   ├── mod.rs          # Axum サーバー + ProxyState
│   ├── handler.rs      # リクエストルーティング + サーキットブレーカー + 401 リトライ
│   ├── adapter/        # プロバイダー固有のアダプター
│   │   ├── mod.rs      # ProviderAdapter トレイト + ファクトリ
│   │   ├── direct.rs   # DirectAnthropic（パススルー）
│   │   ├── chat_completions.rs  # OpenAI Chat Completions
│   │   └── responses.rs         # OpenAI Responses API
│   ├── translate/      # プロトコル変換
│   │   ├── chat_completions.rs
│   │   ├── chat_completions_stream.rs
│   │   ├── responses.rs
│   │   └── responses_stream.rs
│   ├── context_engine.rs
│   ├── fallback.rs     # サーキットブレーカー
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
├── sets/               # 設定セット管理
│   ├── mod.rs
│   ├── schema.rs
│   ├── source.rs
│   ├── install.rs
│   ├── lock.rs
│   ├── conflict.rs
│   └── mcp.rs
├── terminal/           # ターミナル検出 + ハイパーリンク
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

## ライセンス

[MIT](./LICENSE)
