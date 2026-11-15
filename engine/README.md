# garden

A static site generator for noghartt.dev, written in Rust, built around a
database of links. It builds the live site: `.github/workflows/deploy.yml` runs
`garden links import-reader-json` (the daily bookmarks update) and `garden build`
on every push to `main`, and publishes `engine/dist` to GitHub Pages.

It reads the same `src/content/**` as Astro, so nothing in the content moves.

## Commands

Run from anywhere inside the repository, in the dev shell (`nix develop`):

```sh
cargo run --release --manifest-path engine/Cargo.toml -- build          # -> engine/dist
cargo run --release --manifest-path engine/Cargo.toml -- serve          # localhost:4321, rebuilds on change
cargo run --release --manifest-path engine/Cargo.toml -- compare <astro-dist>

# links database (data/links.jsonl)
garden links import-reader-json     # from src/content/bookmarks/bookmarks.json (what the daily workflow writes)
garden links sync-reader            # straight from Readwise Reader, needs READWISE_READER_API_KEY
garden links import-archivebox <archivebox list --json output>
garden links scan                   # record every outside link found in the notes
garden links fetch [--note <slug>] [--cited-only] [--limit N] [--refresh]
garden links frames [--refresh]     # which sites allow popup frames (headers only, 12 at a time)
garden links stats
```

`build` never touches the network. Only `links sync-reader`, `links fetch` and
`links frames` do, and only when run on purpose.

## Pages

Everything the Astro site had: `/`, `/blog/` and every post (drafts and hidden
posts too, at their own address but unlisted; drafts are listed at `/blog/drafts/`),
`/tags/` and `/tag/<tag>/`, `/lists/` and every list, `/til/`, `/now/` (from
`engine/content/now.md`), `/changelog/` (from `src/content/changelog/changelog.json`),
`/bookmarks/`, `/bookmarks/tags/` and `/bookmarks/tags/<tag>/` (a tag holds its
subtags), `/rss.xml`, `/bookmarks/rss.xml`. New: `/bookmarks/site/<site>/` for every
site with 3 or more bookmarks. Any other `engine/content/<name>.md` becomes `/<name>/`.

