use std::path::PathBuf;

use anyhow::{Context, Result};

use crate::cli::ConfigAction;
use crate::config::{ClaudexConfig, CONFIG_DIR_NAMES, CONFIG_FILE_NAMES, GLOBAL_CONFIG_NAMES};
use crate::oauth::AuthType;

pub async fn dispatch(action: Option<ConfigAction>, config: &mut ClaudexConfig) -> Result<()> {
    match action {
        None
        | Some(ConfigAction::Show {
            raw: false,
            json: false,
        }) => cmd_show(config, false, false),
        Some(ConfigAction::Show { raw, json }) => cmd_show(config, raw, json),
        Some(ConfigAction::Path) => cmd_path(config),
        Some(ConfigAction::Init { yaml }) => {
            ClaudexConfig::init_local(yaml)?;
            Ok(())
        }
        Some(ConfigAction::Recreate { force }) => cmd_recreate(config, force),
        Some(ConfigAction::Edit { global }) => cmd_edit(config, global),
        Some(ConfigAction::Validate { connectivity }) => cmd_validate(config, connectivity).await,
        Some(ConfigAction::Get { key }) => cmd_get(config, &key),
        Some(ConfigAction::Set { key, value }) => cmd_set(config, &key, &value),
        Some(ConfigAction::Export { format, output }) => cmd_export(config, &format, output),
    }
}

fn cmd_show(config: &ClaudexConfig, raw: bool, json: bool) -> Result<()> {
    if raw {
        if let Some(ref source) = config.config_source {
            let content = std::fs::read_to_string(source)
                .with_context(|| format!("cannot read {}", source.display()))?;
            print!("{content}");
        } else {
            println!("(no config file found)");
        }
        return Ok(());
    }
    if json {
        let json =
            serde_json::to_string_pretty(config).context("failed to serialize config to JSON")?;
        println!("{json}");
        return Ok(());
    }

    let source_display = config
        .config_source
        .as_ref()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "(default)".to_string());
    println!("Config loaded from: {source_display}");
    println!("Profiles: {}", config.profiles.len());
    println!("Proxy: {}:{}", config.proxy_host, config.proxy_port);
    println!(
        "Router: {}",
        if config.router.enabled {
            "enabled"
        } else {
            "disabled"
        }
    );
    println!("Context engine:");
    println!(
        "  Compression: {}",
        if config.context.compression.enabled {
            "enabled"
        } else {
            "disabled"
        }
    );
    println!(
        "  Sharing: {}",
        if config.context.sharing.enabled {
            "enabled"
        } else {
            "disabled"
        }
    );
    println!(
        "  RAG: {}",
        if config.context.rag.enabled {
            "enabled"
        } else {
            "disabled"
        }
    );
    Ok(())
}

fn cmd_path(config: &ClaudexConfig) -> Result<()> {
    let global_dir = ClaudexConfig::config_dir()?;
    let cwd = std::env::current_dir().unwrap_or_default();

    println!(
        "Active config: {}",
        config
            .config_source
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "(none)".to_string())
    );
    println!();
    println!("Search order:");
    let mut idx = 1;

    // Environment variable
    if let Ok(env_val) = std::env::var("CLAUDEX_CONFIG") {
        let exists = std::path::Path::new(&env_val).exists();
        let marker = if exists { "+" } else { " " };
        println!("  {marker} {idx}. $CLAUDEX_CONFIG = {env_val}");
        idx += 1;
    }

    // Project-level files in CWD
    for name in CONFIG_FILE_NAMES {
        let path = cwd.join(name);
        let marker = if path.exists() { "+" } else { " " };
        println!("  {marker} {idx}. {}", path.display());
        idx += 1;
    }
    for (dir_name, file_names) in CONFIG_DIR_NAMES {
        for file_name in *file_names {
            let path = cwd.join(dir_name).join(file_name);
            let marker = if path.exists() { "+" } else { " " };
            println!("  {marker} {idx}. {}", path.display());
            idx += 1;
        }
    }

    // Global config
    for name in GLOBAL_CONFIG_NAMES {
        let path = global_dir.join(name);
        let marker = if path.exists() { "+" } else { " " };
        println!("  {marker} {idx}. {}", path.display());
        idx += 1;
    }

    println!();
    println!("(+ = exists)");
    Ok(())
}

