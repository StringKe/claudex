# Research: Website SEO + Logo 设计

## 1. 现状分析

### 1.1 当前 SEO 配置

网站基于 Astro 5.7 + Starlight 0.34，部署在 Cloudflare Workers（claudex.space）。

**Starlight 自动生成的 meta tags（无需手动处理）：**

| Tag | 值 | 来源 |
|-----|------|------|
| `<meta charset="utf-8">` | 固定 | headDefaults |
| `<meta name="viewport">` | `width=device-width, initial-scale=1` | headDefaults |
| `<title>` | `{页面title} \| Claudex` | headDefaults |
| `<link rel="canonical">` | 动态生成每页 URL | headDefaults（需 `site` 配置） |
| `<meta name="description">` | 页面 frontmatter description | headDefaults（有值时） |
| `<meta property="og:title">` | 页面 title | headDefaults |
| `<meta property="og:type">` | `article` | headDefaults |
| `<meta property="og:url">` | canonical URL | headDefaults |
| `<meta property="og:locale">` | 当前 lang | headDefaults |
| `<meta property="og:description">` | 页面描述 | headDefaults |
| `<meta property="og:site_name">` | `Claudex` | headDefaults |
| `<meta name="twitter:card">` | `summary_large_image` | headDefaults |
| `<link rel="alternate" hreflang>` | 每个 locale 一条 | headDefaults（多语言时） |
| `<link rel="sitemap">` | sitemap-index.xml | headDefaults（有 `site` 时） |

**astro.config.mjs 手动配置的 head tags（有问题）：**

```js
head: [
  // 与 Starlight 默认重复，会覆盖自动生成的
  { tag: 'meta', attrs: { property: 'og:site_name', content: 'Claudex' } },
  // 指向 @anthropic，应该是 @StringKe
  { tag: 'meta', attrs: { name: 'twitter:site', content: '@anthropic' } },
  // keywords 可保留
  { tag: 'meta', attrs: { name: 'keywords', content: '...' } },
  // 硬编码根域，覆盖了 Starlight 按页动态生成的 canonical，严重 SEO 错误
  { tag: 'link', attrs: { rel: 'canonical', href: 'https://claudex.space' } },
]
```

**完全缺失的关键 tags：**

| Tag | 影响 |
|-----|------|
| `og:image` | 所有社交平台分享无图片 |
| `og:image:width` / `og:image:height` | 爬虫无法预取图片尺寸 |
| `twitter:image` | Twitter/X 卡片无图片（虽设了 summary_large_image） |
| `twitter:site` | 指向错误账号 |
| `twitter:creator` | 缺失 |
| JSON-LD 结构化数据 | Google 搜索无富媒体结果 |
| `theme-color` | 移动浏览器地址栏颜色 |
| Apple touch icon | iOS 保存到主屏幕无图标 |

### 1.2 当前 Logo

`public/favicon.svg`：蓝色字母 C 放在深色圆角矩形上，过于简陋。

```svg
<svg viewBox="0 0 128 128">
  <rect width="128" height="128" rx="24" fill="#1e293b"/>
  <text x="64" y="88" font-size="72" font-weight="bold" fill="#60a5fa">C</text>
</svg>
```

### 1.3 当前配色

主题色为蓝色系：`#b4c7ff`, `#364bab`, `#172554`, `#0f1729`。需要改为橙色系。

### 1.4 图片资源

`src/assets/` 为空。`public/` 中无 OG 图片、无 PNG favicon、无 apple-touch-icon。

## 2. Claude Logo 参考

### 2.1 Claude 品牌色

- 主色（赤陶橙）：`#d97757` / `#da7756`（多处来源略有差异）
- 辅色：`#000000`（黑色）
- 网站背景：`#eeece2`

### 2.2 Claude Symbol 结构

Claude 的 symbol 是一个多方向放射的星芒图案，由一个复杂的单 path 构成。视觉上呈现为中心向外辐射的多条"光线"，每条光线末端呈尖锐状，整体类似一个不规则的星芒/火花。填充色为 `#d97757`。

SVG 源码中的 symbol 是 logo SVG 的第一个 `<path>`（class cls-2），约 2KB 的 path data。

## 3. 各平台 Link Preview 协议

### 3.1 Open Graph（核心，覆盖最广）

Facebook、LinkedIn、Instagram、Pinterest、WhatsApp、Telegram、Discord、Slack、iMessage、Reddit 全部支持 OG tags。

