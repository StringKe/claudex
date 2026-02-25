# Plan: claudex sets 配置集管理功能

## 方案说明

为 claudex 新增 `sets` 子命令组，实现 Claude Code 配置集的安装、更新、移除、列出。配置集通过 `.claudex-sets.json` 清单文件声明组件，支持 git 仓库、本地路径、URL 三种来源，支持 global 和 project 两种作用域。

## 新增文件

### 1. `schemas/claudex-sets.schema.json`

JSON Schema 定义文件，后续部署到 `https://claudex.space/schemas/sets/v1.json`。

### 2. `src/sets/mod.rs`

模块入口，导出子模块，定义 `SetManifest`（.claudex-sets.json 的 Rust 类型）和 `SetLock`（lock 文件类型）。

### 3. `src/sets/schema.rs`

`SetManifest` 及其嵌套类型的完整 serde 定义：

```rust
pub struct SetManifest {
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub author: Option<String>,
    pub homepage: Option<String>,
    pub license: Option<String>,
    pub components: Components,
    pub env: Vec<EnvVar>,
}

pub struct Components {
    pub claude_md: Option<ClaudeMd>,
    pub rules: Vec<Rule>,
    pub skills: Vec<Skill>,
    pub mcp_servers: Vec<McpServer>,
}

pub struct ClaudeMd { pub path: String }
pub struct Rule { pub name: String, pub path: String, pub description: Option<String> }
pub struct Skill { pub name: String, pub path: String, pub description: Option<String> }

pub struct McpServer {
    pub name: String,
    pub r#type: McpServerType,  // "http" | "stdio"
    pub url: Option<String>,
    pub command: Option<String>,
    pub args: Vec<String>,
    pub headers: HashMap<String, String>,
    pub env: HashMap<String, String>,
    pub description: Option<String>,
}

pub struct EnvVar {
    pub name: String,
    pub description: Option<String>,
    pub required: bool,
    pub default: Option<String>,
}
```

### 4. `src/sets/source.rs`

来源解析：git clone、本地路径验证、URL 下载。统一返回一个本地目录路径 + 解析后的 `SetManifest`。

```rust
pub enum SetSource {
    Git { url: String, r#ref: Option<String> },
    Local { path: PathBuf },
    Url { url: String },
}

pub fn resolve_source(input: &str) -> Result<SetSource>
pub async fn fetch_source(source: &SetSource, cache_dir: &Path) -> Result<(PathBuf, SetManifest)>
```

### 5. `src/sets/install.rs`

安装逻辑：逐组件安装到目标目录，处理冲突交互，写入 lock 文件。

```rust
pub struct InstallContext {
    pub scope: Scope,           // Global | Project
    pub manifest: SetManifest,
    pub source_dir: PathBuf,    // 配置集文件所在目录
    pub target_claude_dir: PathBuf,  // ~/.claude/ 或 .claude/
    pub env_values: HashMap<String, String>,  // 用户填入的环境变量
}

pub enum Scope { Global, Project }

pub async fn install_set(ctx: &InstallContext) -> Result<InstallResult>
pub async fn uninstall_set(scope: Scope, name: &str) -> Result<()>
```

### 6. `src/sets/lock.rs`

Lock 文件读写：

```rust
pub struct SetsLockFile {
    pub sets: Vec<LockedSet>,
}

pub struct LockedSet {
    pub name: String,
    pub source: String,          // 原始输入（git URL / 本地路径 / URL）
    pub source_type: String,     // "git" | "local" | "url"
    pub version: String,         // 来自 manifest
    pub locked_ref: Option<String>,  // git commit SHA（git 来源时）
    pub pinned: bool,            // true = 锁定版本，false = latest
    pub installed_components: InstalledComponents,
    pub installed_at: String,    // ISO 8601
    pub updated_at: String,      // ISO 8601
}

pub struct InstalledComponents {
    pub claude_md: bool,
    pub rules: Vec<String>,     // 已安装的规则名
    pub skills: Vec<String>,    // 已安装的技能名
    pub mcp_servers: Vec<String>, // 已安装的 MCP 名
}
```

Lock 文件路径：
- Global: `~/.config/claudex/sets.lock.json`
- Project: `.claudex/sets.lock.json`

### 7. `src/sets/conflict.rs`

冲突检测与交互解决：

