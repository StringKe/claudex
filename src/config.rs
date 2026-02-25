use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::context::ContextEngineConfig;
use crate::oauth::{AuthType, OAuthProvider};
use crate::router::RouterConfig;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudexConfig {
    #[serde(default = "default_claude_binary")]
    pub claude_binary: String,
    #[serde(default = "default_proxy_port")]
    pub proxy_port: u16,
    #[serde(default = "default_proxy_host")]
    pub proxy_host: String,
    #[serde(default = "default_log_level")]
    pub log_level: String,
    #[serde(default)]
    pub profiles: Vec<ProfileConfig>,
    #[serde(default)]
    pub model_aliases: HashMap<String, String>,
    #[serde(default)]
    pub router: RouterConfig,
    #[serde(default)]
    pub context: ContextEngineConfig,
    #[serde(default)]
    pub hyperlinks: HyperlinksConfig,
    #[serde(skip)]
    pub config_source: Option<PathBuf>,
}

/// Hyperlinks mode: "auto" detects terminal support, true/false force on/off.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(from = "HyperlinksRaw")]
pub enum HyperlinksConfig {
    #[default]
    Auto,
    Enabled,
    Disabled,
}

/// Intermediate type for deserializing hyperlinks from TOML (string or bool).
#[derive(Deserialize)]
#[serde(untagged)]
enum HyperlinksRaw {
    Bool(bool),
    Str(String),
}

