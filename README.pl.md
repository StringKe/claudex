<p align="center">
  <h1 align="center">Claudex</h1>
  <p align="center">Wieloinstancyjny menedżer Claude Code z inteligentnym proxy tłumaczącym</p>
</p>

<p align="center">
  <a href="https://github.com/StringKe/claudex/actions/workflows/ci.yml"><img src="https://github.com/StringKe/claudex/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
  <a href="https://github.com/StringKe/claudex/releases"><img src="https://github.com/StringKe/claudex/actions/workflows/release.yml/badge.svg" alt="Release"></a>
  <a href="https://github.com/StringKe/claudex/blob/main/LICENSE"><img src="https://img.shields.io/github/license/StringKe/claudex" alt="License"></a>
  <a href="https://github.com/StringKe/claudex/releases"><img src="https://img.shields.io/github/v/release/StringKe/claudex" alt="Latest Release"></a>
</p>

<p align="center">
  <a href="https://stringke.github.io/claudex/pl/">Dokumentacja</a>
</p>

<p align="center">
  <a href="./README.md">English</a> |
  <a href="./README.zh-CN.md">简体中文</a> |
  <a href="./README.zh-TW.md">繁體中文</a> |
  <a href="./README.ja.md">日本語</a> |
  <a href="./README.ko.md">한국어</a> |
  <a href="./README.ru.md">Русский</a> |
  <a href="./README.fr.md">Français</a> |
  <a href="./README.pt-BR.md">Português do Brasil</a> |
  <a href="./README.es.md">Español</a> |
  <a href="./README.it.md">Italiano</a> |
  <a href="./README.de.md">Deutsch</a> |
  Polski
</p>

---

