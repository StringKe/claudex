# Research: claudex sets 配置集管理功能

## 1. 需求概述

claudex 需要一个配置集包管理器，能够从 git 仓库、本地路径、URL 安装 Claude Code 配置集（CLAUDE.md、rules、skills、MCP servers）。支持全局和项目级两种作用域。

## 2. 参考仓库分析

参考仓库 `/Users/chen/Code/ai-agents`（StringKe/ai-agents）结构：

```
ai-agents/
├── .claudex-sets.json    ← 需要新增的清单文件（当前不存在）
├── CLAUDE.md             → ~/.claude/CLAUDE.md
├── rules/
│   └── sdlc-workflow.md  → ~/.claude/rules/
├── skills/
│   ├── done/SKILL.md     → ~/.claude/skills/done/
│   └── sdlc-workflow/SKILL.md → ~/.claude/skills/sdlc-workflow/
├── setup-mcp.sh          → 废弃，改用 JSON 声明式
└── .env.example           → 环境变量模板
```

配置集包含五类组件：
1. `claude_md` - 全局指令文件
2. `rules` - 始终生效的规则
3. `skills` - 斜杠命令
4. `mcp_servers` - MCP 服务器配置
5. `env` - 环境变量依赖声明

## 3. 现有代码库分析

### CLI 结构 (src/cli.rs)

clap derive 风格。顶层 `Commands` enum，子命令通过嵌套 enum 实现：

```
Commands::Profile { action: ProfileAction }
Commands::Auth { action: AuthAction }
Commands::Proxy { action: ProxyAction }
```

新增 `Commands::Sets { action: SetsAction }` 完全符合现有模式。

### 配置系统 (src/config.rs)

- 全局配置路径：`~/.config/claudex/config.toml`
- 项目配置：向上遍历 10 层查找 `claudex.toml` 或 `.claudex/config.toml`
- 加载后设置 `config_source`（`#[serde(skip)]`），save() 写回同一位置
- 所有子配置用 `#[serde(default)]`

### 分发模式 (src/main.rs)

```rust
match cli.command {
    Some(Commands::Profile { action }) => match action { ... },
    Some(Commands::Auth { action }) => match action { ... },
    ...
}
```

模块化函数调用，config 传 `&mut` 给需要修改的操作。

### 依赖 (Cargo.toml)

已有：clap 4、serde/serde_json、toml、anyhow、tokio、dirs、reqwest。
缺少：无。git 操作可通过 `Command::new("git")` 调用外部命令。

## 4. 作用域模型

| 维度 | Global (`--global`) | Project（默认） |
|------|-------------------|----------------|
| 配置集缓存 | `~/.config/claudex/sets/` | `.claudex/sets/` |
| Lock 文件 | `~/.config/claudex/sets.lock.json` | `.claudex/sets.lock.json` |
| 安装目标 claude_md | `~/.claude/CLAUDE.md` | `.claude/CLAUDE.md` |
| 安装目标 rules | `~/.claude/rules/` | `.claude/rules/` |
| 安装目标 skills | `~/.claude/skills/` | `.claude/skills/` |
| 安装目标 MCP | `~/.claude.json`（user scope） | `.claude.json`（project scope） |
| 优先级 | 低 | 高（Claude Code 自身行为） |

## 5. Lock 文件设计

Lock 文件记录已安装配置集的精确状态，用于 update/remove 操作。

位置：
- 全局：`~/.config/claudex/sets.lock.json`
- 项目：`.claudex/sets.lock.json`

内容：每个已安装的配置集的来源、版本、commit SHA、安装时间、安装了哪些组件。

## 6. MCP 安装方式

放弃 `setup-mcp.sh` 脚本方式。配置集在 `.claudex-sets.json` 中声明式定义 MCP 服务器，claudex 通过以下方式安装：

- 全局：调用 `claude mcp add -s user ...` 或直接写入 `~/.claude.json`
- 项目：调用 `claude mcp add -s project ...` 或直接写入 `.claude.json`

环境变量通过 `${VAR_NAME}` 占位符引用，安装时交互提示用户输入。

## 7. 冲突处理

安装组件时检测目标文件是否已存在。存在则进入交互选择：

- CLAUDE.md：替换 / 追加到末尾 / 插入到开头 / 跳过 / 查看差异
- rules：替换 / 跳过 / 查看差异
- skills：替换 / 跳过 / 查看差异
- MCP servers：同名则更新 / 跳过

## 8. 来源类型处理

| 来源 | 识别方式 | 处理 |
|------|---------|------|
| Git 仓库 | 以 `.git` 结尾或包含 `github.com`/`gitlab.com` 等 | clone 到缓存目录 |
| 本地路径 | 以 `/` 或 `./` 或 `~` 开头，且是目录 | 直接读取 |
| URL | 以 `http://` 或 `https://` 开头（非 git） | 下载到缓存目录 |

所有来源解析完成后，统一检查目标目录下是否存在 `.claudex-sets.json` 或 `claudex-sets.json`。
