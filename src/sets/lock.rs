use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// Lock 文件，记录已安装的配置集状态
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SetsLockFile {
    pub sets: Vec<LockedSet>,
}

/// 单个已安装配置集的锁定记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockedSet {
    pub name: String,
    /// 原始输入（git URL / 本地路径 / URL）
    pub source: String,
    /// 来源类型
    pub source_type: SourceType,
    /// 来自 manifest 的版本号
    pub version: String,
    /// git commit SHA（git 来源时）
    #[serde(default)]
    pub locked_ref: Option<String>,
    /// true = 锁定到指定 ref，false = 跟踪 latest
    #[serde(default)]
    pub pinned: bool,
    /// 已安装的组件记录
    pub installed_components: InstalledComponents,
    /// 安装时间 (ISO 8601)
    pub installed_at: String,
    /// 最近更新时间 (ISO 8601)
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum SourceType {
    Git,
    Local,
    Url,
}

/// 记录每种组件的安装状态
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct InstalledComponents {
    pub claude_md: bool,
    #[serde(default)]
    pub rules: Vec<String>,
    #[serde(default)]
    pub skills: Vec<String>,
    #[serde(default)]
    pub mcp_servers: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Scope {
    Global,
    Project,
}

impl SetsLockFile {
    /// Lock 文件路径
    pub fn lock_path(scope: Scope) -> Result<PathBuf> {
        match scope {
            Scope::Global => {
                let home = dirs::home_dir().context("cannot determine home directory")?;
                Ok(home.join(".config").join("claudex").join("sets.lock.json"))
            }
            Scope::Project => {
                let cwd = std::env::current_dir()?;
                Ok(cwd.join(".claudex").join("sets.lock.json"))
            }
        }
    }

    /// 配置集缓存目录
    pub fn cache_dir(scope: Scope) -> Result<PathBuf> {
        match scope {
            Scope::Global => {
                let home = dirs::home_dir().context("cannot determine home directory")?;
                Ok(home.join(".config").join("claudex").join("sets"))
            }
            Scope::Project => {
                let cwd = std::env::current_dir()?;
                Ok(cwd.join(".claudex").join("sets"))
            }
        }
    }

    /// Claude 配置安装目标目录
    pub fn claude_dir(scope: Scope) -> Result<PathBuf> {
        match scope {
            Scope::Global => {
                let home = dirs::home_dir().context("cannot determine home directory")?;
                Ok(home.join(".claude"))
            }
            Scope::Project => {
                let cwd = std::env::current_dir()?;
                Ok(cwd.join(".claude"))
            }
        }
    }

    /// claude.json 文件路径（MCP 配置写入目标）
    pub fn claude_json_path(scope: Scope) -> Result<PathBuf> {
        match scope {
            Scope::Global => {
                let home = dirs::home_dir().context("cannot determine home directory")?;
                Ok(home.join(".claude.json"))
            }
            Scope::Project => {
                let cwd = std::env::current_dir()?;
                Ok(cwd.join(".claude.json"))
            }
        }
    }

    /// 从文件加载 lock
    pub fn load(scope: Scope) -> Result<Self> {
        let path = Self::lock_path(scope)?;
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = std::fs::read_to_string(&path)
            .with_context(|| format!("failed to read lock file: {}", path.display()))?;
        let lock: Self = serde_json::from_str(&content)
            .with_context(|| format!("failed to parse lock file: {}", path.display()))?;
        Ok(lock)
    }

    /// 保存 lock 文件
    pub fn save(&self, scope: Scope) -> Result<()> {
        let path = Self::lock_path(scope)?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(&path, content)?;
        Ok(())
    }

    /// 查找已安装的 set
    pub fn find(&self, name: &str) -> Option<&LockedSet> {
        self.sets.iter().find(|s| s.name == name)
    }

    /// 查找并返回可变引用
    pub fn find_mut(&mut self, name: &str) -> Option<&mut LockedSet> {
        self.sets.iter_mut().find(|s| s.name == name)
    }

    /// 移除已安装的 set
    pub fn remove(&mut self, name: &str) -> Option<LockedSet> {
        if let Some(idx) = self.sets.iter().position(|s| s.name == name) {
            Some(self.sets.remove(idx))
        } else {
            None
        }
    }

    /// 添加或更新 set 记录
    pub fn upsert(&mut self, entry: LockedSet) {
        if let Some(existing) = self.find_mut(&entry.name) {
            *existing = entry;
        } else {
            self.sets.push(entry);
        }
    }
}

impl std::fmt::Display for Scope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Scope::Global => write!(f, "global"),
            Scope::Project => write!(f, "project"),
        }
    }
}

impl std::fmt::Display for SourceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SourceType::Git => write!(f, "git"),
            SourceType::Local => write!(f, "local"),
            SourceType::Url => write!(f, "url"),
        }
    }
}
