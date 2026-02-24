pub mod classifier;

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouterConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_classifier_url")]
    pub classifier_url: String,
    #[serde(default = "default_classifier_model")]
    pub classifier_model: String,
    #[serde(default)]
    pub classifier_api_key: String,
    #[serde(default)]
    pub rules: HashMap<String, String>,
}

fn default_classifier_url() -> String {
    "http://localhost:11434/v1".to_string()
}

fn default_classifier_model() -> String {
    "qwen2.5:3b".to_string()
}

impl Default for RouterConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            classifier_url: default_classifier_url(),
            classifier_model: default_classifier_model(),
            classifier_api_key: String::new(),
            rules: HashMap::new(),
        }
    }
}

impl RouterConfig {
    pub fn resolve_profile(&self, intent: &str) -> Option<String> {
        self.rules
            .get(intent)
            .or_else(|| self.rules.get("default"))
            .cloned()
    }
}