Claudex to zunifikowane proxy, które umożliwia [Claude Code](https://docs.anthropic.com/en/docs/claude-code) bezproblemową współpracę z wieloma dostawcami AI poprzez automatyczne tłumaczenie protokołów.

## Funkcje

- **Proxy dla wielu dostawców** — bezpośrednie przekazywanie DirectAnthropic + tłumaczenie Anthropic <-> OpenAI Chat Completions + tłumaczenie Anthropic <-> Responses API
- **Ponad 20 dostawców** — Anthropic, OpenRouter, Grok, OpenAI, DeepSeek, Kimi, GLM, Groq, Mistral, Together AI, Perplexity, Cerebras, Azure OpenAI, Google Vertex AI, Ollama, LM Studio i inne
- **Tłumaczenie strumieniowe** — pełne tłumaczenie strumieni SSE z obsługą wywołań narzędzi
- **Wyłącznik automatyczny + failover** — automatyczne przełączanie na dostawców zapasowych z konfigurowalnymi progami
- **Inteligentne routowanie** — automatyczny wybór dostawcy na podstawie intencji za pomocą lokalnego klasyfikatora
- **Silnik kontekstu** — kompresja konwersacji, udostępnianie między profilami, lokalne RAG z osadzeniami
- **Subskrypcje OAuth** — ChatGPT/Codex, Claude Max, GitHub Copilot, GitLab Duo, Google Gemini, Qwen, Kimi
- **Zestawy konfiguracji** — instalacja i zarządzanie wielokrotnie używalnymi zestawami konfiguracji Claude Code z repozytoriów git
- **Panel TUI** — stan profilów w czasie rzeczywistym, metryki, logi i szybkie uruchamianie
- **Automatyczna aktualizacja** — `claudex update` pobiera najnowsze wydanie z GitHuba

## Instalacja

```bash
# Jednolinijkowa instalacja (Linux / macOS)
curl -fsSL https://raw.githubusercontent.com/StringKe/claudex/main/install.sh | bash

# Ze źródła
cargo install --git https://github.com/StringKe/claudex

# Lub pobierz z GitHub Releases
# https://github.com/StringKe/claudex/releases
```

### Wymagania systemowe

- macOS (Intel / Apple Silicon) lub Linux (x86_64 / ARM64)
- Zainstalowany [Claude Code](https://docs.anthropic.com/en/docs/claude-code)
- Windows: pobierz gotowy plik binarny z [Releases](https://github.com/StringKe/claudex/releases)

## Szybki start

```bash
# 1. Zainicjuj konfigurację
claudex config init

# 2. Dodaj profil dostawcy interaktywnie
claudex profile add

# 3. Przetestuj łączność
claudex profile test all

# 4. Uruchom Claude Code z określonym dostawcą
claudex run grok

# 5. Lub użyj inteligentnego routowania do automatycznego wyboru najlepszego dostawcy
claudex run auto
```

## Jak to działa

```
claudex run openrouter-claude
    │
    ├── Uruchom proxy (jeśli nie działa) → 127.0.0.1:13456
    │
    └── exec claude ze zmiennymi środowiskowymi:
        ANTHROPIC_BASE_URL=http://127.0.0.1:13456/proxy/openrouter-claude
        ANTHROPIC_AUTH_TOKEN=claudex-passthrough
        ANTHROPIC_MODEL=anthropic/claude-sonnet-4
        ANTHROPIC_DEFAULT_HAIKU_MODEL=...
        ANTHROPIC_DEFAULT_SONNET_MODEL=...
        ANTHROPIC_DEFAULT_OPUS_MODEL=...
```

Proxy przechwytuje żądania i obsługuje tłumaczenie protokołów:

- **DirectAnthropic** (Anthropic, MiniMax, Vertex AI) → przekazywanie z poprawnymi nagłówkami
- **OpenAICompatible** (Grok, OpenAI, DeepSeek itp.) → Anthropic → OpenAI Chat Completions → tłumaczenie odpowiedzi z powrotem
- **OpenAIResponses** (subskrypcje ChatGPT/Codex) → Anthropic → Responses API → tłumaczenie odpowiedzi z powrotem

## Zgodność z dostawcami

| Dostawca | Typ | Tłumaczenie | Uwierzytelnianie | Przykładowy model |
|----------|-----|-------------|------------------|-------------------|
| Anthropic | DirectAnthropic | Brak | Klucz API | `claude-sonnet-4-20250514` |
| MiniMax | DirectAnthropic | Brak | Klucz API | `claude-sonnet-4-20250514` |
| OpenRouter | OpenAICompatible | Anthropic <-> OpenAI | Klucz API | `anthropic/claude-sonnet-4` |
| Grok (xAI) | OpenAICompatible | Anthropic <-> OpenAI | Klucz API | `grok-3-beta` |
| OpenAI | OpenAICompatible | Anthropic <-> OpenAI | Klucz API | `gpt-4o` |
| DeepSeek | OpenAICompatible | Anthropic <-> OpenAI | Klucz API | `deepseek-chat` |
| Kimi | OpenAICompatible | Anthropic <-> OpenAI | Klucz API | `kimi-k2-0905-preview` |
| GLM (Zhipu) | OpenAICompatible | Anthropic <-> OpenAI | Klucz API | `glm-4-plus` |
| Groq | OpenAICompatible | Anthropic <-> OpenAI | Klucz API | `llama-3.3-70b` |
| Mistral | OpenAICompatible | Anthropic <-> OpenAI | Klucz API | `mistral-large-latest` |
| Together AI | OpenAICompatible | Anthropic <-> OpenAI | Klucz API | `meta-llama/...` |
| Perplexity | OpenAICompatible | Anthropic <-> OpenAI | Klucz API | `sonar-pro` |
| Cerebras | OpenAICompatible | Anthropic <-> OpenAI | Klucz API | `llama-3.3-70b` |
| Azure OpenAI | OpenAICompatible | Anthropic <-> OpenAI | nagłówek api-key | `gpt-4o` |
| Google Vertex AI | DirectAnthropic | Brak | Bearer (gcloud) | `claude-sonnet-4@...` |
| Ollama | OpenAICompatible | Anthropic <-> OpenAI | Brak | `qwen2.5:72b` |
| LM Studio | OpenAICompatible | Anthropic <-> OpenAI | Brak | model lokalny |
| ChatGPT/Codex sub | OpenAIResponses | Anthropic <-> Responses | OAuth (PKCE/Device) | `gpt-5.3-codex` |
| Claude Max sub | DirectAnthropic | Brak | OAuth (plik) | `claude-sonnet-4` |
| GitHub Copilot | OpenAICompatible | Anthropic <-> OpenAI | OAuth (Device+Bearer) | `gpt-4o` |
| GitLab Duo | OpenAICompatible | Anthropic <-> OpenAI | GITLAB_TOKEN | `claude-sonnet-4` |

## Konfiguracja

Claudex przeszukuje pliki konfiguracyjne w następującej kolejności:

1. Zmienna środowiskowa `$CLAUDEX_CONFIG`
2. `./claudex.toml` lub `./claudex.yaml` (bieżący katalog)
3. `./.claudex/config.toml`
4. Katalogi nadrzędne (do 10 poziomów)
5. `~/.config/claudex/config.toml` (globalna, zalecana)

Obsługuje formaty TOML i YAML. Pełny opis znajdziesz w [`config.example.toml`](./config.example.toml).

## Dokumentacja CLI

| Polecenie | Opis |
|-----------|------|
| `claudex run <profile>` | Uruchom Claude Code z określonym dostawcą |
| `claudex run auto` | Inteligentne routowanie — automatyczny wybór najlepszego dostawcy |
| `claudex run <profile> -m <model>` | Nadpisz model dla sesji |
| `claudex profile list` | Wylistuj wszystkie skonfigurowane profile |
| `claudex profile add` | Interaktywny kreator konfiguracji profilu |
| `claudex profile show <name>` | Pokaż szczegóły profilu |
| `claudex profile remove <name>` | Usuń profil |
| `claudex profile test <name\|all>` | Przetestuj łączność z dostawcą |
| `claudex proxy start [-p port] [-d]` | Uruchom proxy (opcjonalnie jako demon) |
| `claudex proxy stop` | Zatrzymaj demona proxy |
| `claudex proxy status` | Pokaż stan proxy |
| `claudex dashboard` | Uruchom panel TUI |
| `claudex config show [--raw] [--json]` | Pokaż załadowaną konfigurację |
| `claudex config init [--yaml]` | Utwórz konfigurację w bieżącym katalogu |
| `claudex config edit [--global]` | Otwórz konfigurację w $EDITOR |
| `claudex config validate [--connectivity]` | Zweryfikuj konfigurację |
| `claudex config get <key>` | Pobierz wartość konfiguracji |
| `claudex config set <key> <value>` | Ustaw wartość konfiguracji |
| `claudex config export --format <fmt>` | Eksportuj konfigurację (json/toml/yaml) |
| `claudex update [--check]` | Automatyczna aktualizacja z GitHub Releases |
| `claudex auth login <provider>` | Logowanie OAuth |
| `claudex auth login github --enterprise-url <domain>` | GitHub Enterprise Copilot |
| `claudex auth status` | Pokaż stan tokenów OAuth |
| `claudex auth logout <profile>` | Usuń token OAuth |
| `claudex auth refresh <profile>` | Wymuś odświeżenie tokenu OAuth |
| `claudex sets add <source> [--global]` | Zainstaluj zestaw konfiguracji |
| `claudex sets remove <name>` | Usuń zestaw konfiguracji |
| `claudex sets list [--global]` | Wylistuj zainstalowane zestawy |
| `claudex sets update [name]` | Zaktualizuj zestawy do najnowszej wersji |

## Subskrypcje OAuth

Używaj istniejących subskrypcji zamiast kluczy API:

```bash
# Subskrypcja ChatGPT (automatycznie wykrywa istniejące poświadczenia Codex CLI)
claudex auth login chatgpt --profile codex-sub

# ChatGPT — wymuś logowanie przez przeglądarkę
claudex auth login chatgpt --profile codex-sub --force

# ChatGPT bez ekranu (SSH/bez przeglądarki)
claudex auth login chatgpt --profile codex-sub --force --headless

# GitHub Copilot
claudex auth login github --profile copilot

# GitHub Copilot Enterprise
claudex auth login github --profile copilot-ent --enterprise-url company.ghe.com

# GitLab Duo (odczytuje zmienną środowiskową GITLAB_TOKEN)
claudex auth login gitlab --profile gitlab-duo

# Sprawdź stan
claudex auth status

# Uruchom z subskrypcją
claudex run codex-sub
```

Obsługiwane: `claude`, `chatgpt`/`openai`, `google`, `qwen`, `kimi`, `github`/`copilot`, `gitlab`

## Mapowanie slotów modeli

Mapuj przełącznik `/model` w Claude Code (haiku/sonnet/opus) na modele dowolnego dostawcy:

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

## Architektura

```
src/
├── main.rs
├── cli.rs
├── update.rs
├── util.rs
├── config/
│   ├── mod.rs          # Wykrywanie i parsowanie konfiguracji (figment)
│   ├── cmd.rs          # Podpolecenia config get/set/export/validate
│   └── profile.rs      # CRUD profili + test łączności
├── process/
│   ├── mod.rs
│   ├── launch.rs       # Uruchamianie procesu Claude
│   └── daemon.rs       # Plik PID + zarządzanie procesem
├── oauth/
│   ├── mod.rs          # AuthType, OAuthProvider, OAuthToken
│   ├── source.rs       # Warstwa 1: źródła poświadczeń (env/plik/keyring)
│   ├── exchange.rs     # Warstwa 2: wymiana tokenów (PKCE/device code/odświeżanie)
│   ├── manager.rs      # Warstwa 3: cache + deduplikacja współbieżna + retry 401
│   ├── handler.rs      # Cecha OAuthProviderHandler
│   ├── providers.rs    # Logika CLI logowania/odświeżania/stanu
│   ├── server.rs       # Serwer wywołań zwrotnych OAuth + polling device code
│   └── token.rs        # Re-eksporty
├── proxy/
│   ├── mod.rs          # Serwer Axum + ProxyState
│   ├── handler.rs      # Routowanie żądań + wyłącznik automatyczny + retry 401
│   ├── adapter/        # Adaptery specyficzne dla dostawców
│   │   ├── mod.rs      # Cecha ProviderAdapter + fabryka
│   │   ├── direct.rs   # DirectAnthropic (passthrough)
│   │   ├── chat_completions.rs  # OpenAI Chat Completions
│   │   └── responses.rs         # OpenAI Responses API
│   ├── translate/      # Tłumaczenie protokołów
│   │   ├── chat_completions.rs
│   │   ├── chat_completions_stream.rs
│   │   ├── responses.rs
│   │   └── responses_stream.rs
│   ├── context_engine.rs
│   ├── fallback.rs     # Wyłącznik automatyczny
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
├── sets/               # Zarządzanie zestawami konfiguracji
│   ├── mod.rs
│   ├── schema.rs
│   ├── source.rs
│   ├── install.rs
│   ├── lock.rs
│   ├── conflict.rs
│   └── mcp.rs
├── terminal/           # Wykrywanie terminala + hiperłącza
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

## Licencja

[MIT](./LICENSE)
