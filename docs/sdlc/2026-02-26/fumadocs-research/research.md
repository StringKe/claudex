# Fumadocs 调研报告

> 调研日期: 2026-02-26
> 信息来源: fumadocs.dev 官方文档, GitHub API, npm, Perplexity 搜索

## 1. 项目概况

| 指标 | 数据 |
|------|------|
| GitHub Stars | 10,915 |
| Forks | 611 |
| Open Issues | 7 |
| 最新版本 | fumadocs-core 16.6.5 |
| 最后更新 | 2026-02-26 (活跃维护) |
| 定位 | Next.js 文档框架, 也支持 React Router / TanStack Start / Waku |
| 作者 | fuma-nama |
| 仓库 | https://github.com/fuma-nama/fumadocs |

Fumadocs 从诞生之初就原生支持 Next.js App Router, 采用 headless 核心 + UI 层分离的架构。核心包 `fumadocs-core` 提供路由、i18n、搜索等基础能力, `fumadocs-ui` 提供开箱即用的 UI 组件（基于 Tailwind + Radix, 类 shadcn 风格）。

## 2. 多语言（i18n）支持

### 2.1 内置 i18n 方案

Fumadocs 提供 `fumadocs-core/i18n` 模块, 负责文档内容的多语言路由和加载。注意官方明确说明: Fumadocs 不是完整的 i18n 库, 它只处理文档部分的国际化, 应用其他部分（如 UI 字符串）需要配合第三方库。

核心 API:

```typescript
// lib/i18n.ts
import { defineI18n } from "fumadocs-core/i18n";

export const i18n = defineI18n({
  defaultLanguage: "en",
  languages: ["en", "zh", "ja"],
  hideLocale: "default-locale", // 默认语言不显示 URL 前缀
  // parser: 内部处理, 支持目录模式和文件后缀模式
});
```

`defineI18n` 选项:

| 选项 | 类型 | 说明 |
|------|------|------|
| `defaultLanguage` | `string` | 默认语言 |
| `languages` | `string[]` | 支持的语言列表 |
| `hideLocale` | `"default-locale" \| "never" \| "non-default"` | URL 前缀策略 |
| `fallbackLanguage` | `string` | 缺失翻译时的回退语言, 默认为 `defaultLanguage` |

### 2.2 翻译文件管理

两种内容组织策略:

**目录模式** (`parser: "dir"`):
```
content/docs/
├── en/
│   ├── index.mdx
│   └── getting-started.mdx
├── zh/
│   ├── index.mdx
│   └── getting-started.mdx
```

**文件后缀模式** (`parser: "suffix"`):
```
content/docs/
├── index.en.mdx
├── index.zh.mdx
├── getting-started.en.mdx
├── getting-started.zh.mdx
```

Source loader 配置:

```typescript
// lib/source.ts
import { loader } from "fumadocs-core/source";
import { createMDXSource } from "fumadocs-mdx";
import { docs, meta } from "@/.source";
import { i18n } from "@/lib/i18n";

export const source = loader({
  source: createMDXSource(docs, meta),
  baseUrl: "/docs",
  i18n, // 必须用 i18n, 不是 mdxi18n
});
```

页面中按语言获取内容:

```typescript
const page = source.getPage(slug, lang); // locale-aware lookup
```

### 2.3 Middleware 配置

```typescript
// middleware.ts
import { createI18nMiddleware } from "fumadocs-core/i18n/middleware";
import { i18n } from "@/lib/i18n";

export default createI18nMiddleware(i18n);

export const config = {
  matcher: ["/((?!api|_next/static|_next/image|favicon.ico).*)"],
};
```

### 2.4 App Router 路由结构

```
app/
├── [lang]/
│   ├── docs/[[...slug]]/page.tsx   # 文档页
│   ├── docs/layout.tsx             # DocsLayout
│   ├── (home)/page.tsx             # 自定义页面
│   └── layout.tsx                  # I18nProvider
└── layout.tsx                      # 根布局
```

### 2.5 与 next-intl 集成

可以共存: next-intl 管应用级 UI 字符串, Fumadocs 管文档内容。

- Middleware 需要手动编排: next-intl middleware 处理非文档路径, Fumadocs middleware 处理文档路径, 或串联调用
- Provider 嵌套: Fumadocs 的 `I18nProvider` 可以套在 next-intl 的 provider 外层或内层
- 页面 params: 统一使用 `params.lang`
- 已知坑: source loader 中必须传 `i18n` 而非 `mdxi18n`, 否则 locale 检测失败

v13+ 更新: `I18nProvider` 需要显式传入 `locale` prop。

### 2.6 i18n 评估