必需 tags：
- `og:title` - 标题
- `og:description` - 描述
- `og:image` - 图片 URL（必须是绝对路径，HTTPS）
- `og:url` - canonical URL
- `og:type` - 内容类型
- `og:site_name` - 站点名
- `og:locale` - 语言

图片尺寸：**1200 x 630 px**（1.91:1 宽高比），这是跨平台最佳尺寸。

### 3.2 Twitter Cards（X）

Twitter 优先使用自己的 tags，找不到时 fallback 到 OG。

- `twitter:card` - `summary_large_image`
- `twitter:site` - `@StringKe`
- `twitter:creator` - `@StringKe`
- `twitter:title` - 标题（fallback og:title）
- `twitter:description` - 描述（fallback og:description）
- `twitter:image` - 图片（fallback og:image）

### 3.3 Discord

优先 Twitter Card tags > OG tags。对 `og:image` 有 thumbnail 显示偏好，建议图片不要太大。支持 `theme-color` meta tag 用于 embed 侧边色条。

### 3.4 Telegram

使用 OG tags，部分场景有自己的处理逻辑。无额外专属 tags。

### 3.5 Slack

使用 OG tags 做 unfurling。无专属 tags。

### 3.6 Google Search

不使用 OG tags，需要 JSON-LD 结构化数据。

推荐 schema：`SoftwareApplication` + `WebSite`（含 SearchAction）。

### 3.7 iMessage

使用 OG tags（`og:title`, `og:image`, `og:description`）。图片必须是 HTTPS 直链，不能有重定向。fallback 用 `<title>` 标签。

### 3.8 各平台汇总

| 平台 | 协议 | 图片尺寸 | 特殊要求 |
|------|------|---------|---------|
| Facebook/Meta | OG | 1200x630 | 最小 200x200 |
| Twitter/X | Twitter Cards + OG fallback | 1200x630 | summary_large_image |
| LinkedIn | OG | 1200x630 | 最小 1200x627 |
| Instagram | OG | 1200x630 | 仅 link in bio |
| Discord | Twitter Cards > OG | 适中尺寸 | theme-color 色条 |
| Telegram | OG | 1200x630 | |
| Slack | OG | 1200x630 | |
| WhatsApp | OG | 1200x630 | |
| Pinterest | OG | 1200x630 | |
| Reddit | OG | 1200x630 | |
| iMessage | OG | 1200x630 | HTTPS 直链 |
| Google | JSON-LD | 无 | SoftwareApplication |

## 4. 可用 AI 服务评估

从 `~/.zsh_secrets` 中的 API keys：

| 服务 | Key | SVG 生成能力 |
|------|-----|-------------|
| OpenAI (DALL-E) | `PRICE_OPENAI_API_KEY` | 光栅图（PNG），不生成 SVG |
| Claude | `PRICE_CLAUDE_API_KEY` | 可以直接输出 SVG 代码 |
| 其他 LLM | 各种 | 文本 API，不适合 |

**结论：Logo SVG 直接手写是最可靠的方式。** Claude 的星芒图案路径已获取，可以在此基础上设计 Claudex logo（星芒 + x 元素）。OG 图片用 SVG 转 PNG 或直接用 SVG 渲染。

## 5. Starlight Head 扩展机制

### 5.1 `head` 配置项（推荐）

全局 tags 通过 `starlight({ head: [...] })` 注入，与 headDefaults 合并。

优先级：`页面 frontmatter head > config.head > headDefaults`

### 5.2 组件覆盖（备选）

```js
starlight({ components: { Head: './src/components/CustomHead.astro' } })
```

完全替换 Head 组件，风险较高，仅在 `head` 配置不够用时考虑。

### 5.3 评估

对于静态 OG image（所有页面共用一张），`head` 配置项完全够用。无需覆盖 Head 组件。

## 6. 相关文件清单

| 文件 | 需要修改 | 说明 |
|------|---------|------|
| `website/astro.config.mjs` | 是 | 修复 head tags，添加 social |
| `website/public/favicon.svg` | 是 | 替换为新 logo |
| `website/src/styles/global.css` | 是 | 蓝色系改为橙色系 |
| `website/public/og.png` | 新建 | OG 社交分享图 |
| `website/public/apple-touch-icon.png` | 新建 | iOS 图标 |
| `website/public/favicon-32x32.png` | 新建 | 标准 favicon |
| `website/public/favicon-16x16.png` | 新建 | 小尺寸 favicon |
