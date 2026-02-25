pub mod classifier;

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RouterConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub profile: String,
    #[serde(default)]
    pub model: String,
    #[serde(default)]
    pub rules: HashMap<String, String>,
}

impl RouterConfig {
    pub fn resolve_profile(&self, intent: &str) -> Option<String> {
        self.rules
            .get(intent)
            .or_else(|| self.rules.get("default"))
            .cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_exact_match() {
        let mut config = RouterConfig::default();
        config
            .rules
            .insert("code".to_string(), "deepseek".to_string());
        assert_eq!(config.resolve_profile("code"), Some("deepseek".to_string()));
    }

    #[test]
    fn test_resolve_fallback_to_default() {
        let mut config = RouterConfig::default();
        config
            .rules
            .insert("default".to_string(), "grok".to_string());
        assert_eq!(
            config.resolve_profile("unknown_intent"),
            Some("grok".to_string())
        );
    }

    #[test]
    fn test_resolve_no_match_no_default() {
        let config = RouterConfig::default();
        assert_eq!(config.resolve_profile("anything"), None);
    }

    #[test]
    fn test_defaults() {
        let config = RouterConfig::default();
        assert!(!config.enabled);
        assert!(config.profile.is_empty());
        assert!(config.model.is_empty());
        assert!(config.rules.is_empty());
    }
}
