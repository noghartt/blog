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
import mdx from "@astrojs/mdx";
const BLOG_DIR = './src/content/blog';
const getBlogRoutesRedirect = async () => {
  const blogRoutesOldSlug = await fs.readdir(BLOG_DIR);
  const blogRoutes = blogRoutesOldSlug.map(post => {
    const frontmatter = graymatter.read(path.join(BLOG_DIR, post));
    return {
      ...frontmatter,
      slug: frontmatter.data.slug
    };
  }).map(({
    slug
  }) => [`/${slug}`, `/blog/${slug}`]);
  return Object.fromEntries(blogRoutes);
};
const disableSitemap = ['/blog/drafts'];


// https://astro.build/config
export default defineConfig({
  site: 'https://noghartt.dev',
  integrations: [
    sitemap({
      filter: (page) => {
        try {
          const url = new URL(page);
          const shouldAdd = disableSitemap.every(path => !url.pathname.startsWith(path));
          return shouldAdd;
        } catch (err) {
          return false;
        }
      }
    }),
    react(),
    mdx(),
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