Also as on the Astro site: every post's old address (`/<slug>/`) redirects to
`/blog/<slug>/` (a page that refreshes to it, since GitHub Pages can't redirect),
`sitemap-index.xml` (without drafts, hidden notes or redirects), Umami analytics
(production builds only), and canonical, Open Graph and Twitter tags on every page
(a note's description is its first paragraph). Math (`$…$`, `$$…$$`) becomes MathML.

Every page also gets a share image, `og.png` next to its `index.html` (`src/og.rs`):
the header's mark and "Noghartt's garden", the page's section, title, description,
address and (for notes) date, drawn with resvg in the site's fonts, compiled in
from `engine/fonts` (SIL OFL). Images are cached in `engine/target/og-cache` by a
hash of what they show; bump `VERSION` in `og.rs` after changing the drawing.

## How a build works

```
src/content/** ──► analyze every note ──► graph ──► render every page ──► engine/dist
data/links.jsonl ─────────────────────────┘
```

1. **Analyze** (`markdown::analyze`): every link with the sentence around it,
   headings (for contents), the first paragraph (for previews), a word count.
2. **Graph** (`site::build`): backlinks between notes, and which notes cite each
   outside link. This is why there are two passes: a note's "Linked from"
   depends on every other note.
3. **Render** (`markdown::Renderer`): comrak with a custom formatter, then
   minijinja templates in `engine/templates`.

## Markdown extensions

All of them stay plain markdown: Obsidian and GitHub still read the files.

| You write | It renders as |
|---|---|
| `[[slug]]`, `[[slug\|text]]`, `[[slug#heading]]` | A link to that note, counted as a backlink; `[[slug]]` shows the note's title |
| `[^1]` | A numbered sidenote in the margin (tap the number on a phone); its number in the text and in the note link to each other, and hovering either lights up both |
| `[^m-anything]` | An unnumbered margin note |
| Any outside link | A mark after it (see below) and a preview card if the database knows it |
| A link to `/blog/<slug>` or `https://noghartt.dev/blog/<slug>` | Same as a wikilink |

Link marks come from the address. Well-known sites get their logo
(`links::icon_of`: GitHub, X, LinkedIn, YouTube, Wikipedia, Arch Linux, Google,
Hacker News, dev.to, Amazon, Reddit, Stack Overflow, Mastodon, Bluesky, Medium,
Substack, arXiv), an envelope for `mailto:` and a document for any `.pdf`, wherever
it is hosted. Everything else gets a `↗` text mark. Both are CSS on `a[data-icon]`
and `a[data-mark]`, so they show without JavaScript. The logos are
[Simple Icons](https://simpleicons.org) (CC0) in `engine/static/icons/` (the
envelope and the document are hand-drawn), drawn
through a CSS mask in the text colour. To add a site, drop its SVG there, add a
line to `icon_of` and one `[data-icon="…"]` rule to `garden.css`.

## Popups

Hovering a link (tapping it on a phone) opens a popup (`static/garden.js`). Every
page ships a `<template>` per link it can preview, so the popup shows at once; for a
note it then fetches the note's page and shows the whole text, which scrolls inside
the popup. A link to a heading (`#section`, the contents, `[[slug#heading]]`) shows
just that section. Links inside a popup open their own popups. The title bar closes,
maximizes, collapses and pins it; dragging the bar moves it and the corner grip
resizes it (both pin it); a popup opened from a maximized one opens maximized; Esc
closes the top one. Without JavaScript the links are
plain links.

A maximized note popup shows the note's own page (without the site's header and
footer), so it looks exactly like the page.

What an outside link's popup shows depends on where it points:

| Link | Popup |
|---|---|
| GitHub file (`/blob/…`, with `#L10-L20`) | The file's code, those lines marked |
| GitHub commit, pull request, comparison | The diff |
| GitHub repository | Like its page: description, stars, forks, language, licence, topics, the root files, the README |
| GitHub folder (`/tree/…`) | Its files (each file or folder opens its own popup) |
| GitHub issue; gist | The issue / the files |
| YouTube video, post on X | The site's own embed player |
| Anything else | The page in a frame, unless its site refuses (then the card) |

GitHub views are read in the reader's browser from GitHub's API and raw files
(`api.github.com` allows 60 requests an hour per reader); until they arrive, or if
GitHub refuses, the popup shows the card.

A link to an outside page otherwise shows the page itself in a frame when its site
is known to allow it, and its card otherwise. `garden links frames` (or `fetch`)
reads each page's `X-Frame-Options` and `Content-Security-Policy: frame-ancestors`
headers into the record's `frame` field; a site never checked shows the card (a
refused frame would only show the browser's error page). Only https pages get a
frame: browsers block http ones inside an https page. Frames are sandboxed (they
can't take over the tab).

Frames load ahead, one at a time, once the page has loaded, so their popups open
ready; that means a reader's browser contacts those sites without hovering
anything. The bookmarks pages (thousands of links) load nothing ahead: a frame loads
only when its link is hovered, and their cards live in one shared file
(`/bookmarks/cards.html`) loaded on the first hover.

A bookmark is a document saved to the Reader library: not an item sitting in the
Feed from a subscription (podcasts, newsletters, channels), not a podcast episode,
feed entry, email, note or highlight, and not tagged `newsletter`
(`links::is_bookmark`, the same rules as `scripts/generate-bookmarks-reader.ts`).
Importing drops bookmarks Reader no longer has.

Every link gets a popup (`markLinks` in `garden.js`): another page of this site
(the header, crumbs, tags, bookmark topics and sites, the tag tree) shows that page;
a feed shows its latest items; a `#` link shows the section, sidenote or paragraph
it points to; an outside address with no card shows a small card from the link
itself (GitHub's view for GitHub). Only permalinks (a link inside what it points
to, like `¶`) and `mailto:` links (they open the mail app) have none.

## Security

- Text from Reader (titles, descriptions, tags) is escaped by the templates; the
  links database keeps only `http`/`https` addresses, so no `javascript:` link
  gets in. Tags become folders only after dropping `.`/`..` parts, and no page is
  written outside the output folder.
- A Content-Security-Policy `<meta>` (GitHub Pages can't send headers) allows only
  this site's scripts (no inline ones), frames from https pages, and fetches from
  this site, `api.github.com` and `raw.githubusercontent.com`.
- Outside pages in popups are sandboxed: they can't navigate the tab, submit forms
  or use camera, microphone or location. GitHub's rendered READMEs and issues run
  no scripts. Addresses read from GitHub's API or a feed become links only if
  http(s).
- Privacy: readers' browsers load Google Fonts, and frames and GitHub data for the
  links they hover (and, outside the bookmarks pages, frames ahead of time).

## The links database

`data/links.jsonl`: one JSON object per line, sorted by key, so diffs are
readable. One record per outside URL, whatever the source:

```json
{"id":"46ed6311f64b","key":"gwern.net/sidenote","url":"https://gwern.net/sidenote","site":"gwern.net","kind":"page",
 "title":"Sidenotes In Web Design","author":"Gwern","description":"In typography/design, ‘sidenotes’ place …","published":"2020-08-06",
 "sources":[{"type":"post","slug":"blog/on-sidenotes-implementation","text":"Sidenotes in Web Design"}],
 "status":{"state":"ok"},"fetched_at":"2026-11-14T20:18:22Z"}
```

- **key** is the cleaned URL (no scheme, `www.`, trailing slash, fragment,
  `utm_*` or `__readwiseLocation`), so the same article from Reader, ArchiveBox
  and a note becomes one record.
- **sources**: `reader` (id, saved_at), `archivebox` (snapshot timestamp,
  archived_at), `post` (note, link text).
- **frame**: whether the site lets other sites show the page in a frame, from its
  headers when fetched.
- Set `GARDEN_ARCHIVE_URL` to your ArchiveBox's public address to make preview
  cards link to the archived copy.

## Not ported yet

The generated share images for posts and lists (`og.png`, satori + resvg): link
previews show the title and description without an image. Dark mode.
