# Contributing to Claudex / 贡献指南

[English](#english) | [中文](#中文)

---

## English

Thank you for your interest in contributing to Claudex! This guide will help you get started.

### Code of Conduct

By participating in this project, you agree to abide by our [Code of Conduct](CODE_OF_CONDUCT.md).

### Getting Started

**Prerequisites:**

- Rust stable (1.75+)
- [Claude Code](https://docs.anthropic.com/en/docs/claude-code) installed (for integration testing)

**Setup:**

```bash
git clone https://github.com/StringKe/claudex.git
cd claudex
cargo build
cargo test
```

### How to Contribute

#### Bug Fixes

1. Find or create an issue describing the bug
2. Fork the repo and create a branch: `git checkout -b fix/description`
3. Fix the bug with tests
4. Submit a PR referencing the issue

#### New Features

1. Open a feature request issue first to discuss the approach
2. Fork the repo and create a branch: `git checkout -b feat/description`
3. Implement with tests and documentation
4. Submit a PR

#### New Provider Support

We especially welcome contributions adding new AI providers! See the [Provider Request](https://github.com/StringKe/claudex/issues/new?template=provider_request.yml) template.

To add a provider:

1. Add a profile example to `config.example.toml`
2. If the provider needs special handling, modify `src/proxy/translation.rs`
3. Test with `claudex profile test <name>` and `claudex run <name> "test" --print`
4. Update documentation in `website/src/content/docs/`

#### Documentation

Documentation lives in two places:
- `README.md` / `README.zh-CN.md` — project overview
- `website/src/content/docs/` — full documentation site (Astro Starlight)

All documentation should be bilingual (English + Chinese).

### Development Workflow

1. Branch from `main`
2. One logical change per commit
3. Run the full check suite before pushing:

```bash
cargo fmt --check
cargo clippy
cargo test
cargo check
```

4. CI must pass (fmt + clippy + test + build)

### Commit Convention

```
<type>[scope]: <description in Chinese>
```

| Type | Usage |
|------|-------|
| `feat` | New feature |
| `fix` | Bug fix |
| `docs` | Documentation |
| `refactor` | Refactoring |
| `perf` | Performance |
| `test` | Tests |
| `build` | Build/dependencies |
| `ci` | CI/CD |
| `chore` | Miscellaneous |

### Code Style

- Run `cargo fmt` before committing
- Zero `cargo clippy` warnings
- Use `anyhow::Result` + `?` for error propagation, no `unwrap()` in production code
- Use `tracing::info!` / `tracing::warn!` / `tracing::error!` for logging
- Keep comments in English or Chinese, code identifiers in English

### Security

To report a security vulnerability, please use [GitHub Private Vulnerability Reporting](https://github.com/StringKe/claudex/security/advisories/new). See [SECURITY.md](SECURITY.md) for details.

---

## 中文

感谢你对 Claudex 的关注！本指南帮助你快速上手贡献。

### 行为准则

参与本项目即表示你同意遵守 [行为准则](CODE_OF_CONDUCT.md)。

### 快速开始

**前置条件：**

- Rust stable (1.75+)
- 已安装 [Claude Code](https://docs.anthropic.com/en/docs/claude-code)（集成测试用）

**本地开发：**

```bash
git clone https://github.com/StringKe/claudex.git
cd claudex
cargo build
cargo test
```

### 如何贡献

#### 修复缺陷

1. 找到或创建描述该缺陷的 issue
2. Fork 仓库并创建分支：`git checkout -b fix/描述`
3. 修复并编写测试
4. 提交 PR 并关联 issue

#### 新功能

1. 先开 feature request issue 讨论方案
2. Fork 仓库并创建分支：`git checkout -b feat/描述`
3. 实现功能、编写测试、更新文档
4. 提交 PR

#### 新增提供商支持

我们特别欢迎添加新 AI 提供商的贡献！请使用 [Provider Request](https://github.com/StringKe/claudex/issues/new?template=provider_request.yml) 模板。

添加步骤：

1. 在 `config.example.toml` 中添加 profile 示例
2. 如果提供商需要特殊处理，修改 `src/proxy/translation.rs`
3. 用 `claudex profile test <name>` 和 `claudex run <name> "test" --print` 测试
4. 更新 `website/src/content/docs/` 中的文档

#### 文档

文档在两处：
- `README.md` / `README.zh-CN.md` — 项目概览
- `website/src/content/docs/` — 完整文档站（Astro Starlight）

所有文档需中英双语。

### 开发流程

1. 从 `main` 分支创建
2. 每个 commit 一个逻辑变更
3. 推送前运行完整检查：

```bash
cargo fmt --check
cargo clippy
cargo test
cargo check
```

4. CI 必须通过（fmt + clippy + test + build）

### 提交规范

```
<类型>[范围]: <中文描述>
```

| 类型 | 说明 |
|------|------|
| `feat` | 新功能 |
| `fix` | 修复 |
| `docs` | 文档 |
| `refactor` | 重构 |
| `perf` | 性能 |
| `test` | 测试 |
| `build` | 构建/依赖 |
| `ci` | CI/CD |
| `chore` | 杂项 |

### 代码规范

- 提交前运行 `cargo fmt`
- `cargo clippy` 零 warning
- 使用 `anyhow::Result` + `?` 传播错误，生产代码禁止 `unwrap()`
- 使用 `tracing` 记录日志
- 注释中英文皆可，代码标识符用英文

### 安全

报告安全漏洞请使用 [GitHub 私密漏洞报告](https://github.com/StringKe/claudex/security/advisories/new)。详见 [SECURITY.md](SECURITY.md)。
