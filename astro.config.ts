import { defineConfig } from 'astro/config';
import sitemap from '@astrojs/sitemap';
import react from '@astrojs/react';
import remarkMath from 'remark-math';
import rehypeKatex from 'rehype-katex';
import rehypePrettyCode from 'rehype-pretty-code';
import fs from 'fs/promises';
import graymatter from 'gray-matter';
import path from 'path';

import { rehypePluginLinkHeading } from './plugins/rehypePluginLinkHeading';
import { rehypePluginTableWrapper } from './plugins/rehypePluginTableWrapper';
import { remarkPluginReadingTime } from './plugins/remarkPluginReadingTime';

const BLOG_DIR = './src/content/blog';

const getBlogRoutesRedirect = async () => {
  const blogRoutesOldSlug = await fs.readdir(BLOG_DIR);
  const blogRoutes = blogRoutesOldSlug
    .map(post => {
      const frontmatter = graymatter.read(path.join(BLOG_DIR, post));
      return {
        ...frontmatter,
        slug: frontmatter.data.slug,
      }
    })
    .map(({ slug }) => [
      `/${slug}`,
      `/blog/${slug}`,
    ]);

  return Object.fromEntries(blogRoutes);
}

// https://astro.build/config
export default defineConfig({
  site: 'https://noghartt.dev',
  integrations: [
    sitemap(),
    react(),
  ],
  output: "static",
  markdown: {
    syntaxHighlight: false,
    remarkPlugins: [
      remarkMath,
      remarkPluginReadingTime,
    ],
    rehypePlugins: [
      rehypeKatex,
      [
        rehypePrettyCode,
        {
          theme: 'github-light',
          keepBackground: false,
        },
      ],
      rehypePluginLinkHeading,
      rehypePluginTableWrapper,
    ],
  },
  vite: {
    optimizeDeps: {
      exclude: ["@resvg/resvg-js"],
    },
  },
  redirects: {
    ...await getBlogRoutesRedirect(),
  }
});
