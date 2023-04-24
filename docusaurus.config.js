// @ts-check
// Note: type annotations allow type checking and IDEs autocompletion

const lightCodeTheme = require('prism-react-renderer/themes/github');
const darkCodeTheme = require('prism-react-renderer/themes/dracula');

/** @type {import('@docusaurus/types').Config} */
const config = {
  title: 'Niva',
  tagline: '轻松构建超轻量级跨平台应用，Niva 让开发变得简单！',
  favicon: 'img/favicon.ico',

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
  onBrokenMarkdownLinks: 'warn',

  // Even if you don't use internalization, you can use this field to set useful
  // metadata like html lang. For example, if your site is Chinese, you may want
  // to replace "en" with "zh-Hans".
  i18n: {
    defaultLocale: 'zh-CN',
    locales: ['zh-CN', 'en'],
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
      // Replace with your project's social card
      image: 'img/docusaurus-social-card.jpg',
      navbar: {
        title: 'Niva',
        logo: {
          alt: 'Niva Logo',
          src: 'img/logo.svg',
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
            label: '配置文档',
          },
          {
            to: '/docs/api/niva',
            position: 'left',
            label: 'API 文档',
          },
          {
            type: 'localeDropdown',
            position: 'right',
          },
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
        style: 'dark',
        links: [
          {
            title: '文档',
            items: [
              {
                label: '快速上手',
                to: '/docs/tutorial/new-project',
              },
              {
                label: '配置文档',
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
        theme: lightCodeTheme,
        darkTheme: darkCodeTheme,
      },

      algolia: {
        // The application ID provided by Algolia
        appId: 'YOUR_APP_ID',

        // Public API key: it is safe to commit it
        apiKey: 'YOUR_SEARCH_API_KEY',

        indexName: 'YOUR_INDEX_NAME',

        // Optional: see doc section below
        contextualSearch: true,

        // Optional: Specify domains where the navigation should occur through window.location instead on history.push. Useful when our Algolia config crawls multiple documentation sites and we want to navigate with window.location.href to them.
        externalUrlRegex: 'external\\.com|domain\\.com',

        // Optional: Replace parts of the item URLs from Algolia. Useful when using the same search index for multiple deployments using a different baseUrl. You can use regexp or string in the `from` param. For example: localhost:3000 vs myCompany.com/docs
        replaceSearchResultPathname: {
          from: '/docs/', // or as RegExp: /\/docs\//
          to: '/',
        },

        // Optional: Algolia search parameters
        searchParameters: {},

        // Optional: path for search page that enabled by default (`false` to disable it)
        searchPagePath: 'search',

        //... other Algolia params
      },
    }),
};

module.exports = config;
