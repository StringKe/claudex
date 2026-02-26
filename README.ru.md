<p align="center">
  <h1 align="center">Claudex</h1>
  <p align="center">Менеджер мульти-инстансов Claude Code с интеллектуальным прокси-переводчиком</p>
</p>

<p align="center">
  <a href="https://github.com/StringKe/claudex/actions/workflows/ci.yml"><img src="https://github.com/StringKe/claudex/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
  <a href="https://github.com/StringKe/claudex/releases"><img src="https://github.com/StringKe/claudex/actions/workflows/release.yml/badge.svg" alt="Release"></a>
  <a href="https://github.com/StringKe/claudex/blob/main/LICENSE"><img src="https://img.shields.io/github/license/StringKe/claudex" alt="License"></a>
  <a href="https://github.com/StringKe/claudex/releases"><img src="https://img.shields.io/github/v/release/StringKe/claudex" alt="Latest Release"></a>
</p>

<p align="center">
  <a href="https://stringke.github.io/claudex/ru/">Документация</a>
</p>

<p align="center">
  <a href="./README.md">English</a> |
  <a href="./README.zh-CN.md">简体中文</a> |
  <a href="./README.zh-TW.md">繁體中文</a> |
  <a href="./README.ja.md">日本語</a> |
  <a href="./README.ko.md">한국어</a> |
  Русский |
  <a href="./README.fr.md">Français</a> |
  <a href="./README.pt-BR.md">Português do Brasil</a> |
  <a href="./README.es.md">Español</a> |
  <a href="./README.it.md">Italiano</a> |
  <a href="./README.de.md">Deutsch</a> |
  <a href="./README.pl.md">Polski</a>
</p>

---

