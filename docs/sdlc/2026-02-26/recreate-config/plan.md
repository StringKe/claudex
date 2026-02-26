# Plan: Config 完整子命令 + TUI 全功能实现

## 方案说明

两大块工作：
1. 将 Config 从扁平 flag 重构为完整子命令体系，覆盖配置生命周期全操作
2. TUI 从只读仪表板升级为全功能管理界面，实现 Profile CRUD、搜索过滤、Proxy 控制、配置编辑

---

## Part A: Config 子命令体系

### A.1 全局 `--config` 选项

在 `Cli` 顶层添加 `--config <path>`，所有子命令可用：

```rust
pub struct Cli {
    #[arg(long, global = true, value_name = "PATH")]
    pub config: Option<PathBuf>,
    #[command(subcommand)]
    pub command: Option<Commands>,
}
```

`ClaudexConfig::load_with_path(path)` 方法：指定路径时直接 `load_from(path)`，否则走 `discover_config()`。

### A.2 ConfigAction 子命令枚举

```rust
Commands::Config {
    #[command(subcommand)]
    action: Option<ConfigAction>,
}

#[derive(Subcommand)]
pub enum ConfigAction {
    /// 显示当前配置摘要
    Show,

    /// 显示配置文件路径和搜索顺序
    Path {
        /// 显示完整搜索路径列表
        #[arg(long)]
        search: bool,
    },

    /// 在当前目录初始化配置文件
    Init {
        #[arg(long)]
        yaml: bool,
    },

    /// 强制重建配置文件（备份原文件，保留 profiles）
    Recreate {
        /// 目标路径（默认当前生效路径）
        #[arg(short, long)]
        output: Option<PathBuf>,
        /// YAML 格式
        #[arg(long)]
        yaml: bool,
        /// 不保留现有 profiles
        #[arg(long)]
        no_keep_profiles: bool,
    },

    /// 用 $EDITOR 打开配置文件编辑
    Edit,

    /// 校验配置文件语法和语义
    Validate,

    /// 读取指定配置值（点号分隔路径）
    Get {
        /// 配置键路径，如 proxy_port, profiles.0.name
        key: String,
    },

    /// 设置指定配置值
    Set {
        /// 配置键路径
        key: String,
        /// 新值
        value: String,
    },

    /// 导出配置为另一种格式
    Export {
        /// 目标格式
        #[arg(long)]
        format: ExportFormat,
        /// 输出路径（默认 stdout）
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
}

#[derive(Clone, clap::ValueEnum)]
pub enum ExportFormat {
    Toml,
    Yaml,
    Json,
}
```

无子命令时等价于 `show`（向后兼容）。

### A.3 各子命令实现细节

#### `config show`（现有逻辑迁移）
打印：config source、profiles 数量、proxy 地址、router/context 状态。

#### `config path [--search]`
- 默认打印当前生效的配置文件路径
- `--search` 列出完整搜索顺序：global 路径、project 搜索的每个候选路径、$CLAUDEX_CONFIG

#### `config init [--yaml]`
现有 `init_local()` 逻辑不变。

#### `config recreate [--output] [--yaml] [--no-keep-profiles]`
1. 备份现有配置为 `.bak`
2. 从 `config.example.toml` 模板重新生成
3. 默认保留 profiles + model_aliases + router + context 配置
4. `--no-keep-profiles` 时全部重来

#### `config edit`
1. 检测 `$EDITOR` 或 `$VISUAL`，fallback 到 `vi`
2. 用 `std::process::Command` 打开配置文件
3. 编辑器退出后重新加载配置做语法校验
4. 校验失败时提示用户

#### `config validate`
1. 加载当前配置文件
2. 检查语法（figment 解析）
3. 检查语义：
   - profiles 名称唯一性
   - base_url 格式
   - backup_providers 引用存在
   - oauth_provider 与 auth_type 一致
   - router.rules 引用的 profile 存在
4. 输出结果：OK 或列出所有问题

#### `config get <key>`
用 serde_json 做中间转换，支持点号路径访问：
```
claudex config get proxy_port        → 13456
claudex config get profiles.0.name   → "grok"
claudex config get router.enabled    → false
```

