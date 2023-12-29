import { defineConfig } from 'astro/config';
import mdx from '@astrojs/mdx';
import sitemap from '@astrojs/sitemap';
import react from '@astrojs/react';
import vercel from "@astrojs/vercel/static";
import remarkMath from 'remark-math';
import rehypeKatex from 'rehype-katex';

// https://astro.build/config
export default defineConfig({
  site: 'https://noghartt.dev',
  integrations: [
    mdx(),
    sitemap(),
    react(),
  ],
  output: "static",
  adapter: vercel({
    analytics: true,
  }),
  markdown: {
    remarkPlugins: [
      remarkMath
    ],
    rehypePlugins: [
      rehypeKatex
    ],
  },
  vite: {
    optimizeDeps: {
      exclude: ["@resvg/resvg-js"],
    },
  },
});