fn cmd_get(config: &ClaudexConfig, key: &str) -> Result<()> {
    let json = serde_json::to_value(config).context("failed to serialize config")?;
    let value = resolve_dot_path(&json, key).with_context(|| format!("key not found: {key}"))?;
    match value {
        serde_json::Value::String(s) => println!("{s}"),
        other => println!("{}", serde_json::to_string_pretty(&other)?),
    }
    Ok(())
}

fn resolve_dot_path<'a>(value: &'a serde_json::Value, path: &str) -> Option<&'a serde_json::Value> {
    let mut current = value;
    for segment in path.split('.') {
        if let Ok(idx) = segment.parse::<usize>() {
            current = current.get(idx)?;
        } else {
            current = current.get(segment)?;
        }
    }
    Some(current)
}

fn cmd_set(config: &mut ClaudexConfig, key: &str, value: &str) -> Result<()> {
    let mut json = serde_json::to_value(&*config).context("failed to serialize config")?;

    let parsed_value: serde_json::Value = serde_json::from_str(value)
        .unwrap_or_else(|_| serde_json::Value::String(value.to_string()));

    set_dot_path(&mut json, key, parsed_value).with_context(|| format!("cannot set key: {key}"))?;

    // Validate by deserializing back
    let new_config: ClaudexConfig =
        serde_json::from_value(json).context("invalid config after modification")?;

    // Preserve skip fields
    let source = config.config_source.clone();
    let format = config.config_format;
    *config = new_config;
    config.config_source = source;
    config.config_format = format;
    config.save()?;
    println!("Set {key} = {value}");
    Ok(())
}

fn set_dot_path(root: &mut serde_json::Value, path: &str, value: serde_json::Value) -> Result<()> {
    let segments: Vec<&str> = path.split('.').collect();
    let mut current = root;
    for (i, segment) in segments.iter().enumerate() {
        if i == segments.len() - 1 {
            // Last segment: set value
            if let Ok(idx) = segment.parse::<usize>() {
                let arr = current.as_array_mut().context("expected array")?;
                if idx < arr.len() {
                    arr[idx] = value;
                } else {
                    anyhow::bail!("array index {idx} out of bounds (len {})", arr.len());
                }
            } else {
                let obj = current.as_object_mut().context("expected object")?;
                obj.insert(segment.to_string(), value);
            }
            return Ok(());
        }
        // Navigate deeper
        if let Ok(idx) = segment.parse::<usize>() {
            current = current
                .get_mut(idx)
                .with_context(|| format!("array index {idx} not found"))?;
        } else {
            current = current
                .get_mut(*segment)
                .with_context(|| format!("key '{segment}' not found"))?;
        }
    }
    Ok(())
}

fn cmd_export(config: &ClaudexConfig, format: &str, output: Option<PathBuf>) -> Result<()> {
    let content = match format {
        "json" => serde_json::to_string_pretty(config).context("failed to serialize to JSON")?,
        "toml" => toml::to_string_pretty(config).context("failed to serialize to TOML")?,
        "yaml" | "yml" => serde_yaml::to_string(config).context("failed to serialize to YAML")?,
        _ => anyhow::bail!("unsupported format: {format} (use json, toml, yaml)"),
    };

    if let Some(path) = output {
        std::fs::write(&path, &content)
            .with_context(|| format!("cannot write to {}", path.display()))?;
        println!("Exported to {}", path.display());
    } else {
        print!("{content}");
    }
    Ok(())
}

