mod conflict;
mod install;
pub mod lock;
mod mcp;
pub mod schema;
mod source;

use anyhow::Result;

use lock::{Scope, SetsLockFile};

pub async fn add(input: &str, global: bool, git_ref: Option<&str>) -> Result<()> {
    let scope = if global { Scope::Global } else { Scope::Project };
    let source = source::resolve_source(input, git_ref)?;

    println!("Resolving source...");
    let cache_dir = SetsLockFile::cache_dir(scope)?;
    let (source_dir, manifest) = source::fetch_source(&source, &cache_dir).await?;

    println!(
        "Found set: {} v{} ({})",
        manifest.name,
        manifest.version,
        manifest.description.as_deref().unwrap_or("")
    );

    // 检查是否已安装
    let mut lock = SetsLockFile::load(scope)?;
    if let Some(existing) = lock.find(&manifest.name) {
        if existing.version == manifest.version {
            println!("Set '{}' v{} is already installed.", manifest.name, manifest.version);
            return Ok(());
        }
        println!(
            "Updating '{}' from v{} to v{}",
            manifest.name, existing.version, manifest.version
        );
    }

    // 处理环境变量
    let env_values = install::collect_env_values(&manifest)?;

    // 安装
    let ctx = install::InstallContext {
        scope,
        manifest: manifest.clone(),
        source_dir,
        env_values,
    };
    let result = install::install_set(&ctx).await?;

    // 写 lock
    let now = chrono::Utc::now().to_rfc3339();
    let git_sha = source::get_git_sha(&source, &cache_dir)?;
    let locked = lock::LockedSet {
        name: manifest.name.clone(),
        source: input.to_string(),
        source_type: source.source_type(),
        version: manifest.version.clone(),
        locked_ref: git_sha,
        pinned: git_ref.is_some(),
        installed_components: result.components,
        installed_at: lock
            .find(&manifest.name)
            .map(|s| s.installed_at.clone())
            .unwrap_or_else(|| now.clone()),
        updated_at: now,
    };
    lock.upsert(locked);
    lock.save(scope)?;

    println!("\nInstalled '{}' v{} ({})", manifest.name, manifest.version, scope);
    Ok(())
}

pub async fn remove(name: &str, global: bool) -> Result<()> {
    let scope = if global { Scope::Global } else { Scope::Project };
    let mut lock = SetsLockFile::load(scope)?;

    let entry = lock
        .find(name)
        .ok_or_else(|| anyhow::anyhow!("set '{}' is not installed ({})", name, scope))?
        .clone();

    install::uninstall_set(scope, &entry).await?;

    // 删除缓存目录
    let cache = SetsLockFile::cache_dir(scope)?.join(name);
    if cache.exists() {
        std::fs::remove_dir_all(&cache)?;
    }

    lock.remove(name);
    lock.save(scope)?;

    println!("Removed '{}' ({})", name, scope);
    Ok(())
}

pub fn list(global: bool) -> Result<()> {
    let scope = if global { Scope::Global } else { Scope::Project };
    let lock = SetsLockFile::load(scope)?;

    if lock.sets.is_empty() {
        println!("No sets installed ({}).", scope);
        return Ok(());
    }

    println!(
        "{:<20} {:<10} {:<8} {:<8} COMPONENTS",
        "NAME", "VERSION", "SOURCE", "PINNED"
    );
    println!("{}", "-".repeat(70));

    for s in &lock.sets {
        let components = format_components(&s.installed_components);
        println!(
            "{:<20} {:<10} {:<8} {:<8} {}",
            s.name,
            s.version,
            s.source_type,
            if s.pinned { "yes" } else { "no" },
            components,
        );
    }
    Ok(())
}

pub async fn update(name: Option<&str>, global: bool) -> Result<()> {
    let scope = if global { Scope::Global } else { Scope::Project };
    let lock = SetsLockFile::load(scope)?;

    let targets: Vec<_> = if let Some(name) = name {
        let entry = lock
            .find(name)
            .ok_or_else(|| anyhow::anyhow!("set '{}' is not installed ({})", name, scope))?;
        vec![entry.clone()]
    } else {
        lock.sets.clone()
    };

    if targets.is_empty() {
        println!("No sets to update ({}).", scope);
        return Ok(());
    }

    for entry in &targets {
        if entry.pinned {
            println!("Skipping '{}' (pinned to ref)", entry.name);
            continue;
        }
        println!("Updating '{}'...", entry.name);
        // 重新 add 会自动处理版本比较和安装
        if let Err(e) = add(&entry.source, global, None).await {
            println!("Failed to update '{}': {e}", entry.name);
        }
    }

    Ok(())
}

pub fn show(name: &str, global: bool) -> Result<()> {
    let scope = if global { Scope::Global } else { Scope::Project };
    let lock = SetsLockFile::load(scope)?;

    let entry = lock
        .find(name)
        .ok_or_else(|| anyhow::anyhow!("set '{}' is not installed ({})", name, scope))?;

    println!("Name:         {}", entry.name);
    println!("Version:      {}", entry.version);
    println!("Source:       {}", entry.source);
    println!("Source Type:  {}", entry.source_type);
    println!("Pinned:       {}", entry.pinned);
    if let Some(ref sha) = entry.locked_ref {
        println!("Locked Ref:   {}", sha);
    }
    println!("Installed At: {}", entry.installed_at);
    println!("Updated At:   {}", entry.updated_at);
    println!("Components:");
    let c = &entry.installed_components;
    if c.claude_md {
        println!("  CLAUDE.md:   installed");
    }
    if !c.rules.is_empty() {
        println!("  Rules:       {}", c.rules.join(", "));
    }
    if !c.skills.is_empty() {
        println!("  Skills:      {}", c.skills.join(", "));
    }
    if !c.mcp_servers.is_empty() {
        println!("  MCP Servers: {}", c.mcp_servers.join(", "));
    }

    Ok(())
}

fn format_components(c: &lock::InstalledComponents) -> String {
    let mut parts = Vec::new();
    if c.claude_md {
        parts.push("CLAUDE.md".to_string());
    }
    if !c.rules.is_empty() {
        parts.push(format!("{} rules", c.rules.len()));
    }
    if !c.skills.is_empty() {
        parts.push(format!("{} skills", c.skills.len()));
    }
    if !c.mcp_servers.is_empty() {
        parts.push(format!("{} mcp", c.mcp_servers.len()));
    }
    parts.join(", ")
}
