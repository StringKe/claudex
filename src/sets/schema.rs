use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// 配置集清单文件 (.claudex-sets.json / claudex-sets.json) 的完整类型定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetManifest {
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub author: Option<String>,
    #[serde(default)]
    pub homepage: Option<String>,
    #[serde(default)]
    pub license: Option<String>,
    pub components: Components,
    #[serde(default)]
    pub env: Vec<EnvVar>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Components {
    #[serde(default)]
    pub claude_md: Option<ClaudeMd>,
    #[serde(default)]
    pub rules: Vec<Rule>,
    #[serde(default)]
    pub skills: Vec<Skill>,
    #[serde(default)]
    pub mcp_servers: Vec<McpServer>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeMd {
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rule {
    pub name: String,
    pub path: String,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skill {
    pub name: String,
    pub path: String,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServer {
    pub name: String,
    #[serde(rename = "type")]
    pub server_type: McpServerType,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub command: Option<String>,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub headers: HashMap<String, String>,
    #[serde(default)]
    pub env: HashMap<String, String>,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum McpServerType {
    Http,
    Stdio,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvVar {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub default: Option<String>,
}

impl SetManifest {
    /// 从 JSON 字符串解析清单
    pub fn from_json(content: &str) -> anyhow::Result<Self> {
        let manifest: Self =
            serde_json::from_str(content).map_err(|e| anyhow::anyhow!("invalid manifest: {e}"))?;
        manifest.validate()?;
        Ok(manifest)
    }

    /// 从文件路径加载清单
    pub fn from_file(path: &std::path::Path) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| anyhow::anyhow!("failed to read {}: {e}", path.display()))?;
        Self::from_json(&content)
    }

    /// 在目录中查找清单文件（.claudex-sets.json 或 claudex-sets.json）
    pub fn find_in_dir(dir: &std::path::Path) -> anyhow::Result<(std::path::PathBuf, Self)> {
        for name in &[".claudex-sets.json", "claudex-sets.json"] {
            let path = dir.join(name);
            if path.exists() {
                let manifest = Self::from_file(&path)?;
                return Ok((path, manifest));
            }
        }
        anyhow::bail!(
            "no .claudex-sets.json or claudex-sets.json found in {}",
            dir.display()
        )
    }

    /// 验证清单字段合法性
    fn validate(&self) -> anyhow::Result<()> {
        if self.name.is_empty() {
            anyhow::bail!("manifest name cannot be empty");
        }
        // 验证 name 格式：^[a-z0-9][a-z0-9._-]*$
        let name_re = regex::Regex::new(r"^[a-z0-9][a-z0-9._-]*$").unwrap();
        if !name_re.is_match(&self.name) {
            anyhow::bail!(
                "invalid manifest name '{}': must match ^[a-z0-9][a-z0-9._-]*$",
                self.name
            );
        }
        if self.version.is_empty() {
            anyhow::bail!("manifest version cannot be empty");
        }
        // 验证 MCP server 完整性
        for mcp in &self.components.mcp_servers {
            match mcp.server_type {
                McpServerType::Http => {
                    if mcp.url.is_none() {
                        anyhow::bail!("MCP server '{}' is http type but missing url", mcp.name);
                    }
                }
                McpServerType::Stdio => {
                    if mcp.command.is_none() {
                        anyhow::bail!(
                            "MCP server '{}' is stdio type but missing command",
                            mcp.name
                        );
                    }
                }
            }
        }
        Ok(())
    }
}