| 维度 | 评价 |
|------|------|
| 文档内容多语言 | 原生支持, 两种文件组织方式 |
| UI 字符串翻译 | 需配合 next-intl 等第三方库 |
| 路由 | 自动处理 `[lang]` 前缀, 支持隐藏默认语言前缀 |
| 搜索 | 需针对多语言单独配置索引（Orama/Algolia） |
| 成熟度 | 基本可用, 但 GitHub discussions 中有用户报告集成问题 |

## 3. 自定义页面能力

### 3.1 布局系统

Fumadocs UI 提供多种预置布局:

| 布局类型 | 用途 | 自定义度 |
|----------|------|---------|
| `DocsLayout` | 文档页面, 含侧边栏、TOC、导航 | 高: sidebar/banner/tabs/footer 均可定制 |
| `HomeLayout` | 着陆页/自定义页面, 仅导航栏 | 中: nav links、共享选项 |
| `NotebookLayout` | 紧凑型文档变体 | 较低, 偏自用 |
| `FluxLayout` | 极简/实验性文档布局 | 较低 |

布局系统基于 CSS Grid, 使用 CSS custom properties (`--fd-docs-row-*`, `--fd-*-width`) 控制尺寸, 支持动画侧边栏折叠。

### 3.2 创建非文档页面

完全基于 Next.js App Router 的 route groups:

```
app/
├── [lang]/
│   ├── (home)/              # 自定义页面组
│   │   ├── layout.tsx       # HomeLayout (仅导航栏 + 搜索)
│   │   ├── page.tsx         # 着陆页
│   │   └── about/page.tsx   # 关于页面
│   └── docs/                # 文档页面组
│       ├── layout.tsx       # DocsLayout (侧边栏 + TOC)
│       └── [[...slug]]/page.tsx
```

可以在 `lib/layout.shared.tsx` 中抽取共享的 `baseOptions`（导航栏、搜索等）, 在不同布局间复用。

### 3.3 Headless 模式

`fumadocs-core` 可以完全脱离 `fumadocs-ui` 使用, 只提供路由/搜索/i18n 的基础能力, UI 完全自定义。这意味着你可以用任何 UI 框架/组件库来构建界面。

### 3.4 灵活度评估

这是 Fumadocs 相比 Nextra 的核心优势。由于非自用（not opinionated）, 它能很好地嵌入已有 Next.js 项目, 或者在文档之外构建复杂的自定义页面。

## 4. Cloudflare 部署

### 4.1 Static Export

通过 Next.js 的 `output: 'export'` 实现:

```javascript
// next.config.mjs
const nextConfig = {
  output: 'export',
};
```

Static export 后可以部署到任何 CDN, 包括 Cloudflare Pages。

搜索行为: 内置搜索在 static 模式下会将索引存储为静态文件, 在浏览器端执行搜索计算, 无需远程服务器。

### 4.2 Cloudflare 非 Static 部署

官方明确声明: **Fumadocs 不支持 Edge Runtime**。

对于需要 SSR 的 Cloudflare 部署, 必须使用 OpenNext (`@opennext.js.org/cloudflare`) 作为适配器。

### 4.3 部署方案对比

| 方案 | 可行性 | 限制 |
|------|--------|------|
| Static Export + Cloudflare Pages | 可行 | 无 SSR, 无 API routes, 所有页面在构建时生成 |
| OpenNext + Cloudflare Workers | 可行 | 需要额外适配层, 配置更复杂 |
| Docker + 自托管 | 可行 | 需要 Node.js 服务器 |
| Vercel | 原生支持 | 最简单的选择 |

### 4.4 Static Export 的限制

- 所有 server-side loaders 必须 pre-render
- 不支持 dynamic routes（除非 `generateStaticParams` 覆盖所有路径）
- 不支持 API routes
- 不支持 ISR (Incremental Static Regeneration)

## 5. 生态与成熟度

### 5.1 核心数据

| 指标 | Fumadocs | Nextra |
|------|----------|--------|
| GitHub Stars | ~10,900 | ~12,200+ |
| npm 最新版本 | fumadocs-core 16.6.5 | nextra 4.x |
| App Router 支持 | 原生, 从诞生之初 | v4 加入 (2025.01) |
| 维护状态 | 活跃, 单作者为主 | 活跃, The Guild 维护 |
| Open Issues | 7 | 较多 |

### 5.2 npm 下载量趋势

Fumadocs 的 npm 下载量低于 Nextra, 但增长趋势明显。在文档框架领域, Docusaurus 和 VitePress 仍占主导, Fumadocs 和 Nextra 在 Next.js 生态中竞争。

### 5.3 谁在用

