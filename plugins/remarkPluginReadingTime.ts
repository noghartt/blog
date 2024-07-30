
import getReadingTime from 'reading-time';
import { toString } from 'mdast-util-to-string';

export function remarkPluginReadingTime() {
  return function (tree, { data }) {
    const textOnPage = toString(tree);
    const readingTime = getReadingTime(textOnPage);
    data.astro.frontmatter.readingTime = readingTime;
  };
}