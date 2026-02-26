use std::path::Path;

use anyhow::Result;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConflictResolution {
    Replace,
    Append,
    Prepend,
    Skip,
}

/// 检查目标文件是否已存在
pub fn has_conflict(target: &Path) -> bool {
    target.exists()
}

/// 交互式解决文件冲突
pub fn resolve_file_conflict(
    source: &Path,
    target: &Path,
    component_label: &str,
) -> Result<ConflictResolution> {
    println!(
        "\n[Conflict] {} already exists: {}",
        component_label,
        target.display()
    );
    println!("  1) Replace (overwrite with set version)");
    println!("  2) Append (add set content to end)");
    println!("  3) Prepend (add set content to beginning)");
    println!("  4) Skip (keep existing)");
    println!("  5) View diff");

    loop {
        let choice = prompt_input("Select [1-5]")?;
        match choice.as_str() {
            "1" => return Ok(ConflictResolution::Replace),
            "2" => return Ok(ConflictResolution::Append),
            "3" => return Ok(ConflictResolution::Prepend),
            "4" => return Ok(ConflictResolution::Skip),
            "5" => {
                show_diff(source, target)?;
                continue;
            }
            _ => {
                println!("Invalid choice, please select 1-5");
                continue;
            }
        }
    }
}

/// 对目录冲突的简化交互（只有 replace/skip）
pub fn resolve_dir_conflict(target: &Path, component_label: &str) -> Result<ConflictResolution> {
    println!(
        "\n[Conflict] {} already exists: {}",
        component_label,
        target.display()
    );
    println!("  1) Replace (overwrite)");
    println!("  2) Skip (keep existing)");

    loop {
        let choice = prompt_input("Select [1/2]")?;
        match choice.as_str() {
            "1" => return Ok(ConflictResolution::Replace),
            "2" => return Ok(ConflictResolution::Skip),
            _ => {
                println!("Invalid choice, please select 1 or 2");
                continue;
            }
        }
    }
}

/// MCP server 冲突的简化交互
pub fn resolve_mcp_conflict(name: &str) -> Result<ConflictResolution> {
    println!("\n[Conflict] MCP server '{}' already exists", name);
    println!("  1) Replace (overwrite)");
    println!("  2) Skip (keep existing)");

    loop {
        let choice = prompt_input("Select [1/2]")?;
        match choice.as_str() {
            "1" => return Ok(ConflictResolution::Replace),
            "2" => return Ok(ConflictResolution::Skip),
            _ => {
                println!("Invalid choice, please select 1 or 2");
                continue;
            }
        }
    }
}

/// 应用冲突解决策略到文件
pub fn apply_file_resolution(
    source: &Path,
    target: &Path,
    resolution: ConflictResolution,
) -> Result<()> {
    match resolution {
        ConflictResolution::Replace => {
            std::fs::copy(source, target)?;
        }
        ConflictResolution::Append => {
            let source_content = std::fs::read_to_string(source)?;
            let mut existing = std::fs::read_to_string(target)?;
            existing.push_str("\n\n");
            existing.push_str(&source_content);
            std::fs::write(target, existing)?;
        }
        ConflictResolution::Prepend => {
            let source_content = std::fs::read_to_string(source)?;
            let existing = std::fs::read_to_string(target)?;
            let combined = format!("{source_content}\n\n{existing}");
            std::fs::write(target, combined)?;
        }
        ConflictResolution::Skip => {
            // do nothing
        }
    }
    Ok(())
}

/// 显示两个文件的差异
fn show_diff(source: &Path, target: &Path) -> Result<()> {
    let status = std::process::Command::new("diff")
        .args([
            "--color=auto",
            "-u",
            &target.to_string_lossy(),
            &source.to_string_lossy(),
        ])
        .status();

    match status {
        Ok(s) if s.success() || s.code() == Some(1) => {
            // diff returns 1 when files differ, that's normal
        }
        _ => {
            // fallback: just show both file sizes
            let source_len = std::fs::metadata(source).map(|m| m.len()).unwrap_or(0);
            let target_len = std::fs::metadata(target).map(|m| m.len()).unwrap_or(0);
            println!(
                "  Existing: {} bytes, New: {} bytes",
                target_len, source_len
            );
        }
    }
    Ok(())
}

use crate::util::prompt_input;
