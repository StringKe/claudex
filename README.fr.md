<p align="center">
  <h1 align="center">Claudex</h1>
  <p align="center">Gestionnaire multi-instances Claude Code avec proxy de traduction intelligent</p>
</p>

<p align="center">
  <a href="https://github.com/StringKe/claudex/actions/workflows/ci.yml"><img src="https://github.com/StringKe/claudex/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
  <a href="https://github.com/StringKe/claudex/releases"><img src="https://github.com/StringKe/claudex/actions/workflows/release.yml/badge.svg" alt="Release"></a>
  <a href="https://github.com/StringKe/claudex/blob/main/LICENSE"><img src="https://img.shields.io/github/license/StringKe/claudex" alt="License"></a>
  <a href="https://github.com/StringKe/claudex/releases"><img src="https://img.shields.io/github/v/release/StringKe/claudex" alt="Latest Release"></a>
</p>

<p align="center">
  <a href="https://stringke.github.io/claudex/fr/">Documentation</a>
</p>

<p align="center">
  <a href="./README.md">English</a> |
  <a href="./README.zh-CN.md">简体中文</a> |
  <a href="./README.zh-TW.md">繁體中文</a> |
  <a href="./README.ja.md">日本語</a> |
  <a href="./README.ko.md">한국어</a> |
  <a href="./README.ru.md">Русский</a> |
  Français |
  <a href="./README.pt-BR.md">Português do Brasil</a> |
  <a href="./README.es.md">Español</a> |
  <a href="./README.it.md">Italiano</a> |
  <a href="./README.de.md">Deutsch</a> |
  <a href="./README.pl.md">Polski</a>
</p>

---