```rust
pub enum ConflictResolution {
    Replace,
    Append,
    Prepend,
    Skip,
    ViewDiff,
}

pub fn check_conflict(target: &Path) -> bool
pub fn resolve_conflict(source: &Path, target: &Path) -> Result<ConflictResolution>
pub fn apply_resolution(source: &Path, target: &Path, resolution: ConflictResolution) -> Result<()>
```

### 8. `src/sets/mcp.rs`

MCP 服务器安装：解析 `${VAR}` 占位符，直接读写 `~/.claude.json`（global）或 `.claude.json`（project）的 `mcpServers` 字段。不调用 `claude mcp add` 命令。

`claude mcp add` 本身也只是写文件，直接操作 JSON 更可控、无外部依赖。

```rust
/// 读取目标 claude.json，合并 mcpServers 字段，写回
pub fn install_mcp_server(server: &McpServer, scope: Scope, env_values: &HashMap<String, String>) -> Result<()>
/// 从目标 claude.json 的 mcpServers 中移除指定 name
pub fn uninstall_mcp_server(name: &str, scope: Scope) -> Result<()>
/// 读取 claude.json 并返回其中的 mcpServers 段
fn read_claude_json(path: &Path) -> Result<serde_json::Value>
/// 写回 claude.json，保留非 mcpServers 的其他字段不变
fn write_claude_json(path: &Path, value: &serde_json::Value) -> Result<()>
```

目标文件路径：
- Global: `~/.claude.json`
- Project: `<project_root>/.claude.json`

写入格式与 `claude mcp add` 产出一致：

```json
{
  "mcpServers": {
    "context7": {
      "type": "http",
      "url": "https://mcp.context7.com/mcp",
      "headers": { "CONTEXT7_API_KEY": "实际值" }
    },
    "perplexity": {
      "type": "stdio",
      "command": "npx",
      "args": ["-y", "@perplexity-ai/mcp-server"],
      "env": { "PERPLEXITY_API_KEY": "实际值" }
    }
  }
}
```

`${VAR}` 占位符在写入时替换为用户提供的实际值。文件已存在时，只合并 `mcpServers` 字段，保留其他所有字段不变。

## 修改文件

### 9. `src/cli.rs`

新增 `Sets` variant 和 `SetsAction` enum：

```rust
// Commands enum 新增：
Sets {
    #[command(subcommand)]
    action: SetsAction,
},

#[derive(Subcommand)]
pub enum SetsAction {
    /// 安装配置集
    Add {
        /// 配置集来源（git URL、本地路径、URL）
        source: String,
        /// 全局安装
        #[arg(long)]
        global: bool,
        /// 锁定到指定 git ref（tag/branch/commit）
        #[arg(long)]
        r#ref: Option<String>,
    },
    /// 移除已安装的配置集
    Remove {
        /// 配置集名称
        name: String,
        /// 从全局移除
        #[arg(long)]
        global: bool,
    },
    /// 列出已安装的配置集
    List {
        /// 列出全局配置集
        #[arg(long)]
        global: bool,
    },
    /// 更新配置集到最新版本
    Update {
        /// 配置集名称（省略则更新全部）
        name: Option<String>,
        /// 更新全局配置集
        #[arg(long)]
        global: bool,
    },
    /// 显示配置集详情
    Show {
        /// 配置集名称
        name: String,
        /// 查看全局配置集
        #[arg(long)]
        global: bool,
    },
}
```

### 10. `src/main.rs`

新增 `mod sets;` 和分发逻辑：

```rust
Some(Commands::Sets { action }) => match action {
    SetsAction::Add { source, global, r#ref } => {
        sets::add(&source, global, r#ref.as_deref()).await?
    }
    SetsAction::Remove { name, global } => {
        sets::remove(&name, global).await?
    }
    SetsAction::List { global } => {
        sets::list(global).await?
    }
    SetsAction::Update { name, global } => {
        sets::update(name.as_deref(), global).await?
    }
    SetsAction::Show { name, global } => {
        sets::show(&name, global).await?
    }
},
```

## Schema 文件内容

