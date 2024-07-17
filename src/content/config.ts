import { defineCollection, z } from 'astro:content';
import { rssSchema } from '@astrojs/rss';

const blogCollection = defineCollection({
  type: 'content',
  schema: z
    .object({
      title: z.string(),
      pubDate: z.coerce.date(),
      tags: z.array(z.string()).optional(),
      draft: z.boolean().optional(),
    })
    .merge(rssSchema)
});

const listsCollection = defineCollection({
  type: 'content',
  schema: z
    .object({
      title: z.string(),
      tags: z.array(z.string()).optional(),
    }),
});

export const collections = {
  blog: blogCollection,
  lists: listsCollection,
}
