use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use super::lock::SourceType;
use super::schema::SetManifest;

/// 解析后的配置集来源
#[derive(Debug, Clone)]
pub enum SetSource {
    Git {
        url: String,
        git_ref: Option<String>,
    },
    Local {
        path: PathBuf,
    },
    Url {
        url: String,
    },
}

impl SetSource {
    pub fn source_type(&self) -> SourceType {
        match self {
            SetSource::Git { .. } => SourceType::Git,
            SetSource::Local { .. } => SourceType::Local,
            SetSource::Url { .. } => SourceType::Url,
        }
    }
}

/// 从用户输入字符串解析来源类型
pub fn resolve_source(input: &str, git_ref: Option<&str>) -> Result<SetSource> {
    // 本地路径：以 / 或 ./ 或 ~ 开头，或者是存在的目录
    let expanded = if input.starts_with('~') {
        let home = dirs::home_dir().context("cannot determine home directory")?;
        home.join(input.trim_start_matches("~/")).to_string_lossy().to_string()
    } else {
        input.to_string()
    };

    let as_path = PathBuf::from(&expanded);
    if as_path.is_dir() {
        return Ok(SetSource::Local { path: as_path });
    }

    // Git 仓库：包含 .git 后缀、github.com、gitlab.com 等
    if input.ends_with(".git")
        || input.contains("github.com")
        || input.contains("gitlab.com")
        || input.contains("bitbucket.org")
        || input.starts_with("git@")
    {
        return Ok(SetSource::Git {
            url: input.to_string(),
            git_ref: git_ref.map(|s| s.to_string()),
        });
    }

    // URL
    if input.starts_with("http://") || input.starts_with("https://") {
        // 可能是 git 仓库的 HTTPS URL（没有 .git 后缀）
        // 尝试作为 git 处理
        return Ok(SetSource::Git {
            url: input.to_string(),
            git_ref: git_ref.map(|s| s.to_string()),
        });
    }

    anyhow::bail!(
        "cannot resolve source '{}': not a local directory, git URL, or HTTP URL",
        input
    )
}

/// 获取配置集到本地目录，返回 (source_dir, manifest)
pub async fn fetch_source(
    source: &SetSource,
    cache_dir: &Path,
) -> Result<(PathBuf, SetManifest)> {
    match source {
        SetSource::Local { path } => {
            let (_manifest_path, manifest) = SetManifest::find_in_dir(path)?;
            Ok((path.clone(), manifest))
        }
        SetSource::Git { url, git_ref } => fetch_git(url, git_ref.as_deref(), cache_dir).await,
        SetSource::Url { url } => fetch_url(url, cache_dir).await,
    }
}

async fn fetch_git(
    url: &str,
    git_ref: Option<&str>,
    cache_dir: &Path,
) -> Result<(PathBuf, SetManifest)> {
    // 从 URL 推导目录名
    let dir_name = url
        .rsplit('/')
        .next()
        .unwrap_or("unknown")
        .trim_end_matches(".git");
    let target_dir = cache_dir.join(dir_name);

    std::fs::create_dir_all(cache_dir)?;

    if target_dir.exists() {
        // 已存在则 fetch + reset
        tracing::info!("updating existing clone: {}", target_dir.display());
        let status = std::process::Command::new("git")
            .args(["fetch", "--all"])
            .current_dir(&target_dir)
            .status()
            .context("failed to run git fetch")?;
        if !status.success() {
            // fetch 失败则重新 clone
            std::fs::remove_dir_all(&target_dir)?;
            clone_repo(url, git_ref, &target_dir)?;
        } else {
            // checkout 到指定 ref
            let checkout_ref = git_ref.unwrap_or("origin/HEAD");
            let status = std::process::Command::new("git")
                .args(["checkout", checkout_ref])
                .current_dir(&target_dir)
                .status()
                .context("failed to run git checkout")?;
            if !status.success() {
                anyhow::bail!("git checkout '{}' failed", checkout_ref);
            }
        }
    } else {
        clone_repo(url, git_ref, &target_dir)?;
    }

    let (_manifest_path, manifest) = SetManifest::find_in_dir(&target_dir)?;
    Ok((target_dir, manifest))
}

fn clone_repo(url: &str, git_ref: Option<&str>, target_dir: &Path) -> Result<()> {
    let mut args = vec!["clone", "--depth", "1"];
    if let Some(r) = git_ref {
        args.extend(["--branch", r]);
    }
    args.push(url);
    args.push(
        target_dir
            .to_str()
            .context("invalid cache directory path")?,
    );

    let status = std::process::Command::new("git")
        .args(&args)
        .status()
        .context("failed to run git clone")?;

    if !status.success() {
        anyhow::bail!("git clone failed for '{}'", url);
    }
    Ok(())
}

async fn fetch_url(url: &str, cache_dir: &Path) -> Result<(PathBuf, SetManifest)> {
    std::fs::create_dir_all(cache_dir)?;

    let client = reqwest::Client::new();
    let resp = client
        .get(url)
        .send()
        .await
        .with_context(|| format!("failed to fetch {url}"))?;

    if !resp.status().is_success() {
        anyhow::bail!("HTTP {} fetching {}", resp.status(), url);
    }

    let content = resp.text().await?;
    let manifest = SetManifest::from_json(&content)?;

    // URL 来源只有清单文件本身，需要下载关联的组件文件
    // 目前 URL 来源主要用于直接指向 git 仓库，纯 URL 下载单文件场景有限
    let dir_name = &manifest.name;
    let target_dir = cache_dir.join(dir_name);
    std::fs::create_dir_all(&target_dir)?;

    // 保存清单文件
    let manifest_path = target_dir.join(".claudex-sets.json");
    std::fs::write(&manifest_path, &content)?;

    Ok((target_dir, manifest))
}

/// 获取 git 来源的当前 commit SHA
pub fn get_git_sha(source: &SetSource, cache_dir: &Path) -> Result<Option<String>> {
    if let SetSource::Git { url, .. } = source {
        let dir_name = url
            .rsplit('/')
            .next()
            .unwrap_or("unknown")
            .trim_end_matches(".git");
        let target_dir = cache_dir.join(dir_name);

        if target_dir.exists() {
            let output = std::process::Command::new("git")
                .args(["rev-parse", "HEAD"])
                .current_dir(&target_dir)
                .output()
                .context("failed to get git SHA")?;
            if output.status.success() {
                let sha = String::from_utf8_lossy(&output.stdout).trim().to_string();
                return Ok(Some(sha));
            }
        }
    }
    Ok(None)
}