impl From<HyperlinksRaw> for HyperlinksConfig {
    fn from(raw: HyperlinksRaw) -> Self {
        match raw {
            HyperlinksRaw::Bool(true) => HyperlinksConfig::Enabled,
            HyperlinksRaw::Bool(false) => HyperlinksConfig::Disabled,
            HyperlinksRaw::Str(s) => match s.to_lowercase().as_str() {
                "auto" => HyperlinksConfig::Auto,
                "true" | "on" | "enabled" => HyperlinksConfig::Enabled,
                "false" | "off" | "disabled" => HyperlinksConfig::Disabled,
                _ => HyperlinksConfig::Auto,
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileConfig {
    pub name: String,
    #[serde(default = "default_provider_type")]
    pub provider_type: ProviderType,
    pub base_url: String,
    #[serde(default)]
    pub api_key: String,
    #[serde(default)]
    pub api_key_keyring: Option<String>,
    pub default_model: String,
    #[serde(default)]
    pub backup_providers: Vec<String>,
    #[serde(default)]
    pub custom_headers: HashMap<String, String>,
    #[serde(default)]
    pub extra_env: HashMap<String, String>,
    #[serde(default = "default_priority")]
    pub priority: u32,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[serde(default)]
    pub auth_type: AuthType,
    #[serde(default)]
    pub oauth_provider: Option<OAuthProvider>,
    /// 模型 slot 映射（对应 Claude Code 的 /model 切换）
    #[serde(default)]
    pub models: ProfileModels,
    /// 最大输出 token 数上限（可选，用于限制转发给 provider 的 max_tokens）
    #[serde(default)]
    pub max_tokens: Option<u64>,
}

/// Claude Code 模型 slot 映射
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProfileModels {
    pub haiku: Option<String>,
    pub sonnet: Option<String>,
    pub opus: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ProviderType {
    DirectAnthropic,
    OpenAICompatible,
    OpenAIResponses,
}

fn default_claude_binary() -> String {
    "claude".to_string()
}

fn default_proxy_port() -> u16 {
    13456
}

fn default_proxy_host() -> String {
    "127.0.0.1".to_string()
}

fn default_log_level() -> String {
    "debug".to_string()
}

fn default_provider_type() -> ProviderType {
    ProviderType::DirectAnthropic
}

fn default_priority() -> u32 {
    100
}

fn default_enabled() -> bool {
    true
}

/// Config file names to search for
const CONFIG_FILE_NAMES: &[&str] = &["claudex.toml"];
const CONFIG_DIR_NAMES: &[(&str, &str)] = &[(".claudex", "config.toml")];
const MAX_PARENT_TRAVERSAL: usize = 10;

impl ClaudexConfig {
    /// Global config path: ~/.config/claudex/config.toml (XDG-style, all platforms)
    pub fn config_path() -> Result<PathBuf> {
        let home = dirs::home_dir().context("cannot determine home directory")?;
        let config_dir = home.join(".config").join("claudex");
        Ok(config_dir.join("config.toml"))
    }

    /// Discover config file with priority-based search
    pub fn discover_config() -> Result<(Self, PathBuf)> {
        let mut searched = Vec::new();

        // 1. $CLAUDEX_CONFIG environment variable
        if let Ok(env_path) = std::env::var("CLAUDEX_CONFIG") {
            let path = PathBuf::from(&env_path);
            searched.push(path.clone());
            if path.exists() {
                let config = Self::load_from(&path)?;
                return Ok((config, path));
            }
        }

        // 2-4. Current directory and parent traversal
        if let Ok(cwd) = std::env::current_dir() {
            let mut dir = Some(cwd.as_path());
            let mut depth = 0;

            while let Some(current) = dir {
                if depth > MAX_PARENT_TRAVERSAL {
                    break;
                }

                // Check claudex.toml in this directory
                for name in CONFIG_FILE_NAMES {
                    let path = current.join(name);
                    searched.push(path.clone());
                    if path.exists() {
                        let config = Self::load_from(&path)?;
                        return Ok((config, path));
                    }
                }

                // Check .claudex/config.toml in this directory
                for (dir_name, file_name) in CONFIG_DIR_NAMES {
                    let path = current.join(dir_name).join(file_name);
                    searched.push(path.clone());
                    if path.exists() {
                        let config = Self::load_from(&path)?;
                        return Ok((config, path));
                    }
                }

                dir = current.parent();
                depth += 1;
            }
        }

        // 5. Global: ~/.config/claudex/config.toml
        let global_path = Self::config_path()?;
        searched.push(global_path.clone());
        if global_path.exists() {
            let config = Self::load_from(&global_path)?;
            return Ok((config, global_path));
        }

        // Nothing found: create default global config
        let default_config = Self::create_default_global()?;
        Ok((default_config, global_path))
    }

    /// Print search results for diagnostics
    pub fn print_discovery_info(source: &Path, searched: &[PathBuf]) {
        println!("Config loaded from: {}", source.display());
        println!("\nSearch order:");
        for (i, path) in searched.iter().enumerate() {
            let exists = path.exists();
            let marker = if exists { "✓" } else { " " };
            println!("  {marker} {}. {}", i + 1, path.display());
        }
    }

    /// Create a minimal default global config (no profiles, user adds their own)
    fn create_default_global() -> Result<Self> {
        let path = Self::config_path()?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let minimal = r#"# Claudex Configuration
# See config.example.toml for full reference:
#   https://github.com/StringKe/claudex/blob/main/config.example.toml

proxy_port = 13456
proxy_host = "127.0.0.1"
log_level = "info"
hyperlinks = "auto"

[model_aliases]

# Add your profiles below. Example:
#
# [[profiles]]
# name = "openrouter"
# provider_type = "OpenAICompatible"
# base_url = "https://openrouter.ai/api/v1"
# api_key = "sk-or-..."
# default_model = "anthropic/claude-sonnet-4"
# enabled = true
# priority = 100

[router]
enabled = false

[context.compression]
enabled = false

[context.sharing]
enabled = false

[context.rag]
enabled = false
"#;

        std::fs::write(&path, minimal)?;
        println!("Created default config at: {}", path.display());
        println!("Edit it to add your API keys and profiles.");
        println!("Full example: https://github.com/StringKe/claudex/blob/main/config.example.toml");

        let mut config: ClaudexConfig =
            toml::from_str(minimal).context("failed to parse default config")?;
        config.config_source = Some(path);
        Ok(config)
    }

    /// Initialize config in the current directory
    pub fn init_local() -> Result<PathBuf> {
        let path = std::env::current_dir()?.join("claudex.toml");
        if path.exists() {
            anyhow::bail!("claudex.toml already exists in current directory");
        }
        let example = include_str!("../config.example.toml");
        std::fs::write(&path, example)?;
        println!("Created: {}", path.display());
        Ok(path)
    }

    fn load_from(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("failed to read config: {}", path.display()))?;
        let mut config: ClaudexConfig =
            toml::from_str(&content).with_context(|| "failed to parse config.toml")?;
        config.resolve_api_keys()?;
        config.config_source = Some(path.to_path_buf());
        Ok(config)
    }

    pub fn load() -> Result<Self> {
        let (config, _path) = Self::discover_config()?;
        Ok(config)
    }

    pub fn save(&self) -> Result<()> {
        let path = self
            .config_source
            .clone()
            .or_else(|| Self::config_path().ok())
            .context("cannot determine config save path")?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = toml::to_string_pretty(self)?;
        std::fs::write(&path, content)?;
        Ok(())
    }

    fn resolve_api_keys(&mut self) -> Result<()> {
        // API key 直接从 config 的 api_key 字段读取，不自动访问 keyring。
        // OAuth token 只在用户显式调用 `claudex auth` 命令时才从 keyring 加载。
        // 这样避免 macOS Keychain 反复弹出授权弹窗。
        Ok(())
    }

    pub fn find_profile(&self, name: &str) -> Option<&ProfileConfig> {
        self.profiles.iter().find(|p| p.name == name)
    }

    pub fn find_profile_mut(&mut self, name: &str) -> Option<&mut ProfileConfig> {
        self.profiles.iter_mut().find(|p| p.name == name)
    }

    pub fn enabled_profiles(&self) -> Vec<&ProfileConfig> {
        self.profiles.iter().filter(|p| p.enabled).collect()
    }

    pub fn resolve_model(&self, model: &str) -> String {
        self.model_aliases
            .get(model)
            .cloned()
            .unwrap_or_else(|| model.to_string())
    }
}

impl Default for ClaudexConfig {
    fn default() -> Self {
        Self {
            claude_binary: default_claude_binary(),
            proxy_port: default_proxy_port(),
            proxy_host: default_proxy_host(),
            log_level: default_log_level(),
            profiles: Vec::new(),
            model_aliases: HashMap::new(),
            router: RouterConfig::default(),
            context: ContextEngineConfig::default(),
            hyperlinks: HyperlinksConfig::default(),
            config_source: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_profile(name: &str, enabled: bool) -> ProfileConfig {
        ProfileConfig {
            name: name.to_string(),
            provider_type: ProviderType::OpenAICompatible,
            base_url: "http://localhost".to_string(),
            api_key: String::new(),
            api_key_keyring: None,
            default_model: "test-model".to_string(),
            backup_providers: Vec::new(),
            custom_headers: HashMap::new(),
            extra_env: HashMap::new(),
            priority: 100,
            enabled,
            auth_type: AuthType::default(),
            oauth_provider: None,
            models: ProfileModels::default(),
            max_tokens: None,
        }
    }

    #[test]
    fn test_default_values() {
        let config = ClaudexConfig::default();
        assert_eq!(config.claude_binary, "claude");
        assert_eq!(config.proxy_port, 13456);
        assert_eq!(config.proxy_host, "127.0.0.1");
        assert_eq!(config.log_level, "debug");
        assert!(config.profiles.is_empty());
        assert!(config.model_aliases.is_empty());
    }

    #[test]
    fn test_find_profile() {
        let mut config = ClaudexConfig::default();
        config.profiles.push(make_profile("grok", true));
        config.profiles.push(make_profile("deepseek", true));

        assert!(config.find_profile("grok").is_some());
        assert_eq!(config.find_profile("grok").unwrap().name, "grok");
        assert!(config.find_profile("nonexistent").is_none());
    }

    #[test]
    fn test_find_profile_mut() {
        let mut config = ClaudexConfig::default();
        config.profiles.push(make_profile("grok", true));

        let p = config.find_profile_mut("grok").unwrap();
        p.enabled = false;
        assert!(!config.find_profile("grok").unwrap().enabled);
    }

    #[test]
    fn test_enabled_profiles() {
        let mut config = ClaudexConfig::default();
        config.profiles.push(make_profile("a", true));
        config.profiles.push(make_profile("b", false));
        config.profiles.push(make_profile("c", true));

        let enabled = config.enabled_profiles();
        assert_eq!(enabled.len(), 2);
        assert_eq!(enabled[0].name, "a");
        assert_eq!(enabled[1].name, "c");
    }

    #[test]
    fn test_resolve_model_alias() {
        let mut config = ClaudexConfig::default();
        config
            .model_aliases
            .insert("grok3".to_string(), "grok-3-beta".to_string());

        assert_eq!(config.resolve_model("grok3"), "grok-3-beta");
    }

    #[test]
    fn test_resolve_model_no_alias() {
        let config = ClaudexConfig::default();
        assert_eq!(config.resolve_model("custom-model"), "custom-model");
    }

    #[test]
    fn test_parse_minimal_toml() {
        let toml_str = r#"
            proxy_port = 9999
            [[profiles]]
            name = "test"
            base_url = "http://localhost:8080"
            default_model = "gpt-4"
        "#;
        let config: ClaudexConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.proxy_port, 9999);
        assert_eq!(config.profiles.len(), 1);
        assert_eq!(config.profiles[0].name, "test");
        assert_eq!(config.profiles[0].default_model, "gpt-4");
        // Check defaults are applied
        assert_eq!(config.proxy_host, "127.0.0.1");
        assert_eq!(config.log_level, "debug");
        assert!(config.profiles[0].enabled);
    }

    #[test]
    fn test_default_hyperlinks_is_auto() {
        let config = ClaudexConfig::default();
        assert_eq!(config.hyperlinks, HyperlinksConfig::Auto);
    }

    #[test]
    fn test_hyperlinks_parse_auto_string() {
        let toml_str = r#"hyperlinks = "auto""#;
        let config: ClaudexConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.hyperlinks, HyperlinksConfig::Auto);
    }

    #[test]
    fn test_hyperlinks_parse_true_bool() {
        let toml_str = "hyperlinks = true";
        let config: ClaudexConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.hyperlinks, HyperlinksConfig::Enabled);
    }

    #[test]
    fn test_hyperlinks_parse_false_bool() {
        let toml_str = "hyperlinks = false";
        let config: ClaudexConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.hyperlinks, HyperlinksConfig::Disabled);
    }

    #[test]
    fn test_hyperlinks_parse_true_string() {
        let toml_str = r#"hyperlinks = "true""#;
        let config: ClaudexConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.hyperlinks, HyperlinksConfig::Enabled);
    }

    #[test]
    fn test_hyperlinks_parse_false_string() {
        let toml_str = r#"hyperlinks = "false""#;
        let config: ClaudexConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.hyperlinks, HyperlinksConfig::Disabled);
    }

