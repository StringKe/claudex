# git-cliff 集成方案调研

## 1. 项目现状

### 1.1 Commit 格式

claudex 项目使用中文 conventional commit 格式：

```
<类型>[范围]: <中文描述>
```

实际示例（从 git log 提取）：

```
feat[proxy]: OpenAI Responses API 适配层，支持 ChatGPT/Codex 订阅
fix[proxy]: 4xx 客户端错误透传原始状态码，不触发断路器和重试
fix[pty]: 修复多字节 UTF-8 字符跨 read 边界被截断导致的终端乱码
refactor[proxy]: ProviderAdapter trait 统一三路 forward 逻辑，消除重复代码
ci[release]: 移除 Windows target，简化打包流程
chore: 添加社区文件（issue 模板、PR 模板、贡献指南、行为准则、安全策略）
docs: 文档站更新 OAuth 认证、非交互模式、模型映射等新功能
feat[core]: 功能补齐 + CI/CD + 单元测试
```

特点：
- type 使用英文（feat/fix/docs/refactor/perf/test/build/ci/chore）
- scope 使用英文，用方括号 `[]` 而非圆括号 `()`
- description 使用中文
- scope 可选（部分 commit 没有 scope）

### 1.2 现有 Release Workflow

文件：`.github/workflows/release.yml`

- 触发条件：`push tags: ["v*"]`
- 当前使用 `gh release create "$TAG" --draft --title "$TAG" --generate-notes` 生成 release notes
- `--generate-notes` 是 GitHub 自动生成的，质量一般，不按 commit 类型分组
- 流程：create-release (draft) -> build (matrix) -> publish-release (draft=false)

### 1.3 仓库信息

- GitHub repo: `StringKe/claudex`
- 版本号：当前 `0.1.0`（Cargo.toml）
- 标签格式：`v*`（如 `v0.1.0`）

---

## 2. git-cliff 配置格式（cliff.toml）

### 2.1 核心结构

```toml
[changelog]
header = "..."      # 全局头部模板（可选）
body = "..."        # 每个 release 的 body 模板（核心）
footer = "..."      # 全局尾部模板（可选）
trim = true         # 去除前后空白
render_always = false  # 无 release 时是否仍渲染
postprocessors = [] # 后处理器（正则替换）
output = "CHANGELOG.md"  # 输出文件（可选）

[git]
conventional_commits = true     # 启用 conventional commit 解析
filter_unconventional = true    # 过滤非 conventional commit
protect_breaking_commits = true # 保护 breaking change 不被跳过
split_commits = false           # 不拆分多行 commit
tag_pattern = "v[0-9].*"       # tag 匹配模式
sort_commits = "oldest"         # 排序方式
filter_commits = false          # 不过滤 commit
topo_order = false              # 不使用拓扑排序

commit_preprocessors = [...]    # commit 消息预处理器（在解析前运行）
commit_parsers = [...]          # commit 分类解析器

[remote.github]                 # GitHub 集成（可选）
owner = "StringKe"
repo = "claudex"
token = ""                      # 通过环境变量 GITHUB_TOKEN 传入
```

### 2.2 关键配置项详解

**commit_parsers** 字段：
| 字段 | 类型 | 说明 |
|------|------|------|
| `message` | regex | 匹配 commit message（description 部分） |
| `body` | regex | 匹配 commit body |
| `footer` | regex | 匹配 commit footer |
| `field` | string | 匹配 commit 对象的其他字段（如 `author.name`） |
| `pattern` | regex | 配合 field 使用的正则 |
| `group` | string | 分配到的组名（显示在 changelog 中） |
| `scope` | string | 覆盖 scope |
| `default_scope` | string | 默认 scope |
| `skip` | bool | 是否跳过该 commit |
| `sha` | string | 匹配特定 commit SHA |

**commit_preprocessors** 字段：
| 字段 | 类型 | 说明 |
|------|------|------|
| `pattern` | regex | 匹配模式 |
| `replace` | string | 替换文本 |
| `replace_command` | string | 使用外部命令替换 |

### 2.3 模板引擎（Tera）

body 模板中可用的上下文变量：

```
version          - 版本号（tag 名）
previous.version - 上一个版本号
timestamp        - 时间戳
commits          - commit 列表
  commit.id      - commit SHA
  commit.message - commit 消息
  commit.group   - 分组名
  commit.scope   - scope
  commit.breaking - 是否 breaking change
  commit.remote.username - GitHub 用户名（需 GitHub 集成）
  commit.remote.pr_number - PR 编号
  commit.remote.pr_title - PR 标题
github.contributors - GitHub 贡献者列表（需 GitHub 集成）
```

模板过滤器：
- `group_by(attribute="group")` - 按属性分组
- `trim_start_matches(pat="v")` - 去除前缀
- `upper_first` - 首字母大写
- `split(pat="\n") | first` - 取第一行
- `date(format="%Y-%m-%d")` - 日期格式化

---

## 3. 中文 Commit 格式匹配方案

### 3.1 问题分析