Claudex est un proxy unifié qui permet à [Claude Code](https://docs.anthropic.com/en/docs/claude-code) de fonctionner de façon transparente avec plusieurs fournisseurs d'IA grâce à la traduction automatique de protocoles.

## Fonctionnalités

- **Proxy multi-fournisseurs** — Transfert direct DirectAnthropic + traduction Anthropic <-> OpenAI Chat Completions + traduction Anthropic <-> Responses API
- **Plus de 20 fournisseurs** — Anthropic, OpenRouter, Grok, OpenAI, DeepSeek, Kimi, GLM, Groq, Mistral, Together AI, Perplexity, Cerebras, Azure OpenAI, Google Vertex AI, Ollama, LM Studio, et plus encore
- **Traduction en streaming** — Traduction complète du flux SSE avec prise en charge des appels d'outils
- **Disjoncteur + basculement** — Repli automatique vers des fournisseurs de secours avec seuils configurables
- **Routage intelligent** — Routage automatique basé sur l'intention via un classificateur local
- **Moteur de contexte** — Compression des conversations, partage inter-profils, RAG local avec embeddings
- **Abonnements OAuth** — ChatGPT/Codex, Claude Max, GitHub Copilot, GitLab Duo, Google Gemini, Qwen, Kimi
- **Ensembles de configuration** — Installez et gérez des ensembles de configuration Claude Code réutilisables depuis des dépôts git
- **Tableau de bord TUI** — Santé des profils en temps réel, métriques, journaux et lancement rapide
- **Mise à jour automatique** — `claudex update` télécharge la dernière version depuis GitHub

## Installation

```bash
# En une seule commande (Linux / macOS)
curl -fsSL https://raw.githubusercontent.com/StringKe/claudex/main/install.sh | bash

# Depuis les sources
cargo install --git https://github.com/StringKe/claudex

# Ou télécharger depuis GitHub Releases
# https://github.com/StringKe/claudex/releases
```

### Configuration requise

- macOS (Intel / Apple Silicon) ou Linux (x86_64 / ARM64)
- [Claude Code](https://docs.anthropic.com/en/docs/claude-code) installé
- Windows : téléchargez le binaire précompilé depuis les [Releases](https://github.com/StringKe/claudex/releases)

## Démarrage rapide

```bash
# 1. Initialiser la configuration
claudex config init

# 2. Ajouter un profil de fournisseur de façon interactive
claudex profile add

# 3. Tester la connectivité
claudex profile test all

# 4. Lancer Claude Code avec un fournisseur spécifique
claudex run grok

# 5. Ou utiliser le routage intelligent pour sélectionner automatiquement le meilleur fournisseur
claudex run auto
```

## Fonctionnement

```
claudex run openrouter-claude
    │
    ├── Démarrer le proxy (si inactif) → 127.0.0.1:13456
    │
    └── exec claude avec les variables d'environnement :
        ANTHROPIC_BASE_URL=http://127.0.0.1:13456/proxy/openrouter-claude
        ANTHROPIC_AUTH_TOKEN=claudex-passthrough
        ANTHROPIC_MODEL=anthropic/claude-sonnet-4
        ANTHROPIC_DEFAULT_HAIKU_MODEL=...
        ANTHROPIC_DEFAULT_SONNET_MODEL=...
        ANTHROPIC_DEFAULT_OPUS_MODEL=...
```

Le proxy intercepte les requêtes et gère la traduction de protocoles :

- **DirectAnthropic** (Anthropic, MiniMax, Vertex AI) → transfert avec les en-têtes corrects
- **OpenAICompatible** (Grok, OpenAI, DeepSeek, etc.) → Anthropic → OpenAI Chat Completions → traduction de la réponse
- **OpenAIResponses** (abonnements ChatGPT/Codex) → Anthropic → Responses API → traduction de la réponse

## Compatibilité des fournisseurs

| Fournisseur | Type | Traduction | Auth | Exemple de modèle |
|-------------|------|------------|------|-------------------|
| Anthropic | DirectAnthropic | Aucune | API Key | `claude-sonnet-4-20250514` |
| MiniMax | DirectAnthropic | Aucune | API Key | `claude-sonnet-4-20250514` |
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
| Azure OpenAI | OpenAICompatible | Anthropic <-> OpenAI | en-tête api-key | `gpt-4o` |
| Google Vertex AI | DirectAnthropic | Aucune | Bearer (gcloud) | `claude-sonnet-4@...` |
| Ollama | OpenAICompatible | Anthropic <-> OpenAI | Aucune | `qwen2.5:72b` |
| LM Studio | OpenAICompatible | Anthropic <-> OpenAI | Aucune | modèle local |
| Abonnement ChatGPT/Codex | OpenAIResponses | Anthropic <-> Responses | OAuth (PKCE/Device) | `gpt-5.3-codex` |
| Abonnement Claude Max | DirectAnthropic | Aucune | OAuth (fichier) | `claude-sonnet-4` |
| GitHub Copilot | OpenAICompatible | Anthropic <-> OpenAI | OAuth (Device+Bearer) | `gpt-4o` |
| GitLab Duo | OpenAICompatible | Anthropic <-> OpenAI | GITLAB_TOKEN | `claude-sonnet-4` |

## Configuration

Claudex recherche les fichiers de configuration dans cet ordre :

1. Variable d'environnement `$CLAUDEX_CONFIG`
2. `./claudex.toml` ou `./claudex.yaml` (répertoire courant)
3. `./.claudex/config.toml`
4. Répertoires parents (jusqu'à 10 niveaux)
5. `~/.config/claudex/config.toml` (global, recommandé)

Prend en charge les formats TOML et YAML. Consultez [`config.example.toml`](./config.example.toml) pour la référence complète.

## Référence CLI

| Commande | Description |
|----------|-------------|
| `claudex run <profile>` | Lancer Claude Code avec un fournisseur spécifique |
| `claudex run auto` | Routage intelligent — sélectionner automatiquement le meilleur fournisseur |
| `claudex run <profile> -m <model>` | Remplacer le modèle pour une session |
| `claudex profile list` | Lister tous les profils configurés |
| `claudex profile add` | Assistant de configuration de profil interactif |
| `claudex profile show <name>` | Afficher les détails d'un profil |
| `claudex profile remove <name>` | Supprimer un profil |
| `claudex profile test <name\|all>` | Tester la connectivité du fournisseur |
| `claudex proxy start [-p port] [-d]` | Démarrer le proxy (optionnellement en tant que démon) |
| `claudex proxy stop` | Arrêter le démon proxy |
| `claudex proxy status` | Afficher l'état du proxy |
| `claudex dashboard` | Lancer le tableau de bord TUI |
| `claudex config show [--raw] [--json]` | Afficher la configuration chargée |
| `claudex config init [--yaml]` | Créer une configuration dans le répertoire courant |
| `claudex config edit [--global]` | Ouvrir la configuration dans $EDITOR |
| `claudex config validate [--connectivity]` | Valider la configuration |
| `claudex config get <key>` | Obtenir une valeur de configuration |
| `claudex config set <key> <value>` | Définir une valeur de configuration |
| `claudex config export --format <fmt>` | Exporter la configuration (json/toml/yaml) |
| `claudex update [--check]` | Mise à jour automatique depuis GitHub Releases |
| `claudex auth login <provider>` | Connexion OAuth |
| `claudex auth login github --enterprise-url <domain>` | GitHub Enterprise Copilot |
| `claudex auth status` | Afficher l'état des tokens OAuth |
| `claudex auth logout <profile>` | Supprimer un token OAuth |
| `claudex auth refresh <profile>` | Forcer le renouvellement du token OAuth |
| `claudex sets add <source> [--global]` | Installer un ensemble de configuration |
| `claudex sets remove <name>` | Supprimer un ensemble de configuration |
| `claudex sets list [--global]` | Lister les ensembles installés |
| `claudex sets update [name]` | Mettre à jour les ensembles vers la dernière version |

## Abonnements OAuth

Utilisez vos abonnements existants à la place des clés API :

```bash
# Abonnement ChatGPT (détecte automatiquement les identifiants Codex CLI existants)
claudex auth login chatgpt --profile codex-sub

# ChatGPT avec connexion navigateur forcée
claudex auth login chatgpt --profile codex-sub --force

# ChatGPT sans interface graphique (SSH/sans navigateur)
claudex auth login chatgpt --profile codex-sub --force --headless

# GitHub Copilot
claudex auth login github --profile copilot

# GitHub Copilot Enterprise
claudex auth login github --profile copilot-ent --enterprise-url company.ghe.com

# GitLab Duo (lit la variable d'environnement GITLAB_TOKEN)
claudex auth login gitlab --profile gitlab-duo

# Vérifier le statut
claudex auth status

# Lancer avec l'abonnement
claudex run codex-sub
```

Pris en charge : `claude`, `chatgpt`/`openai`, `google`, `qwen`, `kimi`, `github`/`copilot`, `gitlab`

## Correspondance des slots de modèles

Associez le sélecteur `/model` de Claude Code (haiku/sonnet/opus) aux modèles de n'importe quel fournisseur :

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

## Architecture

```
src/
├── main.rs
├── cli.rs
├── update.rs
├── util.rs
├── config/
│   ├── mod.rs          # Découverte + analyse de configuration (figment)
│   ├── cmd.rs          # Sous-commandes config get/set/export/validate
│   └── profile.rs      # CRUD des profils + test de connectivité
├── process/
│   ├── mod.rs
│   ├── launch.rs       # Lanceur de processus Claude
│   └── daemon.rs       # Fichier PID + gestion des processus
├── oauth/
│   ├── mod.rs          # AuthType, OAuthProvider, OAuthToken
│   ├── source.rs       # Couche 1 : sources d'identifiants (env/fichier/keyring)
│   ├── exchange.rs     # Couche 2 : échange de tokens (PKCE/device code/refresh)
│   ├── manager.rs      # Couche 3 : cache + déduplication concurrente + retry 401
│   ├── handler.rs      # Trait OAuthProviderHandler
│   ├── providers.rs    # Logique CLI de connexion/refresh/statut
│   ├── server.rs       # Serveur de rappel OAuth + interrogation device code
│   └── token.rs        # Réexportations
├── proxy/
│   ├── mod.rs          # Serveur Axum + ProxyState
│   ├── handler.rs      # Routage des requêtes + disjoncteur + retry 401
│   ├── adapter/        # Adaptateurs spécifiques aux fournisseurs
│   │   ├── mod.rs      # Trait ProviderAdapter + factory
│   │   ├── direct.rs   # DirectAnthropic (transfert direct)
│   │   ├── chat_completions.rs  # OpenAI Chat Completions
│   │   └── responses.rs         # OpenAI Responses API
│   ├── translate/      # Traduction de protocoles
│   │   ├── chat_completions.rs
│   │   ├── chat_completions_stream.rs
│   │   ├── responses.rs
│   │   └── responses_stream.rs
│   ├── context_engine.rs
│   ├── fallback.rs     # Disjoncteur
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
├── sets/               # Gestion des ensembles de configuration
│   ├── mod.rs
│   ├── schema.rs
│   ├── source.rs
│   ├── install.rs
│   ├── lock.rs
│   ├── conflict.rs
│   └── mcp.rs
├── terminal/           # Détection du terminal + hyperliens
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

## Licence

[MIT](./LICENSE)
