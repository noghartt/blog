import { visit } from 'unist-util-visit';
import type { Root } from 'mdast';
import { toString } from 'mdast-util-to-string';

export function remarkPluginSidenotes() {
  return function (tree: Root) {
    const footnoteDefinitions = new Map<string, any>();
    
    // First pass: collect all footnote definitions
    visit(tree, 'footnoteDefinition', (node: any) => {
      footnoteDefinitions.set(node.identifier, node.children);
    });

    // Second pass: replace footnote references with inline sidenote HTML
    visit(tree, 'footnoteReference', (node: any, index, parent: any) => {
      if (typeof index !== 'number' || !parent) return;
      
      const content = footnoteDefinitions.get(node.identifier);
      if (!content) return;
      
      // Convert the footnote content to plain text/HTML
      // We'll render the content as simple text for now
      const contentText = content.map((child: any) => toString(child)).join(' ');
      
      // Create the sidenote structure as raw HTML with IDs for clickability
      const sidenoteId = `sn-${node.identifier}`;
      const checkboxId = `${sidenoteId}-toggle`; // Unique ID for checkbox
      const labelId = `${sidenoteId}-ref`;
      const sidenoteHTML = `<input type="checkbox" id="${checkboxId}" class="margin-toggle"><label for="${checkboxId}" id="${labelId}" class="sidenote-number"></label><span class="sidenote" id="${sidenoteId}">${contentText}</span>`;
      
      // Replace the footnote reference with our HTML node
      parent.children[index] = {
        type: 'html',
        value: sidenoteHTML,
      };
    });
    
    // Third pass: remove footnote definitions (they're now inline)
    const nodesToRemove: any[] = [];
    visit(tree, 'footnoteDefinition', (node: any, index, parent: any) => {
      if (typeof index !== 'number' || !parent) return;
      nodesToRemove.push({ parent, index });
    });
    
    // Remove in reverse order to maintain indices
    nodesToRemove.reverse().forEach(({ parent, index }) => {
      parent.children.splice(index, 1);
    });
  };
}

