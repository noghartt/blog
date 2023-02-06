import rss from "@astrojs/rss";
import { getCollection } from "astro:content";

import { SITE } from "../config";

export async function get() {
  const posts = await getCollection("blog", ({ data }) => !data.draft);

  return rss({
    title: SITE.title,
    description: SITE.description,
    site: SITE.website,
    items: posts.map(({ data, slug }) => ({
      link: slug,
      title: data.title,
      description: data.description,
      pubDate: data.pubDate,
    })),
  });
}