Fumadocs 官网展示了一些采用案例, 但整体用户群体规模小于 Nextra。它在 API/SDK 文档场景中的 OpenAPI 集成和 TypeScript 文档生成能力是独特卖点。

## 6. Fumadocs vs Nextra 详细对比

### 6.1 设计哲学

| 维度 | Fumadocs | Nextra |
|------|----------|--------|
| 设计理念 | 灵活优先, headless 核心 + 可选 UI | 自用优先 (opinionated), 开箱即用 |
| App Router | 从诞生就原生支持 | v4 (2025.01) 才完整支持, 之前是 Pages Router |
| 架构 | `fumadocs-core` + `fumadocs-ui` 分层 | `nextra` + `nextra-theme-docs` 耦合较紧 |

### 6.2 功能特性对比

| 功能 | Fumadocs | Nextra |
|------|----------|--------|
| Static Generation | 支持 | 支持 |
| Caching | 支持 | 支持 |
| Light/Dark Mode | 支持 | 支持 |
| Syntax Highlighting | 支持 | 支持 |
| Table of Contents | 支持 | 支持 |
| Full-text Search | 支持 (内置 + 云) | 支持 |
| i18n | 支持 | 支持 |
| Last Git Edit Time | 支持 | 支持 |
| Page Icons | 支持 | 支持 |
| RSC | 支持 | 支持 (v4) |
| Remote Source | 支持 | 支持 |
| Built-in Components | 支持 | 支持 |
| RTL Layout | 支持 | 支持 |
| **OpenAPI Integration** | **支持** | 不支持 |
| **TypeScript Docs Generation** | **支持** | 不支持 |

### 6.3 关键差异

1. **自定义能力**: Fumadocs 的 headless 模式允许完全自定义 UI, Nextra 的 theme 系统更固化
2. **嵌入现有项目**: Fumadocs 更容易嵌入已有 Next.js 项目, Nextra 通常作为独立站点
3. **API 文档**: Fumadocs 内置 OpenAPI spec 渲染, 这是 Nextra 没有的
4. **社区规模**: Nextra 社区更大, 文档/教程更多
5. **i18n + static export**: Nextra 官方文档指出 i18n 与 `output: 'export'` 不兼容; Fumadocs 未提及此限制
6. **稳定性**: Nextra v3 到 v4 经历了大重构 (Pages Router -> App Router), 社区反馈迁移过程痛苦; Fumadocs 一直在 App Router 上, API 更稳定

### 6.4 Fumadocs 作者对 Nextra 的评价

Fumadocs 官网对比页面提到: Fumadocs 的路由约定（如 `meta.json`）受 Nextra 启发。Nextra 更自用, 需要更多手动配置; Fumadocs 更适合需要更大控制权的场景（如嵌入现有代码库或实现高级路由）。

## 7. 综合评估

### 7.1 优势

- App Router 原生支持, 不存在历史包袱
- Headless 架构, 自定义灵活度极高
- OpenAPI + TypeScript 文档生成是差异化特性
- Static export 可用, 部署灵活
- 活跃维护, issue 数量极少（仅 7 个 open issues）
- stars 增长快, 已突破 10k

### 7.2 劣势/风险

- 以单作者为主（fuma-nama）, 存在 bus factor 风险
- 社区规模和生态仍小于 Nextra/Docusaurus
- i18n 不是完整方案, 需要额外配合 next-intl 等库
- Edge Runtime 不支持, Cloudflare Workers SSR 需要 OpenNext 适配
- 部分集成（如 next-intl + Fumadocs middleware 串联）文档不够充分, 社区反馈有坑

### 7.3 适用场景

| 场景 | 推荐度 |
|------|--------|
| 新建纯文档站点（英文为主） | 强烈推荐 |
| 嵌入已有 Next.js 项目 | 强烈推荐（核心优势） |
| 多语言文档站点 | 推荐, 但需注意 middleware 编排 |
| API/SDK 文档 (OpenAPI) | 强烈推荐（差异化优势） |
| Cloudflare Pages (static) | 推荐 |
| Cloudflare Workers (SSR) | 可行但需 OpenNext, 不如 Vercel 简单 |
| 需要极高社区支持/大量插件 | 考虑 Docusaurus |

## 8. 信息来源

- fumadocs.dev 官方文档: /docs/deploying, /docs/deploying/static, /docs/comparisons, /docs/headless/internationalization, /docs/internationalization
- GitHub API: github.com/fuma-nama/fumadocs (stars/forks/issues)
- GitHub Discussions #932: next-intl 集成问题
- npm: fumadocs-core 16.6.5
- Perplexity 搜索聚合
- lingo.dev Fumadocs 集成指南
- Nextra 4 发布博客 (The Guild)
