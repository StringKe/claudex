use anyhow::{bail, Result};
use reqwest::Client;
use std::time::{Duration, Instant};

use crate::config::{ClaudexConfig, ProfileConfig, ProviderType};

pub async fn list_profiles(config: &ClaudexConfig) {
    if config.profiles.is_empty() {
        println!("No profiles configured. Add one with: claudex profile add");
        return;
    }
    println!("{:<16} {:<20} {:<12} {:<30}", "NAME", "MODEL", "TYPE", "BASE_URL");
    println!("{}", "-".repeat(78));
    for p in &config.profiles {
        let type_str = match p.provider_type {
            ProviderType::DirectAnthropic => "Anthropic",
            ProviderType::OpenAICompatible => "OpenAI",
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
    let client = Client::builder()
        .timeout(Duration::from_secs(10))
        .build()?;

    let start = Instant::now();

    let url = match profile.provider_type {
        ProviderType::DirectAnthropic => format!("{}/v1/models", profile.base_url.trim_end_matches('/')),
        ProviderType::OpenAICompatible => format!("{}/models", profile.base_url.trim_end_matches('/')),
    };

    let mut req = client.get(&url);
    if !profile.api_key.is_empty() {
        match profile.provider_type {
            ProviderType::DirectAnthropic => {
                req = req.header("x-api-key", &profile.api_key);
                req = req.header("anthropic-version", "2023-06-01");
            }
            ProviderType::OpenAICompatible => {
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
