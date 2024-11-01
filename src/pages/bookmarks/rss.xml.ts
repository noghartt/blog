import rss from "@astrojs/rss";

import { SITE } from "../../config";
import bookmarksJson from './_bookmarks.json';

const bookmarksData = bookmarksJson.data;
const bookmarksSorted = bookmarksData.sort((a, b) =>
  Math.floor(Number(b.savedAt) / 1000) -
  Math.floor(Number(a.savedAt) / 1000)
);

export async function GET() {
  const items = bookmarksSorted
    .sort((a, b) => new Date(b.savedAt) - new Date(a.savedAt))
    .map(({ title, description, url, savedAt }) => {
      const getDescription = () => {
        if (description) {
          return {
            description,
            content: description,
          };
        }

        return {};
      }

      return {
        ...getDescription(),
        link: url,
        title,
        pubDate: savedAt,
      }
    })
    .filter(Boolean);

  return rss({
    title: `${SITE.title} | Bookmarks`,
    description: SITE.description,
    site: SITE.website,
    items,
  });
}
