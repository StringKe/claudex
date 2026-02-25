use std::collections::HashMap;
use std::io::{self, Write};
use std::path::Path;

use anyhow::Result;

use super::conflict::{self, ConflictResolution};
use super::lock::{InstalledComponents, LockedSet, Scope, SetsLockFile};
use super::mcp;
use super::schema::SetManifest;

pub struct InstallContext {
    pub scope: Scope,
    pub manifest: SetManifest,
    pub source_dir: std::path::PathBuf,
    pub env_values: HashMap<String, String>,
}

pub struct InstallResult {
    pub components: InstalledComponents,
}

/// 收集环境变量：检查系统环境，缺失的交互式提示用户输入
pub fn collect_env_values(manifest: &SetManifest) -> Result<HashMap<String, String>> {
    let mut values = HashMap::new();

    for var in &manifest.env {
        // 先检查系统环境变量
        if let Ok(val) = std::env::var(&var.name) {
            values.insert(var.name.clone(), val);
            continue;
        }

        let desc = var.description.as_deref().unwrap_or("(no description)");

        if var.required {
            println!("\n[Required] {} - {}", var.name, desc);
            let input = prompt_input(&format!("Enter {}", var.name))?;
            if input.is_empty() {
                if let Some(ref default) = var.default {
                    values.insert(var.name.clone(), default.clone());
                } else {
                    anyhow::bail!("required environment variable '{}' not provided", var.name);
                }
            } else {
                values.insert(var.name.clone(), input);
            }
        } else {
            println!("\n[Optional] {} - {}", var.name, desc);
            if let Some(ref default) = var.default {
                println!("  Default: {default}");
            }
            let input = prompt_input(&format!("Enter {} (or press Enter to skip)", var.name))?;
            if !input.is_empty() {
                values.insert(var.name.clone(), input);
            } else if let Some(ref default) = var.default {
                values.insert(var.name.clone(), default.clone());
            }
        }
    }

    Ok(values)
}

/// 安装配置集所有组件
pub async fn install_set(ctx: &InstallContext) -> Result<InstallResult> {
    let claude_dir = SetsLockFile::claude_dir(ctx.scope)?;
    std::fs::create_dir_all(&claude_dir)?;

    let mut components = InstalledComponents::default();

    // 1. CLAUDE.md
    if let Some(ref claude_md) = ctx.manifest.components.claude_md {
        let source = ctx.source_dir.join(&claude_md.path);
        let target = claude_dir.join("CLAUDE.md");

        if !source.exists() {
            println!("Warning: CLAUDE.md source not found: {}", source.display());
        } else if conflict::has_conflict(&target) {
            let resolution = conflict::resolve_file_conflict(&source, &target, "CLAUDE.md")?;
            if resolution != ConflictResolution::Skip {
                conflict::apply_file_resolution(&source, &target, resolution)?;
                components.claude_md = true;
                println!("  Installed CLAUDE.md");
            } else {
                println!("  Skipped CLAUDE.md");
            }
        } else {
            std::fs::copy(&source, &target)?;
            components.claude_md = true;
            println!("  Installed CLAUDE.md");
        }
    }

    // 2. Rules
    if !ctx.manifest.components.rules.is_empty() {
        let rules_dir = claude_dir.join("rules");
        std::fs::create_dir_all(&rules_dir)?;

        for rule in &ctx.manifest.components.rules {
            let source = ctx.source_dir.join(&rule.path);
            let target = rules_dir.join(format!("{}.md", rule.name));

            if !source.exists() {
                println!(
                    "Warning: rule '{}' source not found: {}",
                    rule.name,
                    source.display()
                );
                continue;
            }

            if conflict::has_conflict(&target) {
                let label = format!("rule '{}'", rule.name);
                let resolution = conflict::resolve_file_conflict(&source, &target, &label)?;
                if resolution != ConflictResolution::Skip {
                    conflict::apply_file_resolution(&source, &target, resolution)?;
                    components.rules.push(rule.name.clone());
                    println!("  Installed rule: {}", rule.name);
                } else {
                    println!("  Skipped rule: {}", rule.name);
                }
            } else {
                std::fs::copy(&source, &target)?;
                components.rules.push(rule.name.clone());
                println!("  Installed rule: {}", rule.name);
            }
        }
    }

    // 3. Skills
    if !ctx.manifest.components.skills.is_empty() {
        let skills_dir = claude_dir.join("skills");
        std::fs::create_dir_all(&skills_dir)?;

        for skill in &ctx.manifest.components.skills {
            let source = ctx.source_dir.join(&skill.path);
            let target = skills_dir.join(&skill.name);

            if !source.exists() {
                println!(
                    "Warning: skill '{}' source not found: {}",
                    skill.name,
                    source.display()
                );
                continue;
            }

            if conflict::has_conflict(&target) {
                let label = format!("skill '{}'", skill.name);
                let resolution = conflict::resolve_dir_conflict(&target, &label)?;
                if resolution != ConflictResolution::Skip {
                    copy_dir_recursive(&source, &target)?;
                    components.skills.push(skill.name.clone());
                    println!("  Installed skill: {}", skill.name);
                } else {
                    println!("  Skipped skill: {}", skill.name);
                }
            } else {
                copy_dir_recursive(&source, &target)?;
                components.skills.push(skill.name.clone());
                println!("  Installed skill: {}", skill.name);
            }
        }
    }

    // 4. MCP Servers
    for server in &ctx.manifest.components.mcp_servers {
        match mcp::install_mcp_server(server, ctx.scope, &ctx.env_values) {
            Ok(true) => {
                components.mcp_servers.push(server.name.clone());
                println!("  Installed MCP: {}", server.name);
            }
            Ok(false) => {
                println!("  Skipped MCP: {}", server.name);
            }
            Err(e) => {
                println!("  Warning: failed to install MCP '{}': {e}", server.name);
            }
        }
    }

    Ok(InstallResult { components })
}

