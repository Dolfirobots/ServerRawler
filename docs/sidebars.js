/**
 * Creating a sidebar enables you to:
 - create an ordered group of docs
 - render a sidebar for each doc of that group
 - provide next/previous navigation

 The sidebars can be generated from the filesystem, or explicitly defined here.

 Create as many sidebars as you want.
 */

// @ts-check

/** @type {import('@docusaurus/plugin-content-docs').SidebarsConfig} */
const sidebars = {
  // By default, Docusaurus generates a sidebar from the docs folder structure
  tutorialSidebar: [
    'intro',
    {
      type: 'category',
      label: 'Getting Started',
      items: ['getting-started/installation', 'getting-started/configuration', 'getting-started/database-setup'],
    },
    {
      type: 'category',
      label: 'Usage',
      link: {
        type: 'doc',
        id: 'usage',
      },
      items: [
        {
          type: 'category',
          label: 'Arguments',
          items: [
            'usage/arguments/log',
            'usage/arguments/no-database',
            'usage/arguments/max-network-tasks',
            'usage/arguments/config',
            'usage/arguments/ping',
            'usage/arguments/query',
            'usage/arguments/join',
            'usage/arguments/convert-image',
            'usage/arguments/generate-ips',
            'usage/arguments/cidr',
            'usage/arguments/crawl',
            'usage/arguments/scan',
          ],
        },
        'usage/examples',
      ],
    },
    'features',
    'api',
    'contributing',
  ],

  // But you can create a sidebar manually
  /*
  tutorialSidebar: [
    'intro',
    'hello',
    {
      type: 'category',
      label: 'Tutorial',
      items: ['tutorial-basics/create-a-document'],
    },
  ],
   */
};

export default sidebars;