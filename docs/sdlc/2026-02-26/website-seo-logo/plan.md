# Plan: Website SEO + Logo 设计

## 方案概述

为 claudex.space 实现全平台社交分享支持和品牌视觉升级。分三个阶段：Logo 设计、OG 图片生成、SEO meta tags 完善。

## Phase 1: Logo 设计

### 1.1 设计 Claudex SVG Logo

设计理念：以 Claude 的星芒（starburst）图案为基础，融合 "x" 元素表达扩展（extension）概念。

具体方案：
- 保留 Claude 星芒的放射线条风格，但简化为更干净的几何形状
- 将 "x" 融入星芒中心或作为星芒的交叉结构
- 主色使用赤陶橙 `#d97757`（Claude 品牌色）
- 深色背景保持 `#1e293b`（与现有暗色主题协调）

输出文件：
- `website/public/favicon.svg` - 主 SVG logo（128x128 viewBox）
- `website/public/logo.svg` - 无背景版本，用于 header 等场景

### 1.2 生成衍生图标

从 SVG 导出 PNG 格式图标：
- `website/public/favicon-32x32.png`
- `website/public/favicon-16x16.png`
- `website/public/apple-touch-icon.png`（180x180）

工具：使用项目已有的 `sharp` 依赖（package.json 中已引入），写一个一次性 Node 脚本将 SVG 转 PNG。

## Phase 2: OG 社交分享图

### 2.1 设计 OG 图片

尺寸：1200 x 630 px（所有平台通用最佳尺寸）

布局方案：
```
+--------------------------------------------------+
|                                                  |
|     [Logo]  Claudex                              |
|                                                  |
|     Multi-instance Claude Code manager           |
|     with intelligent translation proxy           |
|                                                  |
|     claudex.space                                |
|                                                  |
+--------------------------------------------------+
```

- 深色渐变背景（`#0f1729` -> `#1e293b`）
- 左侧或居中放置 logo
- 标题用白色大字
- 描述用浅灰色
- 底部用橙色 accent 线条

输出：`website/public/og.png`

方式：用 SVG 定义完整布局，通过 sharp 转 PNG。纯代码实现，无需外部设计工具。

## Phase 3: SEO Meta Tags 完善

### 3.1 修复 astro.config.mjs head 配置

删除：
- `og:site_name`（与 Starlight 默认重复）
- `canonical` 硬编码（与 Starlight 默认冲突，导致所有页面 canonical 指向首页）

修改：
- `twitter:site` 从 `@anthropic` 改为 `@StringKe`

添加：
```js
// OG Image（全站统一）
{ tag: 'meta', attrs: { property: 'og:image', content: 'https://claudex.space/og.png' } },
{ tag: 'meta', attrs: { property: 'og:image:width', content: '1200' } },
{ tag: 'meta', attrs: { property: 'og:image:height', content: '630' } },
{ tag: 'meta', attrs: { property: 'og:image:type', content: 'image/png' } },

// Twitter
{ tag: 'meta', attrs: { name: 'twitter:site', content: '@StringKe' } },
{ tag: 'meta', attrs: { name: 'twitter:creator', content: '@StringKe' } },
{ tag: 'meta', attrs: { name: 'twitter:image', content: 'https://claudex.space/og.png' } },

// Discord theme color
{ tag: 'meta', attrs: { name: 'theme-color', content: '#d97757' } },

// Apple
{ tag: 'link', attrs: { rel: 'apple-touch-icon', sizes: '180x180', href: '/apple-touch-icon.png' } },
{ tag: 'link', attrs: { rel: 'icon', type: 'image/png', sizes: '32x32', href: '/favicon-32x32.png' } },
{ tag: 'link', attrs: { rel: 'icon', type: 'image/png', sizes: '16x16', href: '/favicon-16x16.png' } },
```

### 3.2 添加 social 配置

Starlight 的 `social` 数组会自动从 twitter/x.com URL 提取 `@username` 并生成 `twitter:site` meta tag。

