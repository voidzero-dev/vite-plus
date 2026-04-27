import { resolve } from 'node:path';

import type { VoidZeroThemeConfig } from '@voidzero-dev/vitepress-theme';
import { extendConfig } from '@voidzero-dev/vitepress-theme/config';
import { defineConfig, type HeadConfig } from 'vitepress';
import { withMermaid } from 'vitepress-plugin-mermaid';
import { groupIconMdPlugin, groupIconVitePlugin } from 'vitepress-plugin-group-icons'

const taskRunnerGuideItems = [
  {
    text: 'Run',
    link: '/guide/run',
  },
  {
    text: 'Task Caching',
    link: '/guide/cache',
  },
  {
    text: 'Running Binaries',
    link: '/guide/vpx',
  },
];

const guideSidebar = [
  {
    text: 'Introduction',
    items: [
      { text: 'Getting Started', link: '/guide/' },
      { text: 'Creating a Project', link: '/guide/create' },
      { text: 'Migrate to Vite+', link: '/guide/migrate' },
      { text: 'Installing Dependencies', link: '/guide/install' },
      { text: 'Environment', link: '/guide/env' },
      { text: 'Why Vite+', link: '/guide/why' },
    ],
  },
  {
    text: 'Develop',
    items: [
      { text: 'Dev', link: '/guide/dev' },
      {
        text: 'Check',
        link: '/guide/check',
        items: [
          { text: 'Lint', link: '/guide/lint' },
          { text: 'Format', link: '/guide/fmt' },
        ],
      },
      { text: 'Test', link: '/guide/test' },
    ],
  },
  {
    text: 'Execute',
    items: taskRunnerGuideItems,
  },
  {
    text: 'Build',
    items: [
      { text: 'Build', link: '/guide/build' },
      { text: 'Pack', link: '/guide/pack' },
    ],
  },
  {
    text: 'Maintain',
    items: [
      { text: 'Upgrading Vite+', link: '/guide/upgrade' },
      { text: 'Removing Vite+', link: '/guide/implode' },
    ],
  },
  {
    text: 'Workflow',
    items: [
      { text: 'IDE Integration', link: '/guide/ide-integration' },
      { text: 'CI', link: '/guide/ci' },
      { text: 'Commit Hooks', link: '/guide/commit-hooks' },
      { text: 'Troubleshooting', link: '/guide/troubleshooting' },
    ],
  },
];

