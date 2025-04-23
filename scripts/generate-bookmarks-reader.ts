import fs from 'fs/promises';
import path from 'path';
import { setTimeout } from 'timers/promises';

const READWISE_API_URL = 'https://readwise.io/api/v3/list/';
const READWISE_READER_API_KEY = `Token ${process.env.READWISE_READER_API_KEY}`;

const fetchBookmarks = async ({ nextPage: nextPageArg } = { nextPage: null }) => {
  const queryParams = new URLSearchParams();
  if (nextPageArg) {
    queryParams.append('pageCursor', nextPageArg);
  }

  try {
    const url = `${READWISE_API_URL}?${queryParams.toString()}`;
    const response = await fetch(url, {
      method: 'GET',
      headers: {
        'Content-Type': 'application/json',
        Authorization: READWISE_READER_API_KEY,
      },
    });

    if (!response.ok) {
      if (response.status === 429) {
        const retryAfter = response.headers.get('Retry-After');
        if (retryAfter) {
          const retryAfterSeconds = parseInt(retryAfter, 10);
          console.log(`Rate limit exceeded. Retrying after ${retryAfterSeconds} seconds...`);
          await setTimeout(retryAfterSeconds * 1000);
          return fetchBookmarks({ nextPage: nextPageArg });
        }
      }

      return { error: response.statusText, data: null };
    }

    const data = await response.json();

    const dataFiltered = data.results.filter(bookmark => {
      if (!['article', 'video'].includes(bookmark.category)) {
        return false;
      }

      if (bookmark.tags.newsletter) {
        return false;
      }

      return true;
    });
    const nextPage = data.nextPageCursor;

    return {
      data: dataFiltered,
      nextPage,
    }
  } catch (err) {
    console.log('Error while fetching bookmarks:', err);
    return { error: err, data: null };
  }
}

const writeJsonFile = async (data: any) => {
  const cwd = process.cwd();
  const filepath = path.join(cwd, 'src', 'pages', 'bookmarks', '_bookmarks.json');
  await fs.writeFile(filepath, JSON.stringify({ lastUpdate: new Date().toISOString(), data }, null, 2));
}

(async () => {
  const bookmarks = [];
  let nextPage = null;
  try {
    do {
      console.log('Fetching bookmarks from:', nextPage ?? 'start');
      const response = await fetchBookmarks({ nextPage });
      if (response.error) {
        console.log('Error while fetching:', response.error);
        break;
      }

      nextPage = response.nextPage;
      bookmarks.push(...response.data);
    } while (nextPage);


  } catch (e) {
    console.log('Error while fetching:', e);
  }

  const mappedBookmarks = bookmarks.map(bookmark => ({
    id: bookmark.id,
    title: bookmark.title,
    url: bookmark.source_url,
    savedAt: bookmark.saved_at,
    description: bookmark.summary || null,
    tags: Object.values(bookmark.tags).map(tags => tags.name),
  }));

  await writeJsonFile(mappedBookmarks);
})();