`schemas/claudex-sets.schema.json`，部署 URL：`https://claudex.space/schemas/sets/v1.json`

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "https://claudex.space/schemas/sets/v1.json",
  "title": "Claudex Configuration Set",
  "description": "Claudex 配置集清单文件",
  "type": "object",
  "required": ["name", "version", "components"],
  "additionalProperties": false,
  "properties": {
    "$schema": { "type": "string" },
    "name": {
      "type": "string",
      "pattern": "^[a-z0-9][a-z0-9._-]*$",
      "maxLength": 64
    },
    "version": {
      "type": "string",
      "pattern": "^\\d+\\.\\d+\\.\\d+"
    },
    "description": { "type": "string", "maxLength": 256 },
    "author": { "type": "string" },
    "homepage": { "type": "string", "format": "uri" },
    "license": { "type": "string" },
    "components": {
      "type": "object",
      "additionalProperties": false,
      "properties": {
        "claude_md": {
          "type": "object",
          "required": ["path"],
          "additionalProperties": false,
          "properties": {
            "path": { "type": "string" }
          }
        },
        "rules": {
          "type": "array",
          "items": {
            "type": "object",
            "required": ["name", "path"],
            "additionalProperties": false,
            "properties": {
              "name": { "type": "string" },
              "path": { "type": "string" },
              "description": { "type": "string" }
            }
          }
        },
        "skills": {
          "type": "array",
          "items": {
            "type": "object",
            "required": ["name", "path"],
            "additionalProperties": false,
            "properties": {
              "name": { "type": "string" },
              "path": { "type": "string" },
              "description": { "type": "string" }
            }
          }
        },
        "mcp_servers": {
          "type": "array",
          "items": {
            "type": "object",
            "required": ["name", "type"],
            "additionalProperties": false,
            "properties": {
              "name": { "type": "string" },
              "type": { "enum": ["http", "stdio"] },
              "url": { "type": "string", "format": "uri" },
              "command": { "type": "string" },
              "args": {
                "type": "array",
                "items": { "type": "string" }
              },
              "headers": {
                "type": "object",
                "additionalProperties": { "type": "string" }
              },
              "env": {
                "type": "object",
                "additionalProperties": { "type": "string" }
              },
              "description": { "type": "string" }
            },
            "if": {
              "properties": { "type": { "const": "http" } }
            },
            "then": { "required": ["url"] },
            "else": { "required": ["command"] }
          }
        }
      }
    },
    "env": {
      "type": "array",
      "items": {
        "type": "object",
        "required": ["name"],
        "additionalProperties": false,
        "properties": {
          "name": { "type": "string" },
          "description": { "type": "string" },
          "required": { "type": "boolean", "default": false },
          "default": { "type": "string" }
        }
      }
    }
  }
}
```

## 目录结构总览

安装后的文件系统布局：

```
# Global
~/.config/claudex/
├── config.toml              # 现有配置
├── sets.lock.json           # 全局 lock 文件
└── sets/                    # 配置集缓存
    └── ai-agents/           # clone/下载的配置集
        ├── .claudex-sets.json
        ├── CLAUDE.md
        ├── rules/
        └── skills/

~/.claude/                   # Claude Code 全局配置（安装目标）
├── CLAUDE.md                # ← 来自配置集
├── rules/
│   └── sdlc-workflow.md     # ← 来自配置集
└── skills/
    ├── done/SKILL.md        # ← 来自配置集
    └── sdlc-workflow/SKILL.md

~/.claude.json               # MCP servers (user scope) ← 来自配置集

# Project
<project>/
├── .claudex/
│   ├── sets.lock.json       # 项目 lock 文件
│   └── sets/                # 项目级配置集缓存
│       └── my-set/
└── .claude/                 # Claude Code 项目配置（安装目标）
    ├── CLAUDE.md
    ├── rules/
    └── skills/
```

## 安装流程

```
claudex sets add [--global] [--ref <ref>] <source>
  │
  ├─ 1. resolve_source(source) → SetSource
  ├─ 2. fetch_source(source, cache_dir) → (dir, manifest)
  │     ├─ Git: git clone [--branch ref] → cache_dir/name/
  │     ├─ Local: 验证路径存在
  │     └─ URL: reqwest download → cache_dir/name/
  │
  ├─ 3. 验证 manifest（name/version/components 字段合法性）
  │
  ├─ 4. 读取 lock 文件，检查是否已安装同名 set
  │     ├─ 已安装且版本相同 → 提示已是最新
  │     └─ 已安装但版本不同 → 提示将更新
  │
  ├─ 5. 处理环境变量
  │     ├─ 遍历 manifest.env
  │     ├─ 检查 $ENV 是否已设置
  │     ├─ 未设置且 required → 交互提示输入
  │     └─ 未设置且非 required → 提示可选，跳过
  │
  ├─ 6. 逐组件安装
  │     ├─ claude_md: 复制到 target/.claude/CLAUDE.md（冲突交互）
  │     ├─ rules: 逐个复制到 target/.claude/rules/（冲突交互）
  │     ├─ skills: 逐个复制目录到 target/.claude/skills/（冲突交互）
  │     └─ mcp_servers: 直接写入目标 claude.json 的 mcpServers 字段（同名则交互确认覆盖/跳过）
  │
  ├─ 7. 写入 lock 文件
  │     ├─ 记录 source, version, git SHA（如适用）
  │     ├─ 记录 pinned = (ref != None)
  │     └─ 记录已安装的组件列表
  │
  └─ 8. 输出安装摘要
