import { resolve } from 'node:path';

import type { VoidZeroThemeConfig } from '@voidzero-dev/vitepress-theme';
import { extendConfig } from '@voidzero-dev/vitepress-theme/config';
import { defineConfig, type HeadConfig } from 'vitepress';

// https://vitepress.dev/reference/site-config
export default extendConfig(
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
      // ["script", {}, "document.documentElement.setAttribute('data-theme', 'light')"],
    ],
    vite: {
      resolve: {
        tsconfigPaths: true,
        alias: {
          '@local-assets': resolve(__dirname, 'theme/assets'),
          '@layouts': resolve(__dirname, 'theme/layouts'),
        },
      },
    },
    themeConfig: {
      variant: 'viteplus' as VoidZeroThemeConfig['variant'],
      nav: [
        {
          text: 'Guide',
          activeMatch: '/vite/guide/',
          items: [
            { text: 'Vite Core', link: '/vite/guide/' },
            { text: 'Test', link: '/vite/guide/test/getting-started' },
            { text: 'Lint', link: '/vite/guide/lint/getting-started' },
            { text: 'Format', link: '/vite/guide/format/getting-started' },
            { text: 'Task Runner', link: '/vite/guide/task/getting-started' },
            { text: 'Library Bundler', link: '/lib/guide/getting-started' },
            { text: 'DevTools', link: '/guide/devtools/getting-started' },
            { text: 'Package Manager', link: '/guide/package-manager/getting-started' },
          ],
        },
        {
          text: 'Config',
          activeMatch: '/config/',
          items: [
            { text: 'Vite Core', link: '/config/' },
            { text: 'Test', link: '/config/test' },
            { text: 'Lint', link: '/config/lint' },
            { text: 'Format', link: '/config/format' },
            { text: 'Task Runner', link: '/config/task' },
            { text: 'Package Manager', link: '/config/package-manager' },
          ],
        },
        {
          text: 'APIs',
          activeMatch: '/apis/',
          items: [
            { text: 'Vite API', link: '/apis/' },
            { text: 'Environment API', link: '/apis/environment' },
            { text: 'Test API', link: '/apis/test' },
            { text: 'Lint API', link: '/apis/lint' },
            { text: 'Format API', link: '/apis/format' },
            { text: 'Task Runner API', link: '/apis/task-runner' },
          ],
        },
        {
          text: 'Plugins',
          activeMatch: '/plugins/',
          items: [
            { text: 'Vite Plugins', link: '/plugins/' },
            { text: 'Test Plugins', link: '/plugins/test' },
            { text: 'Lint Plugins', link: '/plugins/lint' },
            { text: 'Format Plugins', link: '/plugins/format' },
            { text: 'Task Runner Plugins', link: '/plugins/task-runner' },
          ],
        },
        // {
        //   text: 'Test',
        //   activeMatch: '/plugins/',
        //   items: [
        //     { text: 'Guide & API', link: '/test/guide/' },
        //     { text: 'Config', link: '/test/config/' },
        //     { text: 'Browser Mode', link: '/test/browser-mode/' },
        //   ],
        // },
        // {
        //   text: 'Format',
        //   activeMatch: '/plugins/',
        //   items: [
        //     { text: 'Guide & API', link: '/test/guide/' },
        //     { text: 'Config', link: '/test/config/' },
        //     { text: 'Browser Mode', link: '/test/browser-mode/' },
        //   ],
        // },
        {
          text: 'Resources',
          items: [
            { text: 'Team', link: 'https://voidzero.dev/team' },
            { text: 'Blog', link: 'https://voidzero.dev/blog' },
            { text: 'Releases', link: 'https://github.com/voidzero-dev/vite-plus/releases' },
            {
              items: [
                {
                  text: 'Awesome Vite+',
                  link: 'https://github.com/voidzero-dev/awesome-vite-plus',
                },
                {
                  text: 'ViteConf',
                  link: 'https://viteconf.org',
                },
                {
                  text: 'DEV Community',
                  link: 'https://dev.to/t/vite',
                },
                {
                  text: 'Changelog',
                  link: 'https://github.com/voidzero-dev/vite-plus/releases',
                },
                {
                  text: 'Contributing',
                  link: 'https://github.com/voidzero-dev/vite-plus/blob/main/CONTRIBUTING.md',
                },
              ],
            },
          ],
        },
      ],

      sidebar: {
        '/vite/guide/': {
          base: '/vite/guide/',
          items: [
            {
              text: 'Introduction',
              items: [
                {
                  text: 'Getting Started',
                  link: 'index',
                },
                {
                  text: 'Monorepo',
                  link: 'monorepo',
                },
                {
                  text: 'Philosophy',
                  link: 'philosophy',
                },
                {
                  text: 'Why Vite+',
                  link: 'why',
                },
                {
                  text: 'Migration',
                  link: 'migration',
                },
              ],
            },
            {
              text: 'Vite Core',
              items: [
                {
                  text: 'Features',
                  link: 'features',
                },
                {
                  text: 'CLI',
                  link: 'cli',
                },
                {
                  text: 'Using Plugins',
                  link: 'using-plugins',
                },
                {
                  text: 'Dependency Pre-Bundling',
                  link: 'dependency-pre-bundling',
                },
                {
                  text: 'Static Asset Handling',
                  link: 'static-asset-handling',
                },
                {
                  text: 'Building for Production',
                  link: 'building-for-production',
                },
                {
                  text: 'Deploying a Static Site',
                  link: 'deploying-a-static-site',
                },
                {
                  text: 'Env Variables and Modes',
                  link: 'env-variables-and-modes',
                },
                {
                  text: 'Server-Side Rendering',
                  link: 'server-side-rendering',
                },
                {
                  text: 'Backend Integration',
                  link: 'backend-integration',
                },
                {
                  text: 'Troubleshooting',
                  link: 'troubleshooting',
                },
                {
                  text: 'Performance',
                  link: 'performance',
                },
                {
                  text: 'Migration from vite@7+',
                  link: 'migration',
                },
              ],
            },
            {
              text: 'Test',
              items: [
                {
                  text: 'Getting Started',
                  link: '/guide/test/getting-started',
                },
                {
                  text: 'Features',
                  link: '/guide/test/features',
                },
                {
                  text: 'CLI',
                  link: '/guide/test/cli',
                },
                {
                  text: 'Test Filtering',
                  link: '/guide/test/test-filtering',
                },
                {
                  text: '...',
                  link: '/guide/test/dependency-pre-bundling',
                },
                {
                  text: 'Migration from vitest',
                  link: '/guide/test/migration-from-vitest',
                },
              ],
            },
            {
              text: 'Lint',
              items: [
                {
                  text: 'Getting Started',
                  link: '/guide/lint/getting-started',
                },
                {
                  text: 'Features',
                  link: '/guide/lint/features',
                },
                {
                  text: 'CLI',
                  link: '/guide/lint/cli',
                },
                {
                  text: '...',
                  link: '/guide/test/dependency-pre-bundling',
                },
                {
                  text: 'Migration from oxlint',
                  link: '/guide/test/migration-from-oxlint',
                },
                {
                  text: 'Migration from ESLint',
                  link: '/guide/test/migration-from-eslint',
                },
              ],
            },
            {
              text: 'Format',
              items: [
                {
                  text: 'Getting Started',
                  link: '/guide/format/getting-started',
                },
                {
                  text: 'Features',
                  link: '/guide/format/features',
                },
                {
                  text: 'CLI',
                  link: '/guide/format/cli',
                },
                {
                  text: 'Migration from oxfmt',
                  link: '/guide/format/migration-from-oxfmt',
                },
                {
                  text: 'Migration from Prettier',
                  link: '/guide/format/migration-from-prettier',
                },
              ],
            },
            {
              text: 'Task Runner',
              items: [
                {
                  text: 'Getting Started',
                  link: 'task/getting-started',
                },
                {
                  text: 'Features',
                  link: '/guide/task/features',
                },
                {
                  text: 'CLI',
                  link: '/guide/task/cli',
                },
                {
                  text: 'Migration from Turborepo',
                  link: '/guide/format/migration-from-turborepo',
                },
                {
                  text: 'Migration from Nx',
                  link: '/guide/format/migration-from-nx',
                },
                {
                  text: 'Migration from Lerna',
                  link: '/guide/format/migration-from-lerna',
                },
                {
                  text: 'Migration from pnpm',
                  link: '/guide/format/migration-from-pnpm',
                },
                {
                  text: 'Migration from yarn',
                  link: '/guide/format/migration-from-yarn',
                },
                {
                  text: 'Migration from npm',
                  link: '/guide/format/migration-from-npm',
                },
                {
                  text: 'Migration from bun',
                  link: '/guide/format/migration-from-bun',
                },
              ],
            },
            {
              text: 'Library Bundler',
              items: [
                {
                  text: 'Getting Started',
                  link: '/guide/library/getting-started',
                },
                {
                  text: 'Features',
                  link: '/guide/library/features',
                },
                {
                  text: 'CLI',
                  link: '/guide/library/cli',
                },
                {
                  text: 'Migration from tsdown',
                  link: '/guide/library/migration-from-tsdown',
                },
                {
                  text: 'Migration from tsup',
                  link: '/guide/library/migration-from-tsup',
                },
                {
                  text: 'Migration from esbuild',
                  link: '/guide/library/migration-from-esbuild',
                },
              ],
            },
            {
              text: 'DevTools',
              items: [
                {
                  text: 'Getting Started',
                  link: '/guide/devtools/getting-started',
                },
                {
                  text: 'Features',
                  link: '/guide/library/features',
                },
                {
                  text: 'CLI',
                  link: '/guide/devtools/cli',
                },
              ],
            },
            {
              text: 'Package Manager',
              items: [
                {
                  text: 'Getting Started',
                  link: '/guide/package-manager/getting-started',
                },
                {
                  text: 'Features',
                  link: '/guide/package-manager/features',
                },
                {
                  text: 'CLI',
                  link: '/guide/package-manager/cli',
                },
              ],
            },
          ],
        },
        '/lib/guide/': {
          base: '/lib/guide/',
          items: [
            {
              text: 'Library Bundler',
              items: [
                {
                  text: 'Introduction',
                  link: 'introduction',
                },
                {
                  text: 'Getting Started',
                  link: 'getting-started',
                },
                {
                  text: 'Migration from tsdown',
                  link: '/guide/library/migration-from-tsdown',
                },
                {
                  text: 'Migration from tsup',
                  link: '/guide/library/migration-from-tsup',
                },
                {
                  text: 'Migration from esbuild',
                  link: '/guide/library/migration-from-esbuild',
                },
              ],
            },
            {
              text: 'Options',
              items: [
                {
                  text: 'Entry',
                  link: 'entry',
                },
                {
                  text: 'Config File',
                  link: 'config-file',
                },
                {
                  text: 'Declaration Files (dts)',
                  link: 'dts',
                },
              ],
            },
            {
              text: 'Recipes',
              items: [
                {
                  text: 'Vue Support',
                  link: 'vue-support',
                },
                {
                  text: 'React Support',
                  link: 'react-support',
                },
                {
                  text: 'Svelte Support',
                  link: 'svelte-support',
                },
              ],
            },
            {
              text: 'Advanced',
              items: [
                {
                  text: 'Plugins',
                  link: 'plugins',
                },
                {
                  text: 'Hooks',
                  link: 'hooks',
                },
                {
                  text: 'Rolldown Options',
                  link: 'rolldown-options',
                },
                {
                  text: 'Programmatic Usage',
                  link: 'programmatic-usage',
                },
              ],
            },
            {
              text: 'API Reference',
              items: [
                {
                  text: 'Command Line Interface',
                  link: 'command-line-interface',
                },
                {
                  text: 'Config Options',
                  link: 'config-options',
                },
                {
                  text: 'Type Definitions',
                  items: [
                    {
                      text: 'AttwOptions',
                      link: 'attributes-options',
                    },
                    {
                      text: 'BuildContext',
                      link: 'build-context',
                    },
                  ],
                },
              ],
            },
          ],
        },
        '/config/': [
          {
            text: 'Vite Core',
            items: [
              {
                text: 'Configuring Vite+',
                link: '/config/',
              },
              {
                text: 'Shared Options',
                link: '/config/shared-options',
              },
              {
                text: 'Server Options',
                link: '/config/server-options',
              },
              {
                text: 'Build Options',
                link: '/config/build-options',
              },
              {
                text: 'Preview Options',
                link: '/config/preview-options',
              },
              {
                text: 'Dep Optimization Options',
                link: '/config/dep-optimization-options',
              },
              {
                text: 'SSR Options',
                link: '/config/ssr-options',
              },
              {
                text: 'Worker Options',
                link: '/config/worker-options',
              },
            ],
          },
          {
            text: 'Test',
            items: [
              {
                text: 'Configuring Test',
                link: '/config/test',
              },
              {
                text: 'Test Options',
                link: '/config/test-options',
              },
            ],
          },
          {
            text: 'Lint',
            items: [
              {
                text: 'Configuring Lint',
                link: '/config/lint',
              },
              {
                text: 'Lint Options',
                link: '/config/lint-options',
              },
            ],
          },
        ],
        '/apis/': [
          {
            text: 'Vite Core API',
            items: [
              {
                text: 'Plugin API',
                link: '/apis/vite/plugin',
              },
              {
                text: 'HMR API',
                link: '/apis/vite/hmr',
              },
              {
                text: 'JavaScript API',
                link: '/apis/vite/javascript',
              },
            ],
          },
          {
            text: 'Environment API',
            items: [
              {
                text: 'Introduction',
                link: '/apis/environment/introduction',
              },
              {
                text: 'Environment Instances',
                link: '/apis/environment/instances',
              },
              {
                text: 'Plugins',
                link: '/apis/environment/plugins',
              },
              {
                text: 'Frameworks',
                link: '/apis/environment/frameworks',
              },
              {
                text: 'Runtimes',
                link: '/apis/environment/runtimes',
              },
            ],
          },
          {
            text: 'Test API',
            items: [
              {
                text: 'Introduction',
                link: '/apis/test/introduction',
              },
              {
                text: 'Plugin API',
                link: '/apis/test/plugin',
              },
            ],
          },
          {
            text: 'Lint API',
            items: [
              {
                text: 'Introduction',
                link: '/apis/lint/introduction',
              },
              {
                text: 'Plugin API',
                link: '/apis/lint/plugin',
              },
            ],
          },
          {
            text: 'Format API',
            items: [
              {
                text: 'Introduction',
                link: '/apis/format/introduction',
              },
              {
                text: 'Plugin API',
                link: '/apis/format/plugin',
              },
            ],
          },
          {
            text: 'Task Runner API',
            items: [
              {
                text: 'Introduction',
                link: '/apis/task-runner/introduction',
              },
              {
                text: 'Plugin API',
                link: '/apis/task-runner/plugin',
              },
            ],
          },
        ],
        '/changes/': [],
      },

      socialLinks: [
        { icon: 'github', link: 'https://github.com/voidzero-dev/vite-plus' },
        { icon: 'x', link: 'https://x.com/voidzerodev' },
        { icon: 'bluesky', link: 'https://bsky.app/profile/voidzero.dev' },
      ],

      outline: {
        level: [2, 3],
      },

      search: {
        provider: 'local',
      },
    },
    transformHead({ page, pageData, assets }) {
      // Remove .md suffix and replace index with empty string (to cover index.md)
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
  }),
);
