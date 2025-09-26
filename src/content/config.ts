import { defineCollection, z } from 'astro:content';
import { rssSchema } from '@astrojs/rss';

const blogCollection = defineCollection({
  type: 'content',
  schema: z
    .object({
      title: z.string(),
      pubDate: z.coerce.date(),
      tags: z.array(z.string()).default([]).optional(),
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

const tilCollection = defineCollection({
  type: 'content',
  schema: z.object({
    title: z.string(),
    tags: z.array(z.string()).default([]).optional(),
    createdAt: z.coerce.date(),
  }),
});

const changelogCollection = defineCollection({
  type: 'data',
  schema: z.array(z.object({
    time: z.string().datetime(),
    title: z.string(),
  })),
});

const bookmarksCollection = defineCollection({
  type: 'data',
  schema: z.object({
    lastUpdate: z.string().datetime(),
    data: z.array(z.object({
      id: z.string(),
      title: z.string(),
      url: z.string(),
      savedAt: z.coerce.date(),
      description: z.string().nullable(),
      tags: z.array(z.string()),
    })),
  }),
});

export const collections = {
  blog: blogCollection,
  lists: listsCollection,
  til: tilCollection,
  changelog: changelogCollection,
  bookmarks: bookmarksCollection,
}