export default extendConfig(
  withMermaid(
    defineConfig({
      title: 'Vite+',
      titleTemplate: ':title | The Unified Toolchain for the Web',
      description: 'The Unified Toolchain for the Web',
      cleanUrls: true,
      head: [
        ['link', { rel: 'icon', type: 'image/svg+xml', href: '/favicon.svg' }],
        [
          'link',
          {
            rel: 'preconnect',
            href: 'https://fonts.gstatic.com',
            crossorigin: 'true',
          },
        ],
        ['meta', { name: 'theme-color', content: '#7474FB' }],
        ['meta', { property: 'og:type', content: 'website' }],
        ['meta', { property: 'og:site_name', content: 'Vite+' }],
        ['meta', { name: 'twitter:card', content: 'summary_large_image' }],
        ['meta', { name: 'twitter:site', content: '@voidzerodev' }],
        [
          'script',
          {
            src: 'https://cdn.usefathom.com/script.js',
            'data-site': 'JFDLUWBH',
            'data-spa': 'auto',
            defer: '',
          },
        ],
      ],
      vite: {
        optimizeDeps: {
          include: ['mermaid > @braintree/sanitize-url'],
        },
        resolve: {
          tsconfigPaths: true,
          alias: [
            { find: '@local-assets', replacement: resolve(__dirname, 'theme/assets') },
            { find: '@layouts', replacement: resolve(__dirname, 'theme/layouts') },
            // dayjs ships CJS by default; redirect to its ESM build so
            // mermaid (imported via vitepress-plugin-mermaid) works in dev
            { find: /^dayjs$/, replacement: 'dayjs/esm' },
          ],
        },
        plugins: [
          groupIconVitePlugin({
            customIcon: {
              tsdown: 'https://tsdown.dev/tsdown.svg'
            }
          })
        ],
      },
      themeConfig: {
        variant: 'viteplus' as VoidZeroThemeConfig['variant'],
        nav: [
          {
            text: 'Guide',
            link: '/guide/',
            activeMatch: '^/guide/',
          },
          {
            text: 'Config',
            link: '/config/',
            activeMatch: '^/config/',
          },
          {
            text: 'Resources',
            items: [
              { text: 'Team', link: '/team' },
              { text: 'GitHub', link: 'https://github.com/voidzero-dev/vite-plus' },
              { text: 'Releases', link: 'https://github.com/voidzero-dev/vite-plus/releases' },
              {
                text: 'Announcement',
                link: 'https://voidzero.dev/posts/announcing-vite-plus-alpha',
              },
              {
                text: 'Contributing',
                link: 'https://github.com/voidzero-dev/vite-plus/blob/main/CONTRIBUTING.md',
              },
            ],
          },
        ],
        sidebar: {
          '/guide/': guideSidebar,
          '/config/': [
            {
              text: 'Configuration',
              items: [
                { text: 'Configuring Vite+', link: '/config/' },
                { text: 'Create', link: '/config/create' },
                { text: 'Run', link: '/config/run' },
                { text: 'Format', link: '/config/fmt' },
                { text: 'Lint', link: '/config/lint' },
                { text: 'Test', link: '/config/test' },
                { text: 'Build', link: '/config/build' },
                { text: 'Pack', link: '/config/pack' },
                { text: 'Staged', link: '/config/staged' },
              ],
            },
          ],
        },
        socialLinks: [
          { icon: 'github', link: 'https://github.com/voidzero-dev/vite-plus' },
          { icon: 'x', link: 'https://x.com/voidzerodev' },
          { icon: 'discord', link: 'https://discord.gg/cC6TEVFKSx' },
          { icon: 'bluesky', link: 'https://bsky.app/profile/voidzero.dev' },
        ],
        outline: {
          level: [2, 3],
        },
        search: {
          provider: 'local',
        },

        footer: {
          copyright: `© ${new Date().getFullYear()} VoidZero Inc. and Vite+ contributors.`,
          nav: [
             {
              title: "Company",
              items: [
                { text: "VoidZero", link: "https://voidzero.dev" },
                { text: "Vite", link: "https://vite.dev" },
                { text: "Vitest", link: "https://vitest.dev" },
                { text: "Rolldown", link: "https://rolldown.rs" },
                { text: "Oxc", link: "https://oxc.rs" },
              ],
            },
          ],
            social: [
              { icon: "github", link: "https://github.com/voidzero-dev/vite-plus" },
              { icon: "x", link: "https://x.com/voidzerodev" },
              { icon: "discord", link: "https://discord.gg/cC6TEVFKSx" },
              { icon: "bluesky", link: "https://bsky.app/profile/voidzero.dev" },
            ],
        }
      },
      transformHead({ page, pageData }) {
        const url = 'https://viteplus.dev/' + page.replace(/\.md$/, '').replace(/index$/, '');

        const canonicalUrlEntry: HeadConfig = [
          'link',
          {
            rel: 'canonical',
            href: url,
          },
        ];

        const ogInfo: HeadConfig[] = [
          ['meta', { property: 'og:title', content: pageData.frontmatter.title ?? 'Vite+' }],
          [
            'meta',
            {
              property: 'og:image',
              content: `https://viteplus.dev/${pageData.frontmatter.cover ?? 'og.jpg'}`,
            },
          ],
          ['meta', { property: 'og:url', content: url }],
          [
            'meta',
            {
              property: 'og:description',
              content: pageData.frontmatter.description ?? 'The Unified Toolchain for the Web',
            },
          ],
        ];

        return [...ogInfo, canonicalUrlEntry];
      },
      markdown: {
        config(md) {
          md.use(groupIconMdPlugin)
        },
      },
    }),
  ),
);
