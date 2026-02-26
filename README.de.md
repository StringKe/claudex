<p align="center">
  <h1 align="center">Claudex</h1>
  <p align="center">Multi-Instanz Claude Code Manager mit intelligentem Übersetzungsproxy</p>
</p>

<p align="center">
  <a href="https://github.com/StringKe/claudex/actions/workflows/ci.yml"><img src="https://github.com/StringKe/claudex/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
  <a href="https://github.com/StringKe/claudex/releases"><img src="https://github.com/StringKe/claudex/actions/workflows/release.yml/badge.svg" alt="Release"></a>
  <a href="https://github.com/StringKe/claudex/blob/main/LICENSE"><img src="https://img.shields.io/github/license/StringKe/claudex" alt="Lizenz"></a>
  <a href="https://github.com/StringKe/claudex/releases"><img src="https://img.shields.io/github/v/release/StringKe/claudex" alt="Neueste Version"></a>
</p>

<p align="center">
  <a href="https://stringke.github.io/claudex/de/">Dokumentation</a>
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
  Deutsch |
  <a href="./README.pl.md">Polski</a>
</p>

---

Claudex ist ein einheitlicher Proxy, der es [Claude Code](https://docs.anthropic.com/en/docs/claude-code) ermöglicht, durch automatische Protokollübersetzung nahtlos mit mehreren KI-Anbietern zusammenzuarbeiten.

## Funktionen

- **Multi-Anbieter-Proxy** — DirectAnthropic-Durchleitung + Anthropic <-> OpenAI Chat Completions Übersetzung + Anthropic <-> Responses API Übersetzung
- **20+ Anbieter** — Anthropic, OpenRouter, Grok, OpenAI, DeepSeek, Kimi, GLM, Groq, Mistral, Together AI, Perplexity, Cerebras, Azure OpenAI, Google Vertex AI, Ollama, LM Studio und weitere
- **Streaming-Übersetzung** — Vollständige SSE-Stream-Übersetzung mit Tool-Call-Unterstützung
- **Circuit Breaker + Failover** — Automatischer Fallback zu Backup-Anbietern mit konfigurierbaren Schwellenwerten
- **Intelligentes Routing** — Absichtsbasiertes automatisches Routing über lokalen Klassifikator
- **Kontext-Engine** — Gesprächskomprimierung, profilübergreifende Weitergabe, lokales RAG mit Einbettungen
- **OAuth-Abonnements** — ChatGPT/Codex, Claude Max, GitHub Copilot, GitLab Duo, Google Gemini, Qwen, Kimi
- **Konfigurationssets** — Wiederverwendbare Claude Code Konfigurationssets aus Git-Repos installieren und verwalten
- **TUI-Dashboard** — Echtzeit-Profilzustand, Metriken, Protokolle und Schnellstart
- **Selbst-Update** — `claudex update` lädt die neueste Version von GitHub herunter

## Installation

```bash
# Einzeiler (Linux / macOS)
curl -fsSL https://raw.githubusercontent.com/StringKe/claudex/main/install.sh | bash

# Aus dem Quellcode
cargo install --git https://github.com/StringKe/claudex

# Oder von GitHub Releases herunterladen
# https://github.com/StringKe/claudex/releases
```

### Systemanforderungen

- macOS (Intel / Apple Silicon) oder Linux (x86_64 / ARM64)
- [Claude Code](https://docs.anthropic.com/en/docs/claude-code) installiert
- Windows: vorkompilierte Binärdatei von [Releases](https://github.com/StringKe/claudex/releases) herunterladen

## Schnellstart

```bash
# 1. Konfiguration initialisieren
claudex config init

# 2. Interaktiv ein Anbieterprofil hinzufügen
claudex profile add

# 3. Konnektivität testen
claudex profile test all

# 4. Claude Code mit einem bestimmten Anbieter starten
claudex run grok

# 5. Oder intelligentes Routing nutzen, um den besten Anbieter automatisch auszuwählen
claudex run auto
```

## Funktionsweise

```
claudex run openrouter-claude
    │
    ├── Proxy starten (falls nicht aktiv) → 127.0.0.1:13456
    │
    └── claude ausführen mit Umgebungsvariablen:
        ANTHROPIC_BASE_URL=http://127.0.0.1:13456/proxy/openrouter-claude
        ANTHROPIC_AUTH_TOKEN=claudex-passthrough
        ANTHROPIC_MODEL=anthropic/claude-sonnet-4
        ANTHROPIC_DEFAULT_HAIKU_MODEL=...
        ANTHROPIC_DEFAULT_SONNET_MODEL=...
        ANTHROPIC_DEFAULT_OPUS_MODEL=...
```

Der Proxy fängt Anfragen ab und übernimmt die Protokollübersetzung:

- **DirectAnthropic** (Anthropic, MiniMax, Vertex AI) → Weiterleitung mit korrekten Headern
- **OpenAICompatible** (Grok, OpenAI, DeepSeek, etc.) → Anthropic → OpenAI Chat Completions → Antwort zurück übersetzen
- **OpenAIResponses** (ChatGPT/Codex-Abonnements) → Anthropic → Responses API → Antwort zurück übersetzen

## Anbieterkompatibilität

| Anbieter | Typ | Übersetzung | Authentifizierung | Beispielmodell |
|----------|-----|-------------|-------------------|----------------|
| Anthropic | DirectAnthropic | Keine | API Key | `claude-sonnet-4-20250514` |
| MiniMax | DirectAnthropic | Keine | API Key | `claude-sonnet-4-20250514` |
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
| Azure OpenAI | OpenAICompatible | Anthropic <-> OpenAI | api-key Header | `gpt-4o` |
| Google Vertex AI | DirectAnthropic | Keine | Bearer (gcloud) | `claude-sonnet-4@...` |
| Ollama | OpenAICompatible | Anthropic <-> OpenAI | Keine | `qwen2.5:72b` |
| LM Studio | OpenAICompatible | Anthropic <-> OpenAI | Keine | lokales Modell |
| ChatGPT/Codex-Abo | OpenAIResponses | Anthropic <-> Responses | OAuth (PKCE/Device) | `gpt-5.3-codex` |
| Claude Max-Abo | DirectAnthropic | Keine | OAuth (Datei) | `claude-sonnet-4` |
| GitHub Copilot | OpenAICompatible | Anthropic <-> OpenAI | OAuth (Device+Bearer) | `gpt-4o` |
| GitLab Duo | OpenAICompatible | Anthropic <-> OpenAI | GITLAB_TOKEN | `claude-sonnet-4` |

## Konfiguration

Claudex sucht Konfigurationsdateien in dieser Reihenfolge:

1. Umgebungsvariable `$CLAUDEX_CONFIG`
2. `./claudex.toml` oder `./claudex.yaml` (aktuelles Verzeichnis)
3. `./.claudex/config.toml`
4. Übergeordnete Verzeichnisse (bis zu 10 Ebenen)
5. `~/.config/claudex/config.toml` (global, empfohlen)

Unterstützt TOML- und YAML-Format. Vollständige Referenz in [`config.example.toml`](./config.example.toml).

## CLI-Referenz

| Befehl | Beschreibung |
|--------|--------------|
| `claudex run <profile>` | Claude Code mit einem bestimmten Anbieter starten |
| `claudex run auto` | Intelligentes Routing — besten Anbieter automatisch auswählen |
| `claudex run <profile> -m <model>` | Modell für eine Sitzung überschreiben |
| `claudex profile list` | Alle konfigurierten Profile auflisten |
| `claudex profile add` | Interaktiver Profil-Einrichtungsassistent |
| `claudex profile show <name>` | Profildetails anzeigen |
| `claudex profile remove <name>` | Profil entfernen |
| `claudex profile test <name\|all>` | Anbieterkonnektivität testen |
| `claudex proxy start [-p port] [-d]` | Proxy starten (optional als Daemon) |
| `claudex proxy stop` | Proxy-Daemon stoppen |
| `claudex proxy status` | Proxy-Status anzeigen |
| `claudex dashboard` | TUI-Dashboard starten |
| `claudex config show [--raw] [--json]` | Geladene Konfiguration anzeigen |
| `claudex config init [--yaml]` | Konfiguration im aktuellen Verzeichnis erstellen |
| `claudex config edit [--global]` | Konfiguration in $EDITOR öffnen |
| `claudex config validate [--connectivity]` | Konfiguration validieren |
| `claudex config get <key>` | Konfigurationswert abrufen |
| `claudex config set <key> <value>` | Konfigurationswert setzen |
| `claudex config export --format <fmt>` | Konfiguration exportieren (json/toml/yaml) |
| `claudex update [--check]` | Selbst-Update von GitHub Releases |
| `claudex auth login <provider>` | OAuth-Anmeldung |
| `claudex auth login github --enterprise-url <domain>` | GitHub Enterprise Copilot |
| `claudex auth status` | OAuth-Token-Status anzeigen |
| `claudex auth logout <profile>` | OAuth-Token entfernen |
| `claudex auth refresh <profile>` | OAuth-Token zwangsweise erneuern |
| `claudex sets add <source> [--global]` | Konfigurationsset installieren |
| `claudex sets remove <name>` | Konfigurationsset entfernen |
| `claudex sets list [--global]` | Installierte Sets auflisten |
| `claudex sets update [name]` | Sets auf den neuesten Stand bringen |

## OAuth-Abonnements

Vorhandene Abonnements statt API-Schlüssel verwenden:

```bash
# ChatGPT-Abonnement (erkennt vorhandene Codex CLI-Zugangsdaten automatisch)
claudex auth login chatgpt --profile codex-sub

# ChatGPT Browser-Anmeldung erzwingen
claudex auth login chatgpt --profile codex-sub --force

# ChatGPT headless (SSH/kein Browser)
claudex auth login chatgpt --profile codex-sub --force --headless

# GitHub Copilot
claudex auth login github --profile copilot

# GitHub Copilot Enterprise
claudex auth login github --profile copilot-ent --enterprise-url company.ghe.com

# GitLab Duo (liest GITLAB_TOKEN-Umgebungsvariable)
claudex auth login gitlab --profile gitlab-duo

# Status prüfen
claudex auth status

# Mit Abonnement starten
claudex run codex-sub
```

Unterstützt: `claude`, `chatgpt`/`openai`, `google`, `qwen`, `kimi`, `github`/`copilot`, `gitlab`

## Modell-Slot-Zuordnung

Den `/model`-Umschalter von Claude Code (haiku/sonnet/opus) den Modellen beliebiger Anbieter zuordnen:

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

## Architektur

```
src/
├── main.rs
├── cli.rs
├── update.rs
├── util.rs
├── config/
│   ├── mod.rs          # Konfigurationssuche + Parsing (figment)
│   ├── cmd.rs          # config get/set/export/validate Unterbefehle
│   └── profile.rs      # Profil-CRUD + Konnektivitätstest
├── process/
│   ├── mod.rs
│   ├── launch.rs       # Claude-Prozessstarter
│   └── daemon.rs       # PID-Datei + Prozessverwaltung
├── oauth/
│   ├── mod.rs          # AuthType, OAuthProvider, OAuthToken
│   ├── source.rs       # Ebene 1: Zugangsdatenquellen (env/file/keyring)
│   ├── exchange.rs     # Ebene 2: Token-Austausch (PKCE/device code/refresh)
│   ├── manager.rs      # Ebene 3: Cache + parallele Deduplizierung + 401-Wiederholung
│   ├── handler.rs      # OAuthProviderHandler-Trait
│   ├── providers.rs    # Anmelde-/Aktualisierungs-/Status-CLI-Logik
│   ├── server.rs       # OAuth-Callback-Server + Device-Code-Polling
│   └── token.rs        # Re-Exporte
├── proxy/
│   ├── mod.rs          # Axum-Server + ProxyState
│   ├── handler.rs      # Anfrage-Routing + Circuit Breaker + 401-Wiederholung
│   ├── adapter/        # Anbieterspezifische Adapter
│   │   ├── mod.rs      # ProviderAdapter-Trait + Factory
│   │   ├── direct.rs   # DirectAnthropic (Durchleitung)
│   │   ├── chat_completions.rs  # OpenAI Chat Completions
│   │   └── responses.rs         # OpenAI Responses API
│   ├── translate/      # Protokollübersetzung
│   │   ├── chat_completions.rs
│   │   ├── chat_completions_stream.rs
│   │   ├── responses.rs
│   │   └── responses_stream.rs
│   ├── context_engine.rs
│   ├── fallback.rs     # Circuit Breaker
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
├── sets/               # Verwaltung von Konfigurationssets
│   ├── mod.rs
│   ├── schema.rs
│   ├── source.rs
│   ├── install.rs
│   ├── lock.rs
│   ├── conflict.rs
│   └── mcp.rs
├── terminal/           # Terminal-Erkennung + Hyperlinks
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

## Lizenz

[MIT](./LICENSE)
