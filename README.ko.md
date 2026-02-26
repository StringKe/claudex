<p align="center">
  <h1 align="center">Claudex</h1>
  <p align="center">지능형 번역 프록시를 내장한 멀티 인스턴스 Claude Code 매니저</p>
</p>

<p align="center">
  <a href="https://github.com/StringKe/claudex/actions/workflows/ci.yml"><img src="https://github.com/StringKe/claudex/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
  <a href="https://github.com/StringKe/claudex/releases"><img src="https://github.com/StringKe/claudex/actions/workflows/release.yml/badge.svg" alt="Release"></a>
  <a href="https://github.com/StringKe/claudex/blob/main/LICENSE"><img src="https://img.shields.io/github/license/StringKe/claudex" alt="License"></a>
  <a href="https://github.com/StringKe/claudex/releases"><img src="https://img.shields.io/github/v/release/StringKe/claudex" alt="Latest Release"></a>
</p>

<p align="center">
  <a href="https://stringke.github.io/claudex/ko/">문서</a>
</p>

<p align="center">
  <a href="./README.md">English</a> |
  <a href="./README.zh-CN.md">简体中文</a> |
  <a href="./README.zh-TW.md">繁體中文</a> |
  <a href="./README.ja.md">日本語</a> |
  한국어 |
  <a href="./README.ru.md">Русский</a> |
  <a href="./README.fr.md">Français</a> |
  <a href="./README.pt-BR.md">Português do Brasil</a> |
  <a href="./README.es.md">Español</a> |
  <a href="./README.it.md">Italiano</a> |
  <a href="./README.de.md">Deutsch</a> |
  <a href="./README.pl.md">Polski</a>
</p>

---