claudex 使用 `类型[范围]: 中文描述` 格式。标准 conventional commit 格式是 `type(scope): description`。

核心差异：使用方括号 `[]` 而非圆括号 `()`。

git-cliff 的 `conventional_commits = true` 默认按 `type(scope): description` 解析。方括号格式需要：
- 方案 A：关闭 `conventional_commits`，完全使用 `commit_parsers` 正则匹配
- 方案 B：使用 `commit_preprocessors` 在解析前将 `[scope]` 转换为 `(scope)`

### 3.2 推荐：方案 B（预处理器转换）

使用 `commit_preprocessors` 将方括号转为圆括号后，启用 `conventional_commits = true`，让 git-cliff 原生解析 conventional commit 格式。

```toml
commit_preprocessors = [
  # 将 type[scope]: 转换为 type(scope):
  { pattern = '^([a-z]+)\[([^\]]+)\]', replace = '${1}(${2})' },
]
```

优点：
- 保留 conventional commit 原生解析能力（自动提取 type、scope、description、breaking change）
- commit_parsers 只需按 type 分组，无需重复写正则
- 支持 `!` breaking change 标记（如 `feat[api]!: 破坏性变更`）

### 3.3 备选：方案 A（纯正则）

```toml
[git]
conventional_commits = false
filter_unconventional = false

commit_parsers = [
  { message = "^feat", group = "Features" },
  { message = "^fix", group = "Bug Fixes" },
  # ...
]
```

缺点：scope 不会被自动提取，需要手动在模板中处理。

---

## 4. GitHub Actions 集成

### 4.1 orhun/git-cliff-action

最新版本引用方式：`orhun/git-cliff-action@v4`（推荐用 tag 而非 SHA）

**Inputs：**
| 输入 | 默认值 | 说明 |
|------|--------|------|
| `config` | `cliff.toml` | 配置文件路径 |
| `args` | `-v` | 传给 git-cliff 的参数 |
| `version` | `latest` | git-cliff 版本 |
| `github_token` | `${{ github.token }}` | GitHub API token |

**Outputs：**
| 输出 | 说明 |
|------|------|
| `changelog` | 生成的 changelog 文件路径 |
| `content` | changelog 文本内容 |
| `version` | 检测到的最新版本号 |

### 4.2 关键 CLI 参数

| 参数 | 说明 |
|------|------|
| `--latest` | 只生成最新 tag 到上一个 tag 之间的 changelog |
| `--unreleased` | 只生成未发布的变更 |
| `--no-exec` | 不执行 postprocessor 中的外部命令（安全） |
| `--github-repo OWNER/REPO` | 指定 GitHub 仓库（覆盖 cliff.toml） |
| `--github-token TOKEN` | GitHub API token（覆盖 cliff.toml） |
| `-vv` | 详细日志 |
| `--strip header` | 去除 header（用于 release notes 时不需要全局标题） |

### 4.3 集成到现有 release.yml 的方案

现有流程：

```
create-release (draft, --generate-notes)
  -> build (matrix)
    -> publish-release (draft=false)
```

改造后流程：

```
generate-changelog (git-cliff)
  -> create-release (draft, 使用 git-cliff 输出)
    -> build (matrix)
      -> publish-release (draft=false)
```

核心改动：
1. 在 `create-release` job 前新增 `generate-changelog` step（或合并到同一 job）
2. 用 `orhun/git-cliff-action` 生成 release notes
3. 将 `--generate-notes` 替换为 `--notes-file` 或 `--notes "$CONTENT"`

### 4.4 checkout 注意事项

**必须使用 `fetch-depth: 0`**，否则 git-cliff 无法获取完整 git 历史，会生成空 changelog。

```yaml
- uses: actions/checkout@v4
  with:
    fetch-depth: 0
```

---

## 5. 内置模板选择

git-cliff 提供多种内置模板：

| 模板 | 说明 | 适用场景 |
|------|------|----------|
| `keepachangelog` | Keep a Changelog 格式 | 生成 CHANGELOG.md 文件 |
| `github` | GitHub Release 风格 | 生成 release notes |
| `github-keepachangelog` | 两者结合 | 同时适用 |
| `scoped` | 按 scope 分组 | scope 丰富的项目 |
| `detailed` | 含 commit 链接 | 需要详细追溯 |
| `minimal` | 极简风格 | 小项目 |

claudex 项目建议：自定义模板，结合 keepachangelog 的分组方式 + GitHub release 的贡献者信息。

---

## 6. 总结与建议

1. **配置文件**：项目根目录创建 `cliff.toml`
2. **预处理器**：将 `[scope]` 转换为 `(scope)` 以兼容 conventional commit 解析
3. **分组**：按 commit type 分组，中文组名（新功能/Bug 修复/...）
4. **GitHub 集成**：启用 `[remote.github]`，在 release notes 中显示 PR 链接和贡献者
5. **Workflow 改造**：将 `--generate-notes` 替换为 git-cliff 生成的内容
6. **本地使用**：开发者可用 `git cliff` 预览 changelog
