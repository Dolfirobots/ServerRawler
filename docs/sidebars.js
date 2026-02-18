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
      items: [
        'getting-started/installation',
        'getting-started/database-setup',
        'getting-started/configuration'
      ],
    },
    {
      type: 'category',
      label: 'Configuration',
      items: [
        'configuration/config',
        'configuration/database',
      ],
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
            'usage/arguments/config',
            'usage/arguments/cidr',
          ],
        },
        {
          type: 'category',
          label: 'Commands',
          items: [
            'usage/commands/crawl',
            'usage/commands/generate',
            'usage/commands/scan',
            'usage/commands/query',
            'usage/commands/ping',
            'usage/commands/join',
            'usage/commands/convert-img',
          ],
        },
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