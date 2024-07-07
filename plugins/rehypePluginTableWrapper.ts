import type { Root } from 'hast';
import { visit } from 'unist-util-visit';
import { h } from 'hastscript';

function removeFieldFromTree(node: Node, fieldToRemove: string) {
  if (Array.isArray(node)) {
      node.forEach(child => removeFieldFromTree(child, fieldToRemove));
  } else if (node !== null && typeof node === 'object') {
      delete node[fieldToRemove];
      for (const key in node) {
          if (node.hasOwnProperty(key)) {
              removeFieldFromTree(node[key], fieldToRemove);
          }
      }
  }
}

export function rehypePluginTableWrapper() {
  return function(tree: Root) {
    visit(tree, 'element', (node, index, parent) => {
      if (node.tagName !== 'table') {
        return;
      }

      const tableNode = structuredClone(node);
      removeFieldFromTree(tableNode, 'position');

      const wrapper = h('div', { className: 'table-wrapper' }, [tableNode]);
      parent.children.splice(index, 1, wrapper);
    });
  }
}
