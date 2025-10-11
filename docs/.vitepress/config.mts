import { defineConfig } from 'vitepress';

// https://vitepress.dev/reference/site-config
export default defineConfig({
  title: 'Vite+',
  description: 'The Unified Toolchain for the Web',
  themeConfig: {
    // https://vitepress.dev/reference/default-theme-config
    nav: [
      { text: 'Home', link: '/' },
      { text: 'Get Started', link: '/guide/' },
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
      '/guide/': [
        {
          text: 'Introduction',
          items: [
            {
              text: 'Getting Started',
              link: '/guide/',
            },
            {
              text: 'Migration',
              link: '/guide/migration',
            },
          ],
        },
        {
          text: 'Guide',
          items: [
            {
              text: 'Features',
              link: '/guide/features',
            },
            {
              text: 'Monorepo',
              link: '/guide/monorepo',
            },
            {
              text: 'Testing',
              link: '/guide/testing',
            },
            {
              text: 'Linting',
              link: '/guide/linting',
            },
            {
              text: 'Formatting',
              link: '/guide/formatting',
            },
            {
              text: 'Caching',
              link: '/guide/caching',
            },
            {
              text: 'CLI',
              link: '/guide/cli',
            },
          ],
        },
      ],
      '/config/': [
        {
          text: 'Config',
          items: [
            {
              text: 'Configuring Vite+',
              link: '/config/',
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
  },
});
