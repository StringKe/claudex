use std::collections::HashMap;
use std::io::{self, Write};
use std::time::{Duration, Instant};

use anyhow::{bail, Result};
use reqwest::Client;

use crate::config::{ClaudexConfig, ProfileConfig, ProfileModels, ProviderType};
use crate::oauth::AuthType;

pub async fn list_profiles(config: &ClaudexConfig) {
    if config.profiles.is_empty() {
        println!("No profiles configured. Add one with: claudex profile add");
        return;
    }
    println!(
        "{:<16} {:<20} {:<12} {:<30}",
        "NAME", "MODEL", "TYPE", "BASE_URL"
    );
    println!("{}", "-".repeat(78));
    for p in &config.profiles {
        let type_str = match p.provider_type {
            ProviderType::DirectAnthropic => "Anthropic",
            ProviderType::OpenAICompatible => "OpenAI",
            ProviderType::OpenAIResponses => "Responses",
        };
        let status = if p.enabled { "" } else { " (disabled)" };
        println!(
            "{:<16} {:<20} {:<12} {:<30}{}",
            p.name, p.default_model, type_str, p.base_url, status
        );
    }
}

pub async fn show_profile(config: &ClaudexConfig, name: &str) -> Result<()> {
    let profile = config
        .find_profile(name)
        .ok_or_else(|| anyhow::anyhow!("profile '{}' not found", name))?;
    println!("Name:           {}", profile.name);
    println!("Provider:       {:?}", profile.provider_type);
    println!("Base URL:       {}", profile.base_url);
    println!("Default Model:  {}", profile.default_model);
    println!("Enabled:        {}", profile.enabled);
    println!("Priority:       {}", profile.priority);
    if !profile.backup_providers.is_empty() {
        println!("Backups:        {}", profile.backup_providers.join(", "));
    }
    if !profile.custom_headers.is_empty() {
        println!("Custom Headers: {:?}", profile.custom_headers);
    }
    Ok(())
}

pub async fn test_profile(config: &ClaudexConfig, name: &str) -> Result<()> {
    if name == "all" {
        for p in &config.profiles {
            if p.enabled {
                print!("Testing {}... ", p.name);
                match test_connectivity(p).await {
                    Ok(latency) => println!("OK ({latency}ms)"),
                    Err(e) => println!("FAIL: {e}"),
                }
            }
        }
        return Ok(());
    }

    let profile = config
        .find_profile(name)
        .ok_or_else(|| anyhow::anyhow!("profile '{}' not found", name))?;
    print!("Testing {}... ", profile.name);
    match test_connectivity(profile).await {
        Ok(latency) => {
            println!("OK ({latency}ms)");
            Ok(())
        }
        Err(e) => {
            println!("FAIL: {e}");
            bail!("connectivity test failed")
        }
    }
}

pub async fn test_connectivity(profile: &ProfileConfig) -> Result<u128> {
    let client = Client::builder().timeout(Duration::from_secs(10)).build()?;

    let start = Instant::now();

    let url = match profile.provider_type {
        ProviderType::DirectAnthropic => {
            format!("{}/v1/models", profile.base_url.trim_end_matches('/'))
        }
        ProviderType::OpenAICompatible => {
            format!("{}/models", profile.base_url.trim_end_matches('/'))
        }
        ProviderType::OpenAIResponses => {
            // Responses API 没有 /models 端点，直接发一个轻量请求验证连通性
            format!("{}/models", profile.base_url.trim_end_matches('/'))
        }
    };

    let mut req = client.get(&url);
    if !profile.api_key.is_empty() {
        match profile.provider_type {
            ProviderType::DirectAnthropic => {
                req = req.header("x-api-key", &profile.api_key);
                req = req.header("anthropic-version", "2023-06-01");
            }
            ProviderType::OpenAICompatible | ProviderType::OpenAIResponses => {
                req = req.header("Authorization", format!("Bearer {}", profile.api_key));
            }
        }
    }

    let resp = req.send().await?;
    let latency = start.elapsed().as_millis();

    if !resp.status().is_success() {
        bail!("HTTP {}", resp.status());
    }

    Ok(latency)
}

pub fn add_profile(config: &mut ClaudexConfig, profile: ProfileConfig) -> Result<()> {
    if config.find_profile(&profile.name).is_some() {
        bail!("profile '{}' already exists", profile.name);
    }
    config.profiles.push(profile);
    config.save()?;
    Ok(())
}

pub fn remove_profile(config: &mut ClaudexConfig, name: &str) -> Result<()> {
    let idx = config
        .profiles
        .iter()
        .position(|p| p.name == name)
        .ok_or_else(|| anyhow::anyhow!("profile '{}' not found", name))?;
    config.profiles.remove(idx);
    config.save()?;
    println!("Removed profile '{name}'");
    Ok(())
}