#### `config set <key> <value>`
1. 解析键路径
2. 修改内存中的 config
3. 调用 `save()` 持久化
4. 打印修改前后的值

#### `config export --format <toml|yaml|json>`
序列化当前配置到指定格式，输出到 stdout 或文件。

---

## Part B: TUI 全功能实现

### B.1 状态结构扩展

```rust
pub struct App {
    // 现有字段保留...

    // 新增：UI 模式状态机
    pub mode: AppMode,

    // 新增：Profile 表单
    pub form: ProfileForm,

    // 新增：确认对话框
    pub confirm_dialog: Option<ConfirmDialog>,

    // 新增：Profile 详情展开
    pub show_detail: bool,

    // 新增：活动面板
    pub active_panel: Panel,

    // 新增：通知消息
    pub notification: Option<(String, Instant)>,
}

pub enum AppMode {
    Normal,
    Search,
    AddProfile,
    EditProfile,
    Confirm,
}

pub enum Panel {
    Profiles,
    Detail,
    Logs,
}

pub struct ProfileForm {
    pub fields: Vec<FormField>,
    pub focused_field: usize,
    pub editing_profile: Option<String>,  // None = 新增, Some = 编辑
}

pub struct FormField {
    pub label: String,
    pub value: String,
    pub field_type: FieldType,
}

pub enum FieldType {
    Text,
    Select(Vec<String>),  // 下拉选择
    Bool,
}

pub struct ConfirmDialog {
    pub message: String,
    pub on_confirm: ConfirmAction,
}

pub enum ConfirmAction {
    DeleteProfile(String),
    StopProxy,
    StartProxy,
}
```

### B.2 布局重构

从固定 3 区变为动态布局：

```
Normal 模式:
┌──────────────────────────────────────────────┐
│  Profiles (35%)         │  Detail/Logs (65%) │
│  [list with health]     │  [profile detail   │
│                         │   or log stream]   │
├──────────────────────────────────────────────┤
│  Metrics bar                                 │
├──────────────────────────────────────────────┤
│  Status bar (dynamic)                        │
└──────────────────────────────────────────────┘

AddProfile/EditProfile 模式:
┌──────────────────────────────────────────────┐
│              Profile Form                     │
│  Name:          [___________]                │
│  Provider:      [DirectAnthropic ▼]          │
│  Base URL:      [___________]                │
│  API Key:       [***********]                │
│  Model:         [___________]                │
│  Enabled:       [x]                          │
│                                              │
│  [Save]  [Cancel]                            │
└──────────────────────────────────────────────┘

Confirm 模式:
┌────────────────────────────┐
│  Delete profile "grok"?    │
│  [Yes]        [No]         │
└────────────────────────────┘
```

### B.3 输入处理重构

按 AppMode 分发：

```rust
fn handle_key_event(app: &mut App, key: KeyEvent) {
    // 通知消息：任意键清除
    if app.notification.is_some() {
        app.notification = None;
        return;
    }

    // 确认对话框
    if app.mode == AppMode::Confirm {
        handle_confirm_input(app, key);
        return;
    }

    match app.mode {
        AppMode::Normal => handle_normal_input(app, key),
        AppMode::Search => handle_search_input(app, key),
        AppMode::AddProfile | AppMode::EditProfile => handle_form_input(app, key),
        AppMode::Confirm => unreachable!(),
    }
}
```

#### Normal 模式按键

| 按键 | 功能 |
|------|------|
| j/Down | 下移 |
| k/Up | 上移 |
| Enter | 启动 Claude |
| t | 测试连通性 |
| a | 进入 AddProfile 模式 |
| e | 进入 EditProfile 模式（加载选中 profile） |
| d | 弹出删除确认 |
| / | 进入搜索模式 |
| ? | 帮助弹窗 |
| Tab | 切换活动面板 |
| Space | 切换 Detail/Logs 面板 |
| p | Proxy 启停（弹确认） |
| o | 显示 OAuth token 状态 |
| q/Esc | 退出 |

