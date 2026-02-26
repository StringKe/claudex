use std::collections::HashMap;
use std::io::{self, Write};
use std::path::Path;

use anyhow::{Context, Result};

use super::conflict;
use super::lock::Scope;
use super::schema::{McpServer, McpServerType};

/// 检测 command 是否存在于 PATH 中
fn check_command_available(command: &str) -> bool {
    which::which(command).is_ok()
}

/// stdio command 不可用时的安装决策
#[derive(Debug, PartialEq)]
pub enum CommandAction {
    InstallAnyway,
    Skip,
    Retry,
}

/// 交互式询问用户如何处理不可用的 command
fn prompt_command_action(server: &McpServer) -> Result<CommandAction> {
    let command = server.command.as_deref().unwrap_or("unknown");
    println!("  Warning: command '{}' not found in PATH", command);
    if let Some(ref setup) = server.setup {
        println!("    Setup: {}", setup);
    }
    print!("    [1] Install anyway  [2] Skip  [3] Retry: ");
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    match input.trim() {
        "1" => Ok(CommandAction::InstallAnyway),
        "3" => Ok(CommandAction::Retry),
        _ => Ok(CommandAction::Skip),
    }
}

/// 将 MCP server 写入 claude.json
pub fn install_mcp_server(
    server: &McpServer,
    scope: Scope,
    env_values: &HashMap<String, String>,
) -> Result<InstallMcpResult> {
    // stdio 类型检测 command 可用性
    let mut command_missing = false;
    if server.server_type == McpServerType::Stdio {
        if let Some(ref command) = server.command {
            loop {
                if check_command_available(command) {
                    break;
                }
                let action = prompt_command_action(server)?;
                match action {
                    CommandAction::InstallAnyway => {
                        command_missing = true;
                        break;
                    }
                    CommandAction::Skip => return Ok(InstallMcpResult::Skipped),
                    CommandAction::Retry => continue,
                }
            }
        }
    }

    let json_path = super::lock::SetsLockFile::claude_json_path(scope)?;
    let mut doc = read_claude_json(&json_path)?;

    let mcp_servers = doc
        .as_object_mut()
        .context("claude.json is not a JSON object")?
        .entry("mcpServers")
        .or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()));

    let servers_map = mcp_servers
        .as_object_mut()
        .context("mcpServers is not an object")?;

    // 检查冲突
    if servers_map.contains_key(&server.name) {
        let resolution = conflict::resolve_mcp_conflict(&server.name)?;
        if resolution == conflict::ConflictResolution::Skip {
            return Ok(InstallMcpResult::Skipped);
        }
    }

    // 构建 server 配置 JSON
    let server_value = build_server_json(server, env_values)?;
    servers_map.insert(server.name.clone(), server_value);

    write_claude_json(&json_path, &doc)?;

    if command_missing {
        Ok(InstallMcpResult::InstalledCommandMissing)
    } else {
        Ok(InstallMcpResult::Installed)
    }
}

/// MCP 安装结果
#[derive(Debug, PartialEq)]
pub enum InstallMcpResult {
    Installed,
    InstalledCommandMissing,
    Skipped,
}

/// 从 claude.json 移除 MCP server
pub fn uninstall_mcp_server(name: &str, scope: Scope) -> Result<()> {
    let json_path = super::lock::SetsLockFile::claude_json_path(scope)?;
    if !json_path.exists() {
        return Ok(());
    }

    let mut doc = read_claude_json(&json_path)?;

    if let Some(obj) = doc.as_object_mut() {
        if let Some(servers) = obj.get_mut("mcpServers") {
            if let Some(map) = servers.as_object_mut() {
                map.remove(name);
            }
        }
    }

    write_claude_json(&json_path, &doc)?;
    Ok(())
}

/// 构建与 `claude mcp add` 产出一致的 JSON 格式
fn build_server_json(
    server: &McpServer,
    env_values: &HashMap<String, String>,
) -> Result<serde_json::Value> {
    let mut obj = serde_json::Map::new();

    match server.server_type {
        McpServerType::Http => {
            obj.insert(
                "type".to_string(),
                serde_json::Value::String("http".to_string()),
            );
            if let Some(ref url) = server.url {
                obj.insert("url".to_string(), serde_json::Value::String(url.clone()));
            }
            if !server.headers.is_empty() {
                let mut headers = serde_json::Map::new();
                for (k, v) in &server.headers {
                    headers.insert(
                        k.clone(),
                        serde_json::Value::String(interpolate(v, env_values)),
                    );
                }
                obj.insert("headers".to_string(), serde_json::Value::Object(headers));
            }
        }
        McpServerType::Stdio => {
            obj.insert(
                "type".to_string(),
                serde_json::Value::String("stdio".to_string()),
            );
            if let Some(ref cmd) = server.command {
                obj.insert(
                    "command".to_string(),
                    serde_json::Value::String(cmd.clone()),
                );
            }
            if !server.args.is_empty() {
                let args: Vec<serde_json::Value> = server
                    .args
                    .iter()
                    .map(|a| serde_json::Value::String(a.clone()))
                    .collect();
                obj.insert("args".to_string(), serde_json::Value::Array(args));
            }
            if !server.env.is_empty() {
                let mut env = serde_json::Map::new();
                for (k, v) in &server.env {
                    env.insert(
                        k.clone(),
                        serde_json::Value::String(interpolate(v, env_values)),
                    );
                }
                obj.insert("env".to_string(), serde_json::Value::Object(env));
            }
        }
    }

    Ok(serde_json::Value::Object(obj))
}

/// 替换 ${VAR_NAME} 占位符
fn interpolate(template: &str, env_values: &HashMap<String, String>) -> String {
    let mut result = template.to_string();
    // 匹配 ${VAR_NAME} 模式
    static RE: std::sync::LazyLock<regex::Regex> =
        std::sync::LazyLock::new(|| regex::Regex::new(r"\$\{([^}]+)\}").unwrap());
    for cap in RE.captures_iter(template) {
        let full_match = &cap[0];
        let var_name = &cap[1];
        if let Some(value) = env_values.get(var_name) {
            result = result.replace(full_match, value);
        } else if let Ok(value) = std::env::var(var_name) {
            result = result.replace(full_match, &value);
        }
        // 如果都没有，保留原始占位符
    }
    result
}

/// 读取 claude.json，不存在则返回空对象
fn read_claude_json(path: &Path) -> Result<serde_json::Value> {
    if !path.exists() {
        return Ok(serde_json::Value::Object(serde_json::Map::new()));
    }
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read {}", path.display()))?;
    if content.trim().is_empty() {
        return Ok(serde_json::Value::Object(serde_json::Map::new()));
    }
    let doc: serde_json::Value = serde_json::from_str(&content)
        .with_context(|| format!("failed to parse {}", path.display()))?;
    Ok(doc)
}

/// 写回 claude.json，保留格式化
fn write_claude_json(path: &Path, value: &serde_json::Value) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let content = serde_json::to_string_pretty(value)?;
    std::fs::write(path, content)?;
    Ok(())
}
