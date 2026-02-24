use std::collections::HashMap;
use std::path::PathBuf;

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

impl ClaudexConfig {
    pub fn config_path() -> Result<PathBuf> {
        let config_dir = dirs::config_dir()
            .context("cannot determine config directory")?
            .join("claudex");
        Ok(config_dir.join("config.toml"))
    }

    pub fn load() -> Result<Self> {
        let path = Self::config_path()?;
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = std::fs::read_to_string(&path)
            .with_context(|| format!("failed to read config: {}", path.display()))?;
        let mut config: ClaudexConfig =
            toml::from_str(&content).with_context(|| "failed to parse config.toml")?;
        config.resolve_api_keys()?;
        Ok(config)
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::config_path()?;
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
                            tracing::warn!("failed to create keyring entry '{}': {}", keyring_entry, e);
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
        }
    }
}