Claudex — единый прокси, позволяющий [Claude Code](https://docs.anthropic.com/en/docs/claude-code) беспрепятственно работать с множеством AI-провайдеров посредством автоматического перевода протоколов.

## Возможности

- **Мульти-провайдерный прокси** — прямой проброс DirectAnthropic + перевод Anthropic <-> OpenAI Chat Completions + перевод Anthropic <-> Responses API
- **20+ провайдеров** — Anthropic, OpenRouter, Grok, OpenAI, DeepSeek, Kimi, GLM, Groq, Mistral, Together AI, Perplexity, Cerebras, Azure OpenAI, Google Vertex AI, Ollama, LM Studio и другие
- **Потоковый перевод** — полный перевод SSE-потоков с поддержкой вызовов инструментов
- **Автоматический переключатель и резервирование** — автоматический переход на резервных провайдеров с настраиваемыми порогами
- **Умная маршрутизация** — автовыбор провайдера на основе намерения запроса через локальный классификатор
- **Контекстный движок** — сжатие диалогов, совместное использование между профилями, локальный RAG с эмбеддингами
- **OAuth-подписки** — ChatGPT/Codex, Claude Max, GitHub Copilot, GitLab Duo, Google Gemini, Qwen, Kimi
- **Наборы конфигураций** — установка и управление повторно используемыми наборами настроек Claude Code из git-репозиториев
- **TUI-панель** — состояние профилей, метрики, логи и быстрый запуск в реальном времени
- **Самообновление** — `claudex update` скачивает последний релиз с GitHub

## Установка

```bash
# Однострочная установка (Linux / macOS)
curl -fsSL https://raw.githubusercontent.com/StringKe/claudex/main/install.sh | bash

# Из исходников
cargo install --git https://github.com/StringKe/claudex

# Или скачайте готовый бинарник из GitHub Releases
# https://github.com/StringKe/claudex/releases
```

### Системные требования

- macOS (Intel / Apple Silicon) или Linux (x86_64 / ARM64)
- Установленный [Claude Code](https://docs.anthropic.com/en/docs/claude-code)
- Windows: скачайте готовый бинарник из [Releases](https://github.com/StringKe/claudex/releases)

## Быстрый старт

```bash
# 1. Инициализация конфигурации
claudex config init

# 2. Добавление профиля провайдера в интерактивном режиме
claudex profile add

# 3. Проверка подключения
claudex profile test all

# 4. Запуск Claude Code с конкретным провайдером
claudex run grok

# 5. Или используйте умную маршрутизацию для автовыбора лучшего провайдера
claudex run auto
```

## Принцип работы

```
claudex run openrouter-claude
    │
    ├── Запуск прокси (если не запущен) → 127.0.0.1:13456
    │
    └── exec claude с переменными окружения:
        ANTHROPIC_BASE_URL=http://127.0.0.1:13456/proxy/openrouter-claude
        ANTHROPIC_AUTH_TOKEN=claudex-passthrough
        ANTHROPIC_MODEL=anthropic/claude-sonnet-4
        ANTHROPIC_DEFAULT_HAIKU_MODEL=...
        ANTHROPIC_DEFAULT_SONNET_MODEL=...
        ANTHROPIC_DEFAULT_OPUS_MODEL=...
```

Прокси перехватывает запросы и выполняет перевод протоколов:

- **DirectAnthropic** (Anthropic, MiniMax, Vertex AI) → пробрасывает с правильными заголовками
- **OpenAICompatible** (Grok, OpenAI, DeepSeek и др.) → Anthropic → OpenAI Chat Completions → обратный перевод ответа
- **OpenAIResponses** (подписки ChatGPT/Codex) → Anthropic → Responses API → обратный перевод ответа

## Совместимость провайдеров

| Провайдер | Тип | Перевод | Аутентификация | Пример модели |
|-----------|-----|---------|----------------|---------------|
| Anthropic | DirectAnthropic | Нет | API Key | `claude-sonnet-4-20250514` |
| MiniMax | DirectAnthropic | Нет | API Key | `claude-sonnet-4-20250514` |
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
| Google Vertex AI | DirectAnthropic | Нет | Bearer (gcloud) | `claude-sonnet-4@...` |
| Ollama | OpenAICompatible | Anthropic <-> OpenAI | Нет | `qwen2.5:72b` |
| LM Studio | OpenAICompatible | Anthropic <-> OpenAI | Нет | local model |
| ChatGPT/Codex sub | OpenAIResponses | Anthropic <-> Responses | OAuth (PKCE/Device) | `gpt-5.3-codex` |
| Claude Max sub | DirectAnthropic | Нет | OAuth (file) | `claude-sonnet-4` |
| GitHub Copilot | OpenAICompatible | Anthropic <-> OpenAI | OAuth (Device+Bearer) | `gpt-4o` |
| GitLab Duo | OpenAICompatible | Anthropic <-> OpenAI | GITLAB_TOKEN | `claude-sonnet-4` |

## Конфигурация

Claudex ищет файлы конфигурации в следующем порядке:

1. Переменная окружения `$CLAUDEX_CONFIG`
2. `./claudex.toml` или `./claudex.yaml` (текущая директория)
3. `./.claudex/config.toml`
4. Родительские директории (до 10 уровней)
5. `~/.config/claudex/config.toml` (глобальная, рекомендуется)

Поддерживаются форматы TOML и YAML. Полный справочник см. в [`config.example.toml`](./config.example.toml).

## Справочник CLI

| Команда | Описание |
|---------|----------|
| `claudex run <profile>` | Запуск Claude Code с конкретным провайдером |
| `claudex run auto` | Умная маршрутизация — автовыбор лучшего провайдера |
| `claudex run <profile> -m <model>` | Переопределение модели для сессии |
| `claudex profile list` | Список всех настроенных профилей |
| `claudex profile add` | Интерактивный мастер настройки профиля |
| `claudex profile show <name>` | Детали профиля |
| `claudex profile remove <name>` | Удалить профиль |
| `claudex profile test <name\|all>` | Проверить подключение к провайдеру |
| `claudex proxy start [-p port] [-d]` | Запустить прокси (опционально как демон) |
| `claudex proxy stop` | Остановить прокси-демон |
| `claudex proxy status` | Состояние прокси |
| `claudex dashboard` | Открыть TUI-панель |
| `claudex config show [--raw] [--json]` | Показать загруженную конфигурацию |
| `claudex config init [--yaml]` | Создать конфигурацию в текущей директории |
| `claudex config edit [--global]` | Открыть конфигурацию в $EDITOR |
| `claudex config validate [--connectivity]` | Валидировать конфигурацию |
| `claudex config get <key>` | Получить значение параметра |
| `claudex config set <key> <value>` | Установить значение параметра |
| `claudex config export --format <fmt>` | Экспорт конфигурации (json/toml/yaml) |
| `claudex update [--check]` | Самообновление из GitHub Releases |
| `claudex auth login <provider>` | Вход через OAuth |
| `claudex auth login github --enterprise-url <domain>` | GitHub Enterprise Copilot |
| `claudex auth status` | Состояние OAuth-токенов |
| `claudex auth logout <profile>` | Удалить OAuth-токен |
| `claudex auth refresh <profile>` | Принудительное обновление OAuth-токена |
| `claudex sets add <source> [--global]` | Установить набор конфигураций |
| `claudex sets remove <name>` | Удалить набор конфигураций |
| `claudex sets list [--global]` | Список установленных наборов |
| `claudex sets update [name]` | Обновить наборы до последней версии |

## OAuth-подписки

Используйте существующие подписки вместо API-ключей:

```bash
# Подписка ChatGPT (автоматически обнаруживает существующие учётные данные Codex CLI)
claudex auth login chatgpt --profile codex-sub

# ChatGPT с принудительным входом через браузер
claudex auth login chatgpt --profile codex-sub --force

# ChatGPT в безголовом режиме (SSH/без браузера)
claudex auth login chatgpt --profile codex-sub --force --headless

# GitHub Copilot
claudex auth login github --profile copilot

# GitHub Copilot Enterprise
claudex auth login github --profile copilot-ent --enterprise-url company.ghe.com

# GitLab Duo (читает переменную окружения GITLAB_TOKEN)
claudex auth login gitlab --profile gitlab-duo

# Проверка статуса
claudex auth status

# Запуск с подпиской
claudex run codex-sub
```

Поддерживаются: `claude`, `chatgpt`/`openai`, `google`, `qwen`, `kimi`, `github`/`copilot`, `gitlab`

## Маппинг слотов моделей

Сопоставьте переключатель `/model` Claude Code (haiku/sonnet/opus) с моделями любого провайдера:

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

## Архитектура

```
src/
├── main.rs
├── cli.rs
├── update.rs
├── util.rs
├── config/
│   ├── mod.rs          # Обнаружение и разбор конфигурации (figment)
│   ├── cmd.rs          # Подкоманды config get/set/export/validate
│   └── profile.rs      # CRUD профилей + тест подключения
├── process/
│   ├── mod.rs
│   ├── launch.rs       # Запуск процесса Claude
│   └── daemon.rs       # PID-файл + управление процессами
├── oauth/
│   ├── mod.rs          # AuthType, OAuthProvider, OAuthToken
│   ├── source.rs       # Уровень 1: источники учётных данных (env/file/keyring)
│   ├── exchange.rs     # Уровень 2: обмен токенами (PKCE/device code/refresh)
│   ├── manager.rs      # Уровень 3: кэш + дедупликация конкурентных запросов + повтор при 401
│   ├── handler.rs      # Трейт OAuthProviderHandler
│   ├── providers.rs    # Логика CLI для входа/обновления/статуса
│   ├── server.rs       # Сервер OAuth-коллбэков + опрос device code
│   └── token.rs        # Реэкспорты
├── proxy/
│   ├── mod.rs          # Axum-сервер + ProxyState
│   ├── handler.rs      # Маршрутизация запросов + автоматический переключатель + повтор при 401
│   ├── adapter/        # Адаптеры для провайдеров
│   │   ├── mod.rs      # Трейт ProviderAdapter + фабрика
│   │   ├── direct.rs   # DirectAnthropic (проброс)
│   │   ├── chat_completions.rs  # OpenAI Chat Completions
│   │   └── responses.rs         # OpenAI Responses API
│   ├── translate/      # Перевод протоколов
│   │   ├── chat_completions.rs
│   │   ├── chat_completions_stream.rs
│   │   ├── responses.rs
│   │   └── responses_stream.rs
│   ├── context_engine.rs
│   ├── fallback.rs     # Автоматический переключатель
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
├── sets/               # Управление наборами конфигураций
│   ├── mod.rs
│   ├── schema.rs
│   ├── source.rs
│   ├── install.rs
│   ├── lock.rs
│   ├── conflict.rs
│   └── mcp.rs
├── terminal/           # Определение терминала + гиперссылки
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

## Лицензия

[MIT](./LICENSE)