#### Search 模式
- 字符输入追加到 search_query
- Backspace 删除
- Enter 确认（保持过滤，退出搜索模式）
- Esc 取消（清空 query，退出搜索模式）
- **实际过滤逻辑**: render_profiles() 中根据 search_query 过滤 profile_list

#### Form 模式（AddProfile / EditProfile）
- Tab / Shift+Tab 切换字段
- 文本字段：字符输入、Backspace、方向键光标
- Select 字段：Up/Down 切换选项
- Bool 字段：Space 切换
- Enter（在最后一个字段）或 Ctrl+S 保存
- Esc 取消返回 Normal

#### Confirm 模式
- y/Enter 确认执行
- n/Esc 取消

### B.4 Profile 表单字段

新增 profile 表单字段：

| 字段 | 类型 | 说明 |
|------|------|------|
| name | Text | profile 名称 |
| provider_type | Select(DirectAnthropic, OpenAICompatible, OpenAIResponses) | provider 类型 |
| base_url | Text | 预填常用 URL |
| api_key | Text | 显示为 `***` |
| default_model | Text | 默认模型 |
| enabled | Bool | 是否启用 |
| priority | Text | 优先级数字 |

编辑模式从选中 profile 加载现有值。

### B.5 Profile 详情面板

选中 profile 时右侧显示详情（替代日志面板，Space 切换）：

```
Profile: grok
────────────────────
Provider:    OpenAICompatible
Base URL:    https://api.x.ai/v1
Model:       grok-3-beta
Priority:    100
Enabled:     yes
Auth:        API Key
API Key:     sk-***...***abc

Models:
  Haiku:     -
  Sonnet:    -
  Opus:      -

Backup:      deepseek
Headers:     (none)
Strip:       auto

Health:      ● healthy (142ms)
Requests:    1,234
Tokens:      56,789
Success:     99.2%
```

### B.6 Proxy 控制

TUI 内 proxy 启停：
- `p` 键触发
- 如果 proxy 运行中 → 弹出确认停止
- 如果 proxy 未运行 → 弹出确认启动
- 确认后调用 `daemon::stop_proxy()` 或 spawn proxy task
- 更新 `proxy_running` 状态

### B.7 搜索过滤实现

在 `render_profiles()` 中添加过滤：

```rust
let filtered: Vec<&ProfileSnapshot> = if app.search_query.is_empty() {
    app.profile_list.iter().collect()
} else {
    let q = app.search_query.to_lowercase();
    app.profile_list.iter()
        .filter(|p| p.name.to_lowercase().contains(&q))
        .collect()
};
```

ListState 选中索引需要映射到过滤后的列表。

### B.8 通知系统

操作完成后显示临时通知（3 秒自动消失）：

```rust
app.notification = Some((
    "Profile 'grok' deleted".to_string(),
    Instant::now(),
));

// render 时检查超时
if let Some((msg, time)) = &app.notification {
    if time.elapsed() > Duration::from_secs(3) {
        app.notification = None;
    }
    // 否则渲染在状态栏
}
```

### B.9 状态栏动态更新

根据当前模式显示不同提示：

```
Normal:   q:Quit  j/k:Nav  Enter:Run  t:Test  a:Add  e:Edit  d:Del  /:Search  ?:Help
Search:   Type to filter | Enter:Confirm | Esc:Cancel | Query: "gro"
Form:     Tab:Next  Shift+Tab:Prev  Ctrl+S:Save  Esc:Cancel
Confirm:  y:Yes  n:No
```

---

## 改动文件

| 文件 | 改动范围 | 说明 |
|------|---------|------|
| `src/cli.rs` | 大改 | 全局 --config、ConfigAction 子命令、ExportFormat enum |
| `src/config.rs` | 中改 | load_with_path、recreate、validate、get/set、export 方法 |
| `src/main.rs` | 中改 | config dispatch 更新、全局 --config 传递 |
| `src/tui/mod.rs` | 大改 | App 状态扩展、AppMode、Panel、新增异步 action 处理 |
| `src/tui/dashboard.rs` | 重写 | 动态布局、detail 面板、表单渲染、确认对话框 |
| `src/tui/input.rs` | 重写 | 多模式输入分发、表单输入、确认输入 |
| `src/tui/widgets.rs` | 大改 | ProfileForm、ConfirmDialog、通知、详情面板组件 |

