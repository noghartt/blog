import { defineConfig } from 'astro/config';
import mdx from '@astrojs/mdx';
import sitemap from '@astrojs/sitemap';
import remarkMath from 'remark-math';
import rehypeKatex from 'rehype-katex';

import vercel from "@astrojs/vercel/static";

// https://astro.build/config
export default defineConfig({
  site: 'https://noghartt.dev',
  integrations: [mdx(), sitemap()],
  output: "static",
  adapter: vercel(),
  markdown: {
    remarkPlugins: [
      remarkMath
    ],
    rehypePlugins: [
      rehypeKatex
    ],
  }
});
