import { defineConfig } from 'vitepress'

export default defineConfig({
  title: 'Crisp',
  description: 'ESLint reports, made crisp. A readable ESLint formatter powered by a Rust core.',
  base: '/eslint-formatter-crisp/',
  lang: 'en-US',
  ignoreDeadLinks: true,
  head: [
    ['link', { rel: 'icon', type: 'image/svg+xml', href: '/eslint-formatter-crisp/favicon.svg' }],
    ['meta', { name: 'theme-color', content: '#3b82f6' }],
    ['meta', { property: 'og:type', content: 'website' }],
    ['meta', { property: 'og:title', content: 'Crisp - ESLint reports, made crisp.' }],
    ['meta', { property: 'og:description', content: 'A readable ESLint formatter powered by a Rust core and delivered via N-API.' }],
    ['meta', { property: 'og:image', content: '/eslint-formatter-crisp/crisp-hero.png' }]
  ],
  themeConfig: {
    logo: '/logo.svg',
    siteTitle: 'Crisp',
    nav: [
      { text: 'Guide', link: '/guide/getting-started' },
      { text: 'API', link: '/api/' },
      { text: 'GitHub', link: 'https://github.com/jubayerhossaindev/eslint-formatter-crisp' }
    ],
    sidebar: [
      {
        text: 'Guide',
        collapsed: false,
        items: [
          { text: 'Getting Started', link: '/guide/getting-started' },
          { text: 'Usage', link: '/guide/usage' },
          { text: 'Configuration', link: '/guide/configuration' },
          { text: 'Troubleshooting', link: '/guide/troubleshooting' }
        ]
      },
      {
        text: 'API Reference',
        collapsed: false,
        items: [
          { text: 'Overview', link: '/api/' },
          { text: 'Formatter', link: '/api/formatter' },
          { text: 'JSON', link: '/api/json' },
          { text: 'SARIF', link: '/api/sarif' },
          { text: 'Types', link: '/api/types' }
        ]
      }
    ],
    socialLinks: [
      { icon: 'github', link: 'https://github.com/jubayerhossaindev/eslint-formatter-crisp' }
    ],
    footer: {
      message: 'Released under the MIT License.',
      copyright: 'Copyright © 2024-present'
    },
    search: {
      provider: 'local'
    },
    outline: {
      level: [2, 3]
    }
  }
})
