# Research: Config 子命令 + TUI 完整实现

## 一、Config 子命令现状

### 当前实现
`Config` 是扁平 flag 结构，只有两个功能：
- `claudex config` → 打印配置摘要
- `claudex config --init [--yaml]` → 在 CWD 创建 config 文件

### 配置系统能力（已有但未暴露为 CLI）
| 能力 | 已有代码 | CLI 暴露 |
|------|---------|---------|
| 多层发现（global/project/env） | `discover_config()` | 无 |
| 从路径加载 | `load_from(path)` | 仅 `$CLAUDEX_CONFIG` |
| 保存（TOML/YAML） | `save()` | 无直接 CLI |
| 格式转换 | `ConfigFormat::from_path()` | 无 |
| 配置校验 | figment 解析时隐式校验 | 无 |
| Profile 查找/过滤 | `find_profile()`, `enabled_profiles()` | 仅 `profile` 子命令 |

### 缺失的 config 操作
1. 指定配置路径加载（全局 `--config`）
2. 查看发现搜索顺序和各层来源
3. 强制重建配置文件
4. 打开编辑器编辑
5. 校验配置语法和语义
6. 读取/修改单个配置值
7. 格式导出/转换
8. 配置版本迁移

---

## 二、TUI 现状

### 已实现（100%）
- Profile 列表导航（j/k/Up/Down）
- 健康指示灯和延迟显示
- 日志面板（tui-logger 集成）
- 指标面板（请求数/Token/延迟/成功率）
- 帮助弹窗（?）
- 启动 Claude（Enter）
- 测试连通性（t）
- 退出（q/Esc/Ctrl+C）

### Stub/占位（0%）
| 功能 | 按键 | 现状 |
|------|------|------|
| 新增 Profile | a | 仅打印日志提示 |
| 编辑 Profile | e | 仅打印日志提示 |
| 删除 Profile | d | 仅打印日志警告 |
| 搜索过滤 | / | 接收输入但不过滤列表 |
| Proxy 控制 | p | 仅显示状态 |

### 架构评估
- **App 状态结构**: 成熟，`Arc<RwLock>` 共享，ProfileSnapshot 缓存
- **事件循环**: tokio select + crossterm EventStream，250ms tick
- **渲染**: ratatui 3 区布局（profiles | logs | metrics | status bar）
- **异步任务**: Option::take() 一次性消费模式

### TUI 缺失功能清单
1. **Profile CRUD 表单**: 新增/编辑 profile 的内联表单或弹窗
2. **删除确认对话框**: 确认后才执行删除
3. **搜索过滤**: search_query 过滤 profile_list
4. **Proxy 启停控制**: 在 TUI 内启动/停止 proxy
5. **Profile 详情面板**: 选中 profile 的完整信息展示
6. **Tab 导航**: 多面板切换（profiles / config / logs）
7. **状态栏动态更新**: 反映当前模式（搜索模式、编辑模式等）
8. **配置编辑**: 在 TUI 中修改配置值
9. **OAuth 状态**: 在 TUI 显示 token 有效性

---

## 三、相关文件清单

| 文件 | 行数 | 修改频率 |
|------|------|---------|
| `src/cli.rs` | 188 | 本次重构核心 |
| `src/config.rs` | 1104 | 新增方法 |
| `src/main.rs` | 293 | dispatch 更新 |
| `src/tui/mod.rs` | ~250 | 状态结构扩展 |
| `src/tui/dashboard.rs` | ~184 | 渲染逻辑大改 |
| `src/tui/input.rs` | ~82 | 输入处理大改 |
| `src/tui/widgets.rs` | ~48 | 新增弹窗/表单组件 |
| `src/profile.rs` | ~200 | 复用现有逻辑 |
| `src/daemon.rs` | ~100 | TUI 调用启停 |

---

## 四、约束和依赖

1. ratatui 0.30 + crossterm 0.29 已在 Cargo.toml
2. 无 tui-textarea 或类似输入组件依赖，表单输入需要自行实现或引入新依赖
3. Profile CRUD 逻辑已在 `profile.rs` 中（interactive_add, remove_profile），可复用
4. daemon.rs 的 `stop_proxy()` 和 `is_proxy_running()` 可直接在 TUI 中调用
5. TUI 中修改 config 后需要调用 `config.save()` 持久化
