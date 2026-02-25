import { defineConfig } from 'astro/config';
import starlight from '@astrojs/starlight';
import tailwindcss from '@tailwindcss/vite';
import sitemap from '@astrojs/sitemap';

export default defineConfig({
  site: 'https://stringke.github.io',
  base: '/claudex',
  integrations: [
    starlight({
      title: {
        en: 'Claudex',
        'zh-CN': 'Claudex',
      },
      description: 'Multi-instance Claude Code manager with intelligent translation proxy',
      defaultLocale: 'en',
      locales: {
        en: { label: 'English' },
        'zh-cn': { label: '简体中文', lang: 'zh-CN' },
      },
      social: [
        { icon: 'github', label: 'GitHub', href: 'https://github.com/StringKe/claudex' },
      ],
      customCss: ['./src/styles/global.css'],
      sidebar: [
        {
          label: 'Getting Started',
          translations: { 'zh-CN': '快速入门' },
          items: [
            'installation',
            'configuration',
          ],
        },
        {
          label: 'Guides',
          translations: { 'zh-CN': '使用指南' },
          items: [
            'guides/provider-setup',
          ],
        },
        {
          label: 'Features',
          translations: { 'zh-CN': '功能特性' },
          items: [
            'features/translation-proxy',
            'features/circuit-breaker',
            'features/smart-routing',
            'features/context-engine',
            'features/tui-dashboard',
            'features/self-update',
          ],
        },
        {
          label: 'Reference',
          translations: { 'zh-CN': '参考' },
          items: [
            'reference/cli',
            'reference/config',
          ],
        },
      ],
    }),
    sitemap(),
  ],
  vite: {
    plugins: [tailwindcss()],
  },
});