    #[test]
    fn test_hyperlinks_parse_on_string() {
        let toml_str = r#"hyperlinks = "on""#;
        let config: ClaudexConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.hyperlinks, HyperlinksConfig::Enabled);
    }

    #[test]
    fn test_hyperlinks_parse_off_string() {
        let toml_str = r#"hyperlinks = "off""#;
        let config: ClaudexConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.hyperlinks, HyperlinksConfig::Disabled);
    }

    #[test]
    fn test_hyperlinks_parse_unknown_defaults_to_auto() {
        let toml_str = r#"hyperlinks = "whatever""#;
        let config: ClaudexConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.hyperlinks, HyperlinksConfig::Auto);
    }

    #[test]
    fn test_hyperlinks_omitted_defaults_to_auto() {
        let toml_str = "proxy_port = 8080";
        let config: ClaudexConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.hyperlinks, HyperlinksConfig::Auto);
    }

    #[test]
    fn test_parse_oauth_profile() {
        let toml_str = r#"
            [[profiles]]
            name = "chatgpt-sub"
            provider_type = "OpenAICompatible"
            base_url = "https://api.openai.com/v1"
            default_model = "gpt-4o"
            auth_type = "oauth"
            oauth_provider = "openai"
        "#;
        let config: ClaudexConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.profiles.len(), 1);
        assert_eq!(config.profiles[0].auth_type, AuthType::OAuth);
        assert_eq!(
            config.profiles[0].oauth_provider,
            Some(OAuthProvider::Openai)
        );
    }

    #[test]
    fn test_parse_no_auth_type_defaults_to_api_key() {
        let toml_str = r#"
            [[profiles]]
            name = "test"
            base_url = "http://localhost"
            default_model = "gpt-4"
        "#;
        let config: ClaudexConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.profiles[0].auth_type, AuthType::ApiKey);
        assert_eq!(config.profiles[0].oauth_provider, None);
    }

    #[test]
    fn test_existing_config_backward_compat() {
        let toml_str = r#"
            [[profiles]]
            name = "grok"
            provider_type = "OpenAICompatible"
            base_url = "https://api.x.ai/v1"
            api_key = "sk-xxx"
            default_model = "grok-3-beta"
            enabled = true
            priority = 100
        "#;
        let config: ClaudexConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.profiles[0].auth_type, AuthType::ApiKey);
        assert_eq!(config.profiles[0].api_key, "sk-xxx");
    }

    #[test]
    fn test_parse_mixed_auth_type_profiles() {
        let toml_str = r#"
            [[profiles]]
            name = "api-profile"
            base_url = "https://api.x.ai/v1"
            api_key = "sk-xxx"
            default_model = "grok-3"

            [[profiles]]
            name = "oauth-profile"
            base_url = "https://api.openai.com/v1"
            default_model = "gpt-4o"
            auth_type = "oauth"
            oauth_provider = "openai"
        "#;
        let config: ClaudexConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.profiles.len(), 2);
        assert_eq!(config.profiles[0].auth_type, AuthType::ApiKey);
        assert!(config.profiles[0].oauth_provider.is_none());
        assert_eq!(config.profiles[1].auth_type, AuthType::OAuth);
        assert_eq!(
            config.profiles[1].oauth_provider,
            Some(OAuthProvider::Openai)
        );
    }

    #[test]
    fn test_parse_all_oauth_providers() {
        let providers = [
            ("claude", "DirectAnthropic"),
            ("openai", "OpenAIResponses"),
            ("google", "OpenAICompatible"),
            ("qwen", "OpenAICompatible"),
            ("kimi", "OpenAICompatible"),
            ("github", "OpenAICompatible"),
        ];
        for (provider_str, provider_type) in providers {
            let toml_str = format!(
                r#"
                [[profiles]]
                name = "test-{provider_str}"
                provider_type = "{provider_type}"
                base_url = "http://localhost"
                default_model = "test"
                auth_type = "oauth"
                oauth_provider = "{provider_str}"
            "#
            );
            let config: ClaudexConfig = toml::from_str(&toml_str).unwrap();
            assert_eq!(
                config.profiles[0].auth_type,
                AuthType::OAuth,
                "failed for {provider_str}"
            );
            assert!(
                config.profiles[0].oauth_provider.is_some(),
                "oauth_provider missing for {provider_str}"
            );
        }
    }

    #[test]
    fn test_oauth_profile_api_key_defaults_empty() {
        let toml_str = r#"
            [[profiles]]
            name = "oauth-no-key"
            base_url = "http://localhost"
            default_model = "test"
            auth_type = "oauth"
            oauth_provider = "openai"
        "#;
        let config: ClaudexConfig = toml::from_str(toml_str).unwrap();
        assert!(config.profiles[0].api_key.is_empty());
        assert!(config.profiles[0].api_key_keyring.is_none());
    }

    #[test]
    fn test_config_example_toml_parses() {
        let example = include_str!("../config.example.toml");
        let config: ClaudexConfig = toml::from_str(example).unwrap();
        assert!(!config.profiles.is_empty());
        // 确认 OAuth profiles 在其中
        let oauth_profiles: Vec<_> = config
            .profiles
            .iter()
            .filter(|p| p.auth_type == AuthType::OAuth)
            .collect();
        assert!(
            oauth_profiles.len() >= 3,
            "expected at least 3 OAuth profiles in example, got {}",
            oauth_profiles.len()
        );
    }

    #[test]
    fn test_parse_with_router_and_context() {
        let toml_str = r#"
            [router]
            enabled = true
            profile = "openrouter"
            model = "qwen/qwen-2.5-7b-instruct"
            [router.rules]
            code = "deepseek"
            default = "grok"

            [context.compression]
            enabled = true
            threshold_tokens = 10000
            profile = "openrouter"
            model = "qwen/qwen-2.5-7b-instruct"

            [context.rag]
            enabled = false
            profile = "openrouter"
            model = "openai/text-embedding-3-small"
        "#;
        let config: ClaudexConfig = toml::from_str(toml_str).unwrap();
        assert!(config.router.enabled);
        assert_eq!(config.router.profile, "openrouter");
        assert_eq!(config.router.model, "qwen/qwen-2.5-7b-instruct");
        assert_eq!(
            config.router.resolve_profile("code"),
            Some("deepseek".to_string())
        );
        assert!(config.context.compression.enabled);
        assert_eq!(config.context.compression.threshold_tokens, 10000);
        assert_eq!(config.context.compression.profile, "openrouter");
        assert_eq!(
            config.context.compression.model,
            "qwen/qwen-2.5-7b-instruct"
        );
        assert!(!config.context.rag.enabled);
        assert_eq!(config.context.rag.profile, "openrouter");
        assert_eq!(config.context.rag.model, "openai/text-embedding-3-small");
    }
}
