use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::context::ContextEngineConfig;
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
    #[serde(skip)]
    pub config_source: Option<PathBuf>,
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
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ProviderType {
    DirectAnthropic,
    OpenAICompatible,
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
    "info".to_string()
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
    /// Legacy config path (global)
    pub fn config_path() -> Result<PathBuf> {
        let config_dir = dirs::config_dir()
            .context("cannot determine config directory")?
            .join("claudex");
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

        // 5. Global config (~/.config/claudex/config.toml)
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

    /// Create a default global config with the example template
    fn create_default_global() -> Result<Self> {
        let path = Self::config_path()?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let example = include_str!("../config.example.toml");
        std::fs::write(&path, example)?;
        println!("Created default config at: {}", path.display());
        println!("Edit it to add your API keys and profiles.");

        let mut config: ClaudexConfig =
            toml::from_str(example).context("failed to parse default config")?;
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
        for profile in &mut self.profiles {
            if let Some(ref keyring_entry) = profile.api_key_keyring {
                if profile.api_key.is_empty() {
                    match keyring::Entry::new("claudex", keyring_entry) {
                        Ok(entry) => match entry.get_password() {
                            Ok(key) => profile.api_key = key,
                            Err(e) => {
                                tracing::warn!(
                                    "failed to read keyring entry '{}': {}",
                                    keyring_entry,
                                    e
                                );
                            }
                        },
                        Err(e) => {
                            tracing::warn!(
                                "failed to create keyring entry '{}': {}",
                                keyring_entry,
                                e
                            );
                        }
                    }
                }
            }
        }
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
        }
    }

    #[test]
    fn test_default_values() {
        let config = ClaudexConfig::default();
        assert_eq!(config.claude_binary, "claude");
        assert_eq!(config.proxy_port, 13456);
        assert_eq!(config.proxy_host, "127.0.0.1");
        assert_eq!(config.log_level, "info");
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
        assert_eq!(config.log_level, "info");
        assert!(config.profiles[0].enabled);
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
        assert_eq!(config.context.compression.model, "qwen/qwen-2.5-7b-instruct");
        assert!(!config.context.rag.enabled);
        assert_eq!(config.context.rag.profile, "openrouter");
        assert_eq!(config.context.rag.model, "openai/text-embedding-3-small");
    }
}
