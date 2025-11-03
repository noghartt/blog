import { visit } from 'unist-util-visit';
import type { Root } from 'hast';

export function rehypePluginSidenotes() {
  return function (tree: Root) {
    // Find raw HTML nodes containing sidenotes and add IDs
    visit(tree, 'raw', (node: any) => {
      if (node.value && typeof node.value === 'string' && node.value.includes('margin-toggle')) {
        // Parse the HTML to find the ID
        const match = node.value.match(/id="(sn-[^"]+)"/);
        if (match) {
          const sidenoteId = match[1];
          const refId = `${sidenoteId}-ref`;

          // Add ID to the label and sidenote span
          node.value = node.value
            .replace(
              /(<label for="sn-[^"]+" class="sidenote-number")>/,
              `$1 id="${refId}">`
            )
            .replace(
              /(<span class="sidenote")>/,
              `$1 id="${sidenoteId}">`
            );
        }
      }
    });
  };
}
