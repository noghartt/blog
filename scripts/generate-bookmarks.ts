import fs from 'fs/promises';
import path from 'path';

const OMNIVORE_API_URL = 'https://api-prod.omnivore.app/api/graphql';
const OMNIVORE_API_KEY = process.env.OMNIVORE_API_KEY;

const query = `\
  query SearchBookmarks($first: Int, $after: String, $query: String) {
    search(first: $first, query: $query, after: $after) {
      ... on SearchSuccess {
        pageInfo {
          hasNextPage
          endCursor
          totalCount
        }
        edges {
          cursor
          node {
            id
            title
            url: originalArticleUrl
            description
            labels {
              name
            }
            savedAt
          }
        }
      }

      ... on SearchError {
        errorCodes
      }
    }
  }
`;

const fetchBookmarks = async (first: number, after: string) => {
  console.log('Fetching bookmarks:', { first, after })

  try {
    const response = await fetch(OMNIVORE_API_URL, {
      method: 'POST',
      body: JSON.stringify({ query, variables: { first, after, query: "no:subscription" } }),
      headers: {
        'Content-Type': 'application/json',
        Authorization: OMNIVORE_API_KEY,
      },
    });

    if (!response.ok) {
      return { error: response.statusText, data: null };
    }

    return {
      error: null,
      data: await response.json(),
    }
  } catch (err) {
    return { error: err, data: null };
  }
}

const mapBookmark = (edge) => {
  const { node } = edge;
  return {
    id: node.id,
    title: node.title,
    url: node.url,
    savedAt: node.savedAt,
    description: node.description || null,
    tags: node.labels.map(({ name }) => name),
  };
}

const writeJsonFile = async (data: any) => {
  const cwd = process.cwd();
  const filepath = path.join(cwd, 'src', 'pages', 'bookmarks', 'bookmarks.json');
  await fs.writeFile(filepath, JSON.stringify({ lastUpdate: new Date().toISOString(), data }, null, 2));
}

(async () => {
  const bookmarks = await fetchBookmarks(1000, null);
  if (bookmarks.error) {
    console.log(bookmarks.error);
    return;
  }

  const { edges } = bookmarks.data.data.search;

  const bookmarksList = edges.map(mapBookmark);

  let after = edges[edges.length - 1].cursor;
  let hasNextPage = bookmarks.data.data.search.pageInfo.hasNextPage;
  while (hasNextPage) {
    const nextBookmarks = await fetchBookmarks(1000, after);
    if (nextBookmarks.error) {
      console.log(nextBookmarks.error);
      return;
    }

    after = nextBookmarks.data.data.search.pageInfo.endCursor;
    hasNextPage = nextBookmarks.data.data.search.pageInfo.hasNextPage;

    const alreadyExists = bookmarksList.find(bookmark => bookmark.url === nextBookmarks.data.data.search.edges[0].node.url)

    if (!alreadyExists) {
      bookmarksList.push(...nextBookmarks.data.data.search.edges.map(mapBookmark));
    } else {
      console.log('Already exists:', nextBookmarks.data.data.search.edges[0].node.url);
    }
  }

  await writeJsonFile(bookmarksList);
})();
