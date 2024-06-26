import type { Root } from 'hast';
import { visit } from 'unist-util-visit';
import { modifyChildren } from 'unist-util-modify-children';
import slugify from 'slugify';

const BASE_URL = process.env.NODE_ENV === 'development'
  ? 'http://localhost:4321'
  : 'https://noghartt.dev';

const modifyHeading = (url: string) => modifyChildren((node) => {
  if (node.type !== 'text') {
    return;
  }

  const text = node.value;

  delete node.position;
  node.type = 'element';
  node.tagName = 'a';
  node.properties = {
    class: 'heading-link',
    href: url,
  };
  node.children = [
    {
      type: 'text',
      value: text,
    },
  ]
})

export function rehypePluginLinkHeading() {
  return function(tree: Root, { history }) {
    const contentPath = history[0].split('/content')[1].replace('.md', '');
    const url = BASE_URL + contentPath;
    visit(tree, 'element', (node) => {
      const headingElements = ['h1', 'h2', 'h3', 'h4', 'h5', 'h6'];
      if (headingElements.includes(node.tagName)) {
        const title = node.children.map((child) => child.value).join('');
        const slug = slugify(title, { lower: true, replacement: '-', strict: true });
        const hashedUrl = `${url}#${slug}`;
        modifyHeading(hashedUrl)(node);
      }
    });
  }
}
