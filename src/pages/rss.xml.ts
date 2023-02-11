import rss from "@astrojs/rss";
import { getCollection } from "astro:content";
import sanitizeHtml from 'sanitize-html';
import MarkdownIt from 'markdown-it';

import { SITE } from "../config";

const parser = new MarkdownIt();

export async function get() {
  const posts = await getCollection("blog", ({ data }) => !data.draft);

  return rss({
    title: SITE.title,
    description: SITE.description,
    site: SITE.website,
    items: posts.map(({ data, slug, body }) => ({
      link: slug,
      title: data.title,
      description: data.description,
      pubDate: data.pubDate,
      content: sanitizeHtml(parser.render(body))
    })),
  });
}