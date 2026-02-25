use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result};

use super::conflict;
use super::lock::Scope;
use super::schema::{McpServer, McpServerType};

/// 将 MCP server 写入 claude.json
pub fn install_mcp_server(
    server: &McpServer,
    scope: Scope,
    env_values: &HashMap<String, String>,
) -> Result<bool> {
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
            return Ok(false);
        }
    }

    // 构建 server 配置 JSON
    let server_value = build_server_json(server, env_values)?;
    servers_map.insert(server.name.clone(), server_value);

    write_claude_json(&json_path, &doc)?;
    Ok(true)
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
                    headers.insert(k.clone(), serde_json::Value::String(interpolate(v, env_values)));
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
                    env.insert(k.clone(), serde_json::Value::String(interpolate(v, env_values)));
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
    let re = regex::Regex::new(r"\$\{([^}]+)\}").unwrap();
    for cap in re.captures_iter(template) {
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
