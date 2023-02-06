import { defineCollection, z } from 'astro:content';

const blogCollection = defineCollection({
  schema: z.object({
    title: z.string(),
    tags: z.optional(z.array(z.string())),
    draft: z.optional(z.boolean()),
    pubDate: z.date(),
  }),
});

export const collection = {
  blog: blogCollection,
}
