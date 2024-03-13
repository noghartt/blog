export const prerender = false;

const OMNIVORE_API_URL = 'https://api-prod.omnivore.app/api/graphql';
const OMNIVORE_API_KEY = import.meta.env.OMNIVORE_API_KEY;

const query = `\
  query SearchBookmarks {
    search(first: 1000, query: "no:subscription") {
      ... on SearchSuccess {
        edges {
          node {
            id
            title
            url
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

export const bookmarksGet = async () => {
  try {
    const bookmarks = await fetch(OMNIVORE_API_URL, {
      method: 'POST',
      body: JSON.stringify({ query }),
      headers: {
        'Content-Type': 'application/json',
        Authorization: OMNIVORE_API_KEY,
      },
    });

    const json = await bookmarks.json();

    return new Response(
      JSON.stringify(json),
      {
        status: 200,
        headers: {
          'Content-Type': 'application/json',
        },
      }
    );
  } catch (err) {
    return new Response(
      err,
      {
        status: 400,
        statusText: 'Error while fetching bookmarks',
        headers: {
          'content-type': 'text/html; charset=UTF-8',
        },
      }
    );
  }
}