Claudex는 [Claude Code](https://docs.anthropic.com/en/docs/claude-code)가 자동 프로토콜 번역을 통해 여러 AI 제공자와 원활하게 연동할 수 있도록 해주는 통합 프록시입니다.

## 기능

- **멀티 제공자 프록시** — DirectAnthropic 직접 전달 + Anthropic <-> OpenAI Chat Completions 번역 + Anthropic <-> Responses API 번역
- **20개 이상의 제공자** — Anthropic, OpenRouter, Grok, OpenAI, DeepSeek, Kimi, GLM, Groq, Mistral, Together AI, Perplexity, Cerebras, Azure OpenAI, Google Vertex AI, Ollama, LM Studio 등
- **스트리밍 번역** — 툴 호출을 지원하는 완전한 SSE 스트림 번역
- **서킷 브레이커 + 장애 조치** — 설정 가능한 임계값으로 백업 제공자에 자동 전환
- **스마트 라우팅** — 로컬 분류기를 통한 의도 기반 자동 라우팅
- **컨텍스트 엔진** — 대화 압축, 프로필 간 공유, 임베딩 기반 로컬 RAG
- **OAuth 구독** — ChatGPT/Codex, Claude Max, GitHub Copilot, GitLab Duo, Google Gemini, Qwen, Kimi
- **구성 세트** — git 저장소에서 재사용 가능한 Claude Code 구성 세트 설치 및 관리
- **TUI 대시보드** — 실시간 프로필 상태, 메트릭, 로그, 빠른 실행
- **자동 업데이트** — `claudex update`로 GitHub에서 최신 릴리즈 다운로드

## 설치

```bash
# 원라이너 (Linux / macOS)
curl -fsSL https://raw.githubusercontent.com/StringKe/claudex/main/install.sh | bash

# 소스에서 빌드
cargo install --git https://github.com/StringKe/claudex

# 또는 GitHub Releases에서 다운로드
# https://github.com/StringKe/claudex/releases
```

### 시스템 요구사항

- macOS (Intel / Apple Silicon) 또는 Linux (x86_64 / ARM64)
- [Claude Code](https://docs.anthropic.com/en/docs/claude-code) 설치 필요
- Windows: [Releases](https://github.com/StringKe/claudex/releases)에서 사전 빌드된 바이너리 다운로드

## 빠른 시작

```bash
# 1. 설정 초기화
claudex config init

# 2. 제공자 프로필 대화식으로 추가
claudex profile add

# 3. 연결 테스트
claudex profile test all

# 4. 특정 제공자로 Claude Code 실행
claudex run grok

# 5. 또는 스마트 라우팅으로 최적 제공자 자동 선택
claudex run auto
```

## 작동 방식

```
claudex run openrouter-claude
    │
    ├── 프록시 시작 (실행 중이 아닌 경우) → 127.0.0.1:13456
    │
    └── 환경 변수와 함께 claude 실행:
        ANTHROPIC_BASE_URL=http://127.0.0.1:13456/proxy/openrouter-claude
        ANTHROPIC_AUTH_TOKEN=claudex-passthrough
        ANTHROPIC_MODEL=anthropic/claude-sonnet-4
        ANTHROPIC_DEFAULT_HAIKU_MODEL=...
        ANTHROPIC_DEFAULT_SONNET_MODEL=...
        ANTHROPIC_DEFAULT_OPUS_MODEL=...
```

프록시는 요청을 가로채어 프로토콜 번역을 처리합니다:

- **DirectAnthropic** (Anthropic, MiniMax, Vertex AI) → 올바른 헤더로 직접 전달
- **OpenAICompatible** (Grok, OpenAI, DeepSeek 등) → Anthropic → OpenAI Chat Completions → 응답 역번역
- **OpenAIResponses** (ChatGPT/Codex 구독) → Anthropic → Responses API → 응답 역번역

## 제공자 호환성

| 제공자 | 타입 | 번역 | 인증 | 예시 모델 |
|--------|------|------|------|-----------|
| Anthropic | DirectAnthropic | 없음 | API Key | `claude-sonnet-4-20250514` |
| MiniMax | DirectAnthropic | 없음 | API Key | `claude-sonnet-4-20250514` |
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
| Google Vertex AI | DirectAnthropic | 없음 | Bearer (gcloud) | `claude-sonnet-4@...` |
| Ollama | OpenAICompatible | Anthropic <-> OpenAI | 없음 | `qwen2.5:72b` |
| LM Studio | OpenAICompatible | Anthropic <-> OpenAI | 없음 | local model |
| ChatGPT/Codex 구독 | OpenAIResponses | Anthropic <-> Responses | OAuth (PKCE/Device) | `gpt-5.3-codex` |
| Claude Max 구독 | DirectAnthropic | 없음 | OAuth (file) | `claude-sonnet-4` |
| GitHub Copilot | OpenAICompatible | Anthropic <-> OpenAI | OAuth (Device+Bearer) | `gpt-4o` |
| GitLab Duo | OpenAICompatible | Anthropic <-> OpenAI | GITLAB_TOKEN | `claude-sonnet-4` |

## 구성

Claudex는 다음 순서로 구성 파일을 탐색합니다:

1. `$CLAUDEX_CONFIG` 환경 변수
2. `./claudex.toml` 또는 `./claudex.yaml` (현재 디렉토리)
3. `./.claudex/config.toml`
4. 상위 디렉토리 (최대 10단계)
5. `~/.config/claudex/config.toml` (전역, 권장)

TOML 및 YAML 형식을 지원합니다. 전체 참조는 [`config.example.toml`](./config.example.toml)을 확인하세요.

## CLI 참조

| 명령 | 설명 |
|------|------|
| `claudex run <profile>` | 특정 제공자로 Claude Code 실행 |
| `claudex run auto` | 스마트 라우팅 — 최적 제공자 자동 선택 |
| `claudex run <profile> -m <model>` | 세션에서 모델 재정의 |
| `claudex profile list` | 설정된 모든 프로필 목록 표시 |
| `claudex profile add` | 대화식 프로필 설정 마법사 |
| `claudex profile show <name>` | 프로필 상세 정보 표시 |
| `claudex profile remove <name>` | 프로필 삭제 |
| `claudex profile test <name\|all>` | 제공자 연결 테스트 |
| `claudex proxy start [-p port] [-d]` | 프록시 시작 (선택적으로 데몬으로) |
| `claudex proxy stop` | 프록시 데몬 중지 |
| `claudex proxy status` | 프록시 상태 표시 |
| `claudex dashboard` | TUI 대시보드 실행 |
| `claudex config show [--raw] [--json]` | 로드된 구성 표시 |
| `claudex config init [--yaml]` | 현재 디렉토리에 구성 파일 생성 |
| `claudex config edit [--global]` | $EDITOR로 구성 파일 열기 |
| `claudex config validate [--connectivity]` | 구성 유효성 검사 |
| `claudex config get <key>` | 구성 값 조회 |
| `claudex config set <key> <value>` | 구성 값 설정 |
| `claudex config export --format <fmt>` | 구성 내보내기 (json/toml/yaml) |
| `claudex update [--check]` | GitHub Releases에서 자동 업데이트 |
| `claudex auth login <provider>` | OAuth 로그인 |
| `claudex auth login github --enterprise-url <domain>` | GitHub Enterprise Copilot |
| `claudex auth status` | OAuth 토큰 상태 표시 |
| `claudex auth logout <profile>` | OAuth 토큰 삭제 |
| `claudex auth refresh <profile>` | OAuth 토큰 강제 갱신 |
| `claudex sets add <source> [--global]` | 구성 세트 설치 |
| `claudex sets remove <name>` | 구성 세트 삭제 |
| `claudex sets list [--global]` | 설치된 세트 목록 표시 |
| `claudex sets update [name]` | 세트를 최신 버전으로 업데이트 |

## OAuth 구독

API 키 대신 기존 구독을 사용합니다:

```bash
# ChatGPT 구독 (기존 Codex CLI 자격증명 자동 감지)
claudex auth login chatgpt --profile codex-sub

# ChatGPT 브라우저 강제 로그인
claudex auth login chatgpt --profile codex-sub --force

# ChatGPT 헤드리스 (SSH/브라우저 없는 환경)
claudex auth login chatgpt --profile codex-sub --force --headless

# GitHub Copilot
claudex auth login github --profile copilot

# GitHub Copilot Enterprise
claudex auth login github --profile copilot-ent --enterprise-url company.ghe.com

# GitLab Duo (GITLAB_TOKEN 환경 변수 읽기)
claudex auth login gitlab --profile gitlab-duo

# 상태 확인
claudex auth status

# 구독으로 실행
claudex run codex-sub
```

지원 대상: `claude`, `chatgpt`/`openai`, `google`, `qwen`, `kimi`, `github`/`copilot`, `gitlab`

## 모델 슬롯 매핑

Claude Code의 `/model` 전환기 (haiku/sonnet/opus)를 임의 제공자 모델에 매핑합니다:

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

## 아키텍처

```
src/
├── main.rs
├── cli.rs
├── update.rs
├── util.rs
├── config/
│   ├── mod.rs          # 구성 탐색 + 파싱 (figment)
│   ├── cmd.rs          # config get/set/export/validate 서브커맨드
│   └── profile.rs      # Profile CRUD + 연결 테스트
├── process/
│   ├── mod.rs
│   ├── launch.rs       # Claude 프로세스 런처
│   └── daemon.rs       # PID 파일 + 프로세스 관리
├── oauth/
│   ├── mod.rs          # AuthType, OAuthProvider, OAuthToken
│   ├── source.rs       # 레이어 1: 자격증명 소스 (env/file/keyring)
│   ├── exchange.rs     # 레이어 2: 토큰 교환 (PKCE/device code/refresh)
│   ├── manager.rs      # 레이어 3: 캐시 + 동시성 중복 제거 + 401 재시도
│   ├── handler.rs      # OAuthProviderHandler 트레이트
│   ├── providers.rs    # Login/refresh/status CLI 로직
│   ├── server.rs       # OAuth 콜백 서버 + device code 폴링
│   └── token.rs        # 재내보내기
├── proxy/
│   ├── mod.rs          # Axum 서버 + ProxyState
│   ├── handler.rs      # 요청 라우팅 + 서킷 브레이커 + 401 재시도
│   ├── adapter/        # 제공자별 어댑터
│   │   ├── mod.rs      # ProviderAdapter 트레이트 + 팩토리
│   │   ├── direct.rs   # DirectAnthropic (직접 전달)
│   │   ├── chat_completions.rs  # OpenAI Chat Completions
│   │   └── responses.rs         # OpenAI Responses API
│   ├── translate/      # 프로토콜 번역
│   │   ├── chat_completions.rs
│   │   ├── chat_completions_stream.rs
│   │   ├── responses.rs
│   │   └── responses_stream.rs
│   ├── context_engine.rs
│   ├── fallback.rs     # 서킷 브레이커
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
├── sets/               # 구성 세트 관리
│   ├── mod.rs
│   ├── schema.rs
│   ├── source.rs
│   ├── install.rs
│   ├── lock.rs
│   ├── conflict.rs
│   └── mcp.rs
├── terminal/           # 터미널 감지 + 하이퍼링크
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

## 라이선스

[MIT](./LICENSE)
