import { defineCollection, z } from 'astro:content';
import { rssSchema } from '@astrojs/rss';

const blogCollection = defineCollection({
  type: 'content',
  schema: z
    .object({
      slug: z.string().optional(),
      title: z.string(),
      pubDate: z.coerce.date(),
      tags: z.optional(z.array(z.string())),
      draft: z.boolean().optional(),
    })
    .merge(rssSchema)
});

const listsCollection = defineCollection({
  type: 'content',
  schema: z
    .object({
      tags: z.optional(z.array(z.string())),
    })
    .merge(rssSchema)
});

export const collections = {
  blog: blogCollection,
  lists: listsCollection,
}