---

## 考量

1. **表单输入**: 不引入新依赖（tui-textarea），自行实现简单文本输入。字段较少（7 个），复杂度可控
2. **API Key 安全**: 表单中显示为 `***`，编辑时可见，保存后立即遮蔽
3. **config set 类型推断**: 值字符串自动推断类型（数字→u16/u64，true/false→bool，其余→string）
4. **config validate 深度**: 第一版做语法+基础语义检查，不做网络连通性验证
5. **TUI 表单保存**: 保存后调用 `config.save()` 持久化，同时更新内存 config
6. **搜索选中映射**: 过滤列表后维护独立的 filtered_state，避免索引错乱
7. **Proxy 启停**: TUI 内启动 proxy 用 `tokio::spawn`，停止用 `daemon::stop_proxy()`

---

## Todo List

### Phase 1: CLI 基础设施
- [ ] 在 `Cli` 添加全局 `--config` 参数
- [ ] 将 `Config` 重构为 `ConfigAction` 子命令枚举
- [ ] 添加 `ExportFormat` enum
- [ ] 更新 `main.rs` dispatch

### Phase 2: Config 核心方法
- [ ] 实现 `load_with_path()`
- [ ] 实现 `recreate()` 备份+重建
- [ ] 实现 `validate()` 语法+语义检查
- [ ] 实现 `config_search_paths()` 返回搜索顺序

### Phase 3: Config 编辑操作
- [ ] 实现 `config edit` 打开 $EDITOR
- [ ] 实现 `config get` 点号路径读取
- [ ] 实现 `config set` 值修改+保存
- [ ] 实现 `config export` 格式转换

### Phase 4: main.rs 全部 Config dispatch
- [ ] `config show` dispatch
- [ ] `config path` dispatch
- [ ] `config init` dispatch
- [ ] `config recreate` dispatch
- [ ] `config edit` dispatch
- [ ] `config validate` dispatch
- [ ] `config get` dispatch
- [ ] `config set` dispatch
- [ ] `config export` dispatch

### Phase 5: TUI 状态结构
- [ ] 定义 AppMode、Panel、ConfirmAction enum
- [ ] 定义 ProfileForm、FormField、FieldType 结构
- [ ] 定义 ConfirmDialog 结构
- [ ] 扩展 App 状态（mode、form、confirm_dialog、show_detail、active_panel、notification）
- [ ] ProfileSnapshot 扩展（增加 provider_type、base_url、model 等详情字段）

### Phase 6: TUI 输入处理
- [ ] 重构 handle_key_event 为多模式分发
- [ ] 实现 handle_normal_input（含新按键 a/e/d/Tab/Space/p/o）
- [ ] 实现 handle_search_input（含实际过滤逻辑）
- [ ] 实现 handle_form_input（字段导航、文本输入、Select/Bool 切换）
- [ ] 实现 handle_confirm_input（y/n 处理）

### Phase 7: TUI 渲染
- [ ] 重构布局为动态模式（Normal/Form/Confirm）
- [ ] 搜索过滤 render_profiles
- [ ] Profile 详情面板渲染
- [ ] Profile 表单渲染（居中弹窗，字段列表，焦点高亮）
- [ ] 确认对话框渲染
- [ ] 通知消息渲染
- [ ] 动态状态栏

### Phase 8: TUI 业务逻辑
- [ ] Profile 新增：表单保存 → config.profiles.push + config.save
- [ ] Profile 编辑：表单保存 → 更新 config.profiles + config.save
- [ ] Profile 删除：确认后 → config.profiles.retain + config.save
- [ ] Proxy 启停：确认后 → daemon 调用
- [ ] OAuth 状态显示：读取 token 状态

### Phase 9: 编译验证
- [ ] `cargo check` 通过
- [ ] `cargo clippy` 无 warning
- [ ] `cargo fmt`