```js
social: [
  { icon: 'github', label: 'GitHub', href: 'https://github.com/StringKe/claudex' },
  { icon: 'x.com', label: 'X', href: 'https://x.com/StringKe' },
],
```

这样 Starlight 会自动生成 `twitter:site: @StringKe`，就不需要在 head 中手动加了。

### 3.3 添加 JSON-LD 结构化数据

在 head 中添加 `<script type="application/ld+json">`：

```json
{
  "@context": "https://schema.org",
  "@type": "SoftwareApplication",
  "name": "Claudex",
  "description": "Multi-instance Claude Code manager with intelligent translation proxy",
  "url": "https://claudex.space",
  "applicationCategory": "DeveloperApplication",
  "operatingSystem": "macOS, Linux, Windows",
  "author": {
    "@type": "Person",
    "name": "StringKe",
    "url": "https://x.com/StringKe"
  },
  "offers": {
    "@type": "Offer",
    "price": "0",
    "priceCurrency": "USD"
  },
  "codeRepository": "https://github.com/StringKe/claudex"
}
```

### 3.4 更新主题色

`src/styles/global.css` 中的 accent 色从蓝色改为橙色系：

```css
@theme {
  --color-accent-200: #f5c4a1;  /* 浅橙 */
  --color-accent-600: #d97757;  /* Claude 主色 */
  --color-accent-900: #7c3a1a;  /* 深橙 */
  --color-accent-950: #4a1d0a;  /* 最深 */
}
```

## 改动文件清单

| 文件 | 操作 | 说明 |
|------|------|------|
| `website/public/favicon.svg` | 重写 | 新 Claudex logo |
| `website/public/logo.svg` | 新建 | 无背景 logo |
| `website/public/og.png` | 新建 | 社交分享图 |
| `website/public/apple-touch-icon.png` | 新建 | iOS 图标 |
| `website/public/favicon-32x32.png` | 新建 | PNG favicon |
| `website/public/favicon-16x16.png` | 新建 | PNG favicon |
| `website/astro.config.mjs` | 修改 | SEO tags + social |
| `website/src/styles/global.css` | 修改 | 橙色主题色 |
| `website/scripts/generate-assets.mjs` | 新建 | SVG 转 PNG 脚本（一次性） |

## 考量与权衡

1. **OG 图片策略：静态 vs 动态**
   - 静态（选择）：一张通用图，简单可靠
   - 动态：按页面生成不同图片，复杂度高，对文档站收益低
   - 如后续需要动态 OG，可引入 `@vercel/og` 或 Satori

2. **Logo 设计：直接 SVG vs AI 生成**
   - 直接手写 SVG（选择）：完全可控，矢量清晰，文件极小
   - AI 生成：DALL-E 只出光栅图，质量不可控

3. **图标格式：多 PNG vs ICO**
   - 多 PNG + SVG（选择）：现代浏览器原生支持，覆盖全
   - ICO：仅 IE 需要，可忽略

4. **Starlight Head 覆盖 vs head 配置**
   - head 配置（选择）：简单，与框架集成好
   - Head 组件覆盖：过度，静态 meta tags 不需要

## Todo List

### Phase 1: Logo 设计
- [x] 设计并写入 `website/public/favicon.svg`（星芒 + X，带深色背景）
- [x] 创建 `website/public/logo.svg`（无背景独立版）
- [x] 编写 `website/scripts/generate-assets.mjs` 脚本
- [x] 运行脚本生成 `favicon-32x32.png`、`favicon-16x16.png`、`apple-touch-icon.png`

### Phase 2: OG 社交分享图
- [x] 在 generate-assets.mjs 中生成 `website/public/og.png`（1200x630）

### Phase 3: SEO Meta Tags
- [x] 更新 `website/astro.config.mjs` head 配置（删除错误项、添加 OG/Twitter/Apple/theme-color/JSON-LD）
- [x] 更新 `website/astro.config.mjs` social 配置（添加 X/Twitter）
- [x] 更新 `website/src/styles/global.css` 主题色（蓝 -> 橙）

### Phase 4: 验证
- [x] 运行 website build 确认无报错