/// 卸载配置集的所有已安装组件
pub async fn uninstall_set(scope: Scope, entry: &LockedSet) -> Result<()> {
    let claude_dir = SetsLockFile::claude_dir(scope)?;
    let c = &entry.installed_components;

    // CLAUDE.md
    if c.claude_md {
        let target = claude_dir.join("CLAUDE.md");
        if target.exists() {
            println!("Removing CLAUDE.md...");
            // CLAUDE.md 可能被手动修改，提示确认
            print!("  CLAUDE.md may have been modified. Remove? [y/N]: ");
            io::stdout().flush()?;
            let mut input = String::new();
            io::stdin().read_line(&mut input)?;
            if input.trim().eq_ignore_ascii_case("y") {
                std::fs::remove_file(&target)?;
                println!("  Removed CLAUDE.md");
            } else {
                println!("  Kept CLAUDE.md");
            }
        }
    }

    // Rules
    for rule_name in &c.rules {
        let target = claude_dir.join("rules").join(format!("{rule_name}.md"));
        if target.exists() {
            std::fs::remove_file(&target)?;
            println!("  Removed rule: {rule_name}");
        }
    }

    // Skills
    for skill_name in &c.skills {
        let target = claude_dir.join("skills").join(skill_name);
        if target.exists() {
            std::fs::remove_dir_all(&target)?;
            println!("  Removed skill: {skill_name}");
        }
    }

    // MCP Servers
    for mcp_name in &c.mcp_servers {
        mcp::uninstall_mcp_server(mcp_name, scope)?;
        println!("  Removed MCP: {mcp_name}");
    }

    Ok(())
}

/// 递归复制目录
fn copy_dir_recursive(source: &Path, target: &Path) -> Result<()> {
    if target.exists() {
        std::fs::remove_dir_all(target)?;
    }
    std::fs::create_dir_all(target)?;

    for entry in std::fs::read_dir(source)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let src_path = entry.path();
        let dst_path = target.join(entry.file_name());

        if file_type.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            std::fs::copy(&src_path, &dst_path)?;
        }
    }

    Ok(())
}

fn prompt_input(label: &str) -> Result<String> {
    print!("{label}: ");
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    Ok(input.trim().to_string())
}
