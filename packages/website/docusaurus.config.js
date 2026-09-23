// @ts-check
// Note: type annotations allow type checking and IDEs autocompletion

const { themes } = require('prism-react-renderer');

/** @type {import('@docusaurus/types').Config} */
const config = {
  title: 'Niva',
  tagline: '轻松构建超轻量级跨平台应用，Niva 让开发变得简单！',
  favicon: 'img/icon.png',

  // Set the production url of your site here
  url: 'https://bramblex.github.io',
  // Set the /<baseUrl>/ pathname under which your site is served
  // For GitHub pages deployment, it is often '/<projectName>/'
  baseUrl: '/niva/',

  // GitHub pages deployment config.
  // If you aren't using GitHub pages, you don't need these.
  organizationName: 'bramblex', // Usually your GitHub org/user name.
  projectName: 'niva', // Usually your repo name.
  deploymentBranch: 'gh-pages', // Branch that GitHub pages deploys from.
  trailingSlash: false,

  onBrokenLinks: 'throw',
  markdown: {
    hooks: {
      onBrokenMarkdownLinks: 'warn',
    },
  },

  i18n: {
    defaultLocale: 'zh-Hans',
    locales: ['zh-Hans'],
  },

  presets: [
    [
      'classic',
      /** @type {import('@docusaurus/preset-classic').Options} */
      ({
        docs: {
          sidebarPath: require.resolve('./sidebars.js'),
          // Please change this to your repo.
          // Remove this to remove the "edit this page" links.
          // editUrl:
          //   'https://github.com/facebook/docusaurus/tree/main/packages/create-docusaurus/templates/shared/',
        },
        theme: {
          customCss: require.resolve('./src/css/custom.css'),
        },
      }),
    ],
  ],

  themeConfig:
    /** @type {import('@docusaurus/preset-classic').ThemeConfig} */
    ({
      colorMode: {
        defaultMode: 'light',
        respectPrefersColorScheme: false,
      },
      navbar: {
        title: 'Niva',
        logo: {
          alt: 'Niva Logo',
          src: 'img/logo.png',
        },
        items: [
          {
            to: '/docs/tutorial/new-project',
            position: 'left',
            label: '快速上手',
          },
          {
            to: '/docs/options/project',
            position: 'left',
            label: '选项文档',
          },
          {
            to: '/docs/api/niva',
            position: 'left',
            label: 'API 文档',
          },
          // {
          //   type: 'localeDropdown',
          //   position: 'right',
          // },
          {
            href: 'https://github.com/bramblex/niva/releases',
            label: '下载',
            position: 'right',
          },

          {
            href: 'https://github.com/bramblex/niva',
            label: 'GitHub',
            position: 'right',
          },
        ],
      },
      footer: {
        style: 'light',
        links: [
          {
            title: '文档',
            items: [
              {
                label: '快速上手',
                to: '/docs/tutorial/new-project',
              },
              {
                label: '选项文档',
                to: '/docs/options/project',
              },
              {
                label: 'API 文档',
                to: '/docs/api/niva',
              },
            ],
          },
          {
            title: '社区',
            items: [
              {
                label: 'Issues',
                href: 'https://github.com/bramblex/niva/issues',
              },
            ],
          },
          {
            title: '更多',
            items: [
              {
                label: 'GitHub',
                href: 'https://github.com/bramblex/niva',
              },
              {
                label: '下载',
                href: 'https://github.com/bramblex/niva/releases',
              },
            ],
          },
        ],
        copyright: `Copyright © ${new Date().getFullYear()} Niva, Inc. Built with Docusaurus.`,
      },

      prism: {
        theme: themes.github,
        darkTheme: themes.dracula,
      },

    }),
};

module.exports = config;