```

## 更新流程

```
claudex sets update [--global] [name]
  │
  ├─ 1. 读取 lock 文件
  ├─ 2. 筛选：指定 name 则只更新该 set，否则更新全部
  ├─ 3. 跳过 pinned=true 的 set（除非指定 --force）
  ├─ 4. 对每个 set：
  │     ├─ Git: git fetch + git log HEAD..origin/main 检查更新
  │     ├─ Local: 比较 manifest version
  │     └─ URL: 重新下载比较
  ├─ 5. 有更新则重新执行安装流程（同上步骤 3-8）
  └─ 6. 更新 lock 文件中的 version/SHA/updated_at
```

## 移除流程

```
claudex sets remove [--global] <name>
  │
  ├─ 1. 读取 lock 文件，找到对应 set
  ├─ 2. 根据 installed_components 逐个移除：
  │     ├─ claude_md: 提示用户确认删除（因为可能已被手动修改）
  │     ├─ rules: 删除对应文件
  │     ├─ skills: 删除对应目录
  │     └─ mcp_servers: 从目标 claude.json 的 mcpServers 中移除对应条目
  ├─ 3. 删除缓存目录
  ├─ 4. 从 lock 文件移除记录
  └─ 5. 输出移除摘要
```

## 考量与权衡

### 为什么 lock 文件用 JSON 而非 TOML

- 与 `.claudex-sets.json` 保持一致
- serde_json 已是项目依赖
- Lock 文件是工具生成的，JSON 更适合机器读写

### MCP 安装方式：直接写 JSON

直接读写 `~/.claude.json` / `.claude.json` 的 `mcpServers` 字段：
- `claude mcp add` 本身也只是写这个文件，没有额外逻辑
- 直接操作避免了对 `claude` CLI 的运行时依赖
- 读取现有文件 → 合并 mcpServers → 写回，保留其他字段不变

### 本地路径 set 的 update 语义

本地路径指向的 set 没有版本控制。update 时比较 manifest 中的 version 字段：
- 版本号变了 → 重新安装
- 版本号没变 → 跳过（除非 --force）

### 冲突交互的 --yes 模式

后续可加 `--yes` flag 跳过所有交互，默认 replace。初版先做交互式。

## Todo List

### Phase 1: Schema 与基础类型
- [ ] 创建 `schemas/claudex-sets.schema.json`
- [ ] 创建 `src/sets/mod.rs` 模块入口
- [ ] 创建 `src/sets/schema.rs` 定义 SetManifest 等 serde 类型
- [ ] 创建 `src/sets/lock.rs` 定义 SetsLockFile 等 serde 类型

### Phase 2: CLI 定义与分发
- [ ] 修改 `src/cli.rs` 新增 `Sets` command 和 `SetsAction` enum
- [ ] 修改 `src/main.rs` 新增 `mod sets` 和分发逻辑

### Phase 3: 来源解析
- [ ] 创建 `src/sets/source.rs` 实现 `resolve_source()` 和 `fetch_source()`
- [ ] 实现 Git clone 逻辑（支持 --ref）
- [ ] 实现本地路径验证
- [ ] 实现 URL 下载逻辑

### Phase 4: 安装引擎
- [ ] 创建 `src/sets/conflict.rs` 冲突检测与交互
- [ ] 创建 `src/sets/mcp.rs` MCP 服务器安装/卸载
- [ ] 创建 `src/sets/install.rs` 完整安装/卸载逻辑

### Phase 5: 命令实现
- [ ] 实现 `sets add` 命令
- [ ] 实现 `sets remove` 命令
- [ ] 实现 `sets list` 命令
- [ ] 实现 `sets update` 命令
- [ ] 实现 `sets show` 命令

### Phase 6: 验证
- [ ] cargo check 通过
- [ ] cargo clippy 通过
- [ ] 用 ai-agents 仓库手动测试完整流程

### Phase 7: ai-agents 仓库适配
- [ ] 为 `/Users/chen/Code/ai-agents` 生成 `.claudex-sets.json`