async fn cmd_validate(config: &ClaudexConfig, connectivity: bool) -> Result<()> {
    let mut errors: Vec<String> = Vec::new();
    let mut warnings: Vec<String> = Vec::new();

    // 1. Profile name uniqueness
    let mut seen_names = std::collections::HashSet::new();
    for p in &config.profiles {
        if !seen_names.insert(&p.name) {
            errors.push(format!("duplicate profile name: '{}'", p.name));
        }
    }

    // 2. backup_providers reference existing profiles
    for p in &config.profiles {
        for backup in &p.backup_providers {
            if config.find_profile(backup).is_none() {
                errors.push(format!(
                    "profile '{}': backup_provider '{}' does not exist",
                    p.name, backup
                ));
            }
        }
    }

    // 3. OAuth profiles must have oauth_provider
    for p in &config.profiles {
        if p.auth_type == AuthType::OAuth && p.oauth_provider.is_none() {
            errors.push(format!(
                "profile '{}': auth_type is 'oauth' but oauth_provider is not set",
                p.name
            ));
        }
    }

    // 4. Router/context references
    if config.router.enabled
        && !config.router.profile.is_empty()
        && config.find_profile(&config.router.profile).is_none()
    {
        warnings.push(format!(
            "router.profile '{}' does not match any profile",
            config.router.profile
        ));
    }
    if config.context.compression.enabled
        && !config.context.compression.profile.is_empty()
        && config
            .find_profile(&config.context.compression.profile)
            .is_none()
    {
        warnings.push(format!(
            "context.compression.profile '{}' does not match any profile",
            config.context.compression.profile
        ));
    }
    if config.context.rag.enabled
        && !config.context.rag.profile.is_empty()
        && config.find_profile(&config.context.rag.profile).is_none()
    {
        warnings.push(format!(
            "context.rag.profile '{}' does not match any profile",
            config.context.rag.profile
        ));
    }

    // 5. base_url format
    for p in &config.profiles {
        if !p.base_url.starts_with("http://") && !p.base_url.starts_with("https://") {
            errors.push(format!(
                "profile '{}': base_url must start with http:// or https://",
                p.name
            ));
        }
    }

    // 6. proxy_port
    if config.proxy_port == 0 {
        errors.push("proxy_port must not be 0".to_string());
    }

    // 7. Enabled ApiKey profiles need api_key or keyring
    for p in &config.profiles {
        if p.enabled
            && p.auth_type == AuthType::ApiKey
            && p.api_key.is_empty()
            && p.api_key_keyring.is_none()
        {
            warnings.push(format!(
                "profile '{}': enabled with auth_type=ApiKey but no api_key or api_key_keyring",
                p.name
            ));
        }
    }

    // Print results
    if errors.is_empty() && warnings.is_empty() {
        println!("Config is valid.");
    }
    for w in &warnings {
        println!("WARNING: {w}");
    }
    for e in &errors {
        println!("ERROR: {e}");
    }

    // 8. Connectivity test
    if connectivity {
        println!();
        for p in &config.profiles {
            if p.enabled {
                print!("Testing {}... ", p.name);
                match crate::profile::test_connectivity(p).await {
                    Ok(latency) => println!("OK ({latency}ms)"),
                    Err(e) => println!("FAIL: {e}"),
                }
            }
        }
    }

    if !errors.is_empty() {
        anyhow::bail!("{} error(s) found", errors.len());
    }
    Ok(())
}

fn cmd_edit(config: &ClaudexConfig, global: bool) -> Result<()> {
    let path = if global {
        ClaudexConfig::config_path()?
    } else {
        config
            .config_source
            .clone()
            .unwrap_or_else(|| ClaudexConfig::config_path().unwrap_or_default())
    };

    let editor = std::env::var("VISUAL")
        .or_else(|_| std::env::var("EDITOR"))
        .unwrap_or_else(|_| "vi".to_string());

    println!("Opening {} with {editor}...", path.display());
    let status = std::process::Command::new(&editor)
        .arg(&path)
        .status()
        .with_context(|| format!("failed to launch editor: {editor}"))?;

    if !status.success() {
        anyhow::bail!("editor exited with status {status}");
    }
    Ok(())
}

fn cmd_recreate(config: &mut ClaudexConfig, force: bool) -> Result<()> {
    let global_path = ClaudexConfig::config_path()?;

    if !force {
        print!(
            "This will backup and recreate {}. Continue? [y/N] ",
            global_path.display()
        );
        std::io::Write::flush(&mut std::io::stdout())?;
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        if !input.trim().eq_ignore_ascii_case("y") {
            println!("Aborted.");
            return Ok(());
        }
    }

    // Backup existing config
    if global_path.exists() {
        let timestamp = chrono::Local::now().format("%Y%m%d%H%M%S");
        let backup_path = global_path.with_extension(format!("toml.bak.{timestamp}"));
        std::fs::copy(&global_path, &backup_path)
            .with_context(|| format!("failed to backup to {}", backup_path.display()))?;
        println!("Backed up to: {}", backup_path.display());
    }

    // Preserve profiles and model_aliases from current config
    let profiles = config.profiles.clone();
    let aliases = config.model_aliases.clone();

    // Write example template
    let example = include_str!("../config.example.toml");
    if let Some(parent) = global_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&global_path, example)?;

    // Reload, merge back profiles/aliases, save
    let mut new_config = ClaudexConfig::load_from(&global_path)?;
    new_config.profiles = profiles;
    new_config.model_aliases = aliases;
    new_config.save()?;

    println!("Recreated: {}", global_path.display());
    println!("Your profiles and model_aliases have been preserved.");
    Ok(())
}
