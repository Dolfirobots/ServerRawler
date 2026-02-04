// @ts-check
// `@type` JSDoc annotations allow editor autocompletion and type checking
// (when paired with `@ts-check`).
// There are various equivalent ways to declare your Docusaurus config.
// See: https://docusaurus.io/docs/api/docusaurus-config

import {themes as prismThemes} from 'prism-react-renderer';

// This runs in Node.js - Don't use client-side code here (browser APIs, JSX...)

/** @type {import('@docusaurus/types').Config} */
const config = {
  title: 'ServerRawler',
  tagline: 'A high-performance Minecraft server crawler written in Rust.',
  favicon: 'img/favicon.ico',

  // Future flags, see https://docusaurus.io/docs/api/docusaurus-config#future
  future: {
    v4: true, // Improve compatibility with the upcoming Docusaurus v4
  },

  url: 'https://cyberdolfi.github.io',
  baseUrl: '/ServerRawler/',

  // GitHub pages deployment config.
  organizationName: 'Cyberdolfi',
  projectName: 'ServerRawler',

  onBrokenLinks: 'throw',

  // Even if you don't use internationalization, you can use this field to set
  // useful metadata like html lang. For example, if your site is Chinese, you
  // may want to replace "en" with "zh-Hans".
  i18n: {
    defaultLocale: 'en',
    locales: ['en'],
  },

  presets: [
    [
      'classic',
      /** @type {import('@docusaurus/preset-classic').Options} */
      ({
        docs: {
          sidebarPath: './sidebars.js',
          editUrl:
            'https://github.com/Cyberdolfi/ServerRawler/tree/main/docs/',
        },
        theme: {
          customCss: './src/css/custom.css',
        },
      }),
    ],
  ],

  themeConfig:
    /** @type {import('@docusaurus/preset-classic').ThemeConfig} */
    ({
      //image: '',
      colorMode: {
        defaultMode: 'dark',
        disableSwitch: true,
        respectPrefersColorScheme: false,
      },
      navbar: {
        title: 'ServerRawler',
        logo: {
          alt: 'ServerRawler Logo',
          src: 'img/logo.png',
        },
        items: [
          {
            type: 'docSidebar',
            sidebarId: 'tutorialSidebar',
            position: 'left',
            label: 'Docs',
          },
          {
            href: 'https://github.com/Cyberdolfi/ServerRawler',
            label: 'GitHub',
            position: 'right',
          },
        ],
      },
      footer: {
        style: 'dark',
        links: [
          {
            title: 'Docs',
            items: [
              { label: 'Introduction', to: '/docs/intro' },
              { label: 'Installation', to: '/docs/getting-started/installation' },
              { label: 'Configuration', to: '/docs/getting-started/configuration' },
              { label: 'Database Setup', to: '/docs/getting-started/database-setup' },
              { label: 'Usage', to: '/docs/usage' },
              { label: 'API', to: '/docs/api' },
            ],
          },
          {
            title: 'Community & Social',
            items: [
              { label: 'GitHub', href: 'https://github.com/Cyberdolfi/ServerRawler' },
              { label: 'Discord', href: 'https://discord.gg/4wHFzBjDTY' },
            ],
          },
          {
            title: 'Project',
            items: [
              { label: 'Features', to: '/docs/features' },
              { label: 'Contributing', to: '/docs/contributing' },
              { label: 'License', href: 'https://github.com/Cyberdolfi/ServerRawler/blob/main/LICENSE' },
            ],
          },
        ],
        copyright: `Crafted by Cyberdolfi. Built with Docusaurus. Copyright © ${new Date().getFullYear()} ServerRawler.`,
      },

      prism: {
        theme: prismThemes.github,
        darkTheme: prismThemes.dracula,
        additionalLanguages: ['powershell', 'toml', 'rust', 'bash'],
        magicComments: [
          {
            className: 'code-block-error-line',
            line: '@(error)',
            block: {start: '@(error-start)', end: '@(error-end)'},
          },
        ],
      },
    }),
};

export default config;