/// Interactive profile creation via stdin prompts
pub async fn interactive_add(config: &mut ClaudexConfig) -> Result<()> {
    println!("=== Add New Profile ===\n");

    // 1. Profile name
    let name = prompt_input("Profile name")?;
    if name.is_empty() {
        bail!("profile name cannot be empty");
    }
    if config.find_profile(&name).is_some() {
        bail!("profile '{}' already exists", name);
    }

    // 2. Provider type
    println!("\nProvider type:");
    println!("  1) DirectAnthropic  (Anthropic, MiniMax, OpenRouter)");
    println!("  2) OpenAICompatible (Grok, OpenAI, DeepSeek, Kimi, GLM, Ollama)");
    println!("  3) OpenAIResponses  (ChatGPT/Codex subscription)");
    let choice = prompt_input("Select [1/2/3]")?;
    let provider_type = match choice.as_str() {
        "1" => ProviderType::DirectAnthropic,
        "2" => ProviderType::OpenAICompatible,
        "3" => ProviderType::OpenAIResponses,
        _ => {
            println!("Invalid choice, defaulting to OpenAICompatible");
            ProviderType::OpenAICompatible
        }
    };

    // 3. Base URL
    let presets = match provider_type {
        ProviderType::DirectAnthropic => vec![
            ("Anthropic", "https://api.anthropic.com"),
            ("MiniMax", "https://api.minimax.io/anthropic"),
            ("OpenRouter", "https://openrouter.ai/api"),
        ],
        ProviderType::OpenAICompatible => vec![
            ("Grok/xAI", "https://api.x.ai/v1"),
            ("OpenAI", "https://api.openai.com/v1"),
            ("DeepSeek", "https://api.deepseek.com"),
            ("Kimi", "https://api.moonshot.cn/v1"),
            ("GLM", "https://open.bigmodel.cn/api/paas/v4"),
            ("Ollama", "http://localhost:11434/v1"),
        ],
        ProviderType::OpenAIResponses => {
            vec![("ChatGPT/Codex", "https://chatgpt.com/backend-api/codex")]
        }
    };

    println!("\nBase URL presets:");
    for (i, (label, url)) in presets.iter().enumerate() {
        println!("  {}) {} ({})", i + 1, label, url);
    }
    println!("  or enter a custom URL");

    let url_input = prompt_input("Base URL")?;
    let base_url = if let Ok(idx) = url_input.parse::<usize>() {
        if idx >= 1 && idx <= presets.len() {
            presets[idx - 1].1.to_string()
        } else {
            url_input
        }
    } else if url_input.is_empty() {
        presets[0].1.to_string()
    } else {
        url_input
    };

    // 4. API Key
    let api_key = prompt_input("API Key (leave empty for none)")?;

    // 5. Optionally store in keyring
    let api_key_keyring = if !api_key.is_empty() {
        let store = prompt_input("Store API key in system keyring? [y/N]")?;
        if store.eq_ignore_ascii_case("y") {
            let entry_name = format!("{name}-api-key");
            match keyring::Entry::new("claudex", &entry_name) {
                Ok(entry) => {
                    if let Err(e) = entry.set_password(&api_key) {
                        println!("Warning: failed to store in keyring: {e}");
                        None
                    } else {
                        println!("Stored in keyring as '{entry_name}'");
                        Some(entry_name)
                    }
                }
                Err(e) => {
                    println!("Warning: keyring not available: {e}");
                    None
                }
            }
        } else {
            None
        }
    } else {
        None
    };

    // 6. Default model
    let default_model = prompt_input("Default model")?;
    if default_model.is_empty() {
        bail!("model name cannot be empty");
    }

    // 7. Backup providers (optional)
    let backup_input = prompt_input("Backup providers (comma-separated, or empty)")?;
    let backup_providers: Vec<String> = if backup_input.is_empty() {
        Vec::new()
    } else {
        backup_input
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    };

    let profile = ProfileConfig {
        name: name.clone(),
        provider_type,
        base_url,
        api_key: if api_key_keyring.is_some() {
            String::new()
        } else {
            api_key
        },
        api_key_keyring,
        default_model,
        backup_providers,
        custom_headers: HashMap::new(),
        extra_env: HashMap::new(),
        priority: 100,
        enabled: true,
        auth_type: AuthType::ApiKey,
        oauth_provider: None,
        models: ProfileModels::default(),
        max_tokens: None,
        strip_params: crate::config::StripParams::default(),
    };

    // Test connectivity
    print!("\nTesting connectivity... ");
    io::stdout().flush()?;
    match test_connectivity(&profile).await {
        Ok(latency) => println!("OK ({latency}ms)"),
        Err(e) => {
            println!("FAIL: {e}");
            let proceed = prompt_input("Add anyway? [y/N]")?;
            if !proceed.eq_ignore_ascii_case("y") {
                bail!("aborted");
            }
        }
    }

    add_profile(config, profile)?;
    println!("\nProfile '{name}' added successfully!");
    Ok(())
}

fn prompt_input(label: &str) -> Result<String> {
    print!("{label}: ");
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    Ok(input.trim().to_string())
}
