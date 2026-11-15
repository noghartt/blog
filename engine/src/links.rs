//! The links database: one record per outside URL, whatever the source
//! (Readwise Reader, ArchiveBox, or a link inside a note).
//!
//! Stored as `data/links.jsonl`: one JSON object per line, sorted by key, so
//! git diffs and the daily bot's PRs stay readable.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::io::Write;
use std::path::Path;
use url::Url;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    #[default]
    Page,
    Pdf,
    Video,
    Repo,
    Wiki,
    ArchWiki,
    Social,
}

impl Kind {
    /// The mark drawn after a link (see the F4 link kit).
    pub fn mark(self) -> &'static str {
        match self {
            Kind::Page => "↗",
            Kind::Pdf => "PDF",
            Kind::Video => "▶",
            Kind::Repo => "GH",
            Kind::Wiki => "W",
            Kind::ArchWiki => "AW",
            Kind::Social => "X",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Kind::Page => "another site",
            Kind::Pdf => "a file",
            Kind::Video => "a video",
            Kind::Repo => "code",
            Kind::Wiki => "Wikipedia",
            Kind::ArchWiki => "Arch Wiki",
            Kind::Social => "a post",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Source {
    /// Saved in Readwise Reader.
    Reader { id: String, saved_at: String },
    /// Snapshotted by ArchiveBox; `timestamp` is ArchiveBox's snapshot id.
    Archivebox {
        timestamp: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        archived_at: Option<String>,
    },
    /// Linked from a note in this garden, with the link's own text.
    Post { slug: String, text: String },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum Status {
    #[default]
    Unknown,
    Ok,
    Redirected {
        to: String,
    },
    Dead {
        code: u16,
    },
}

fn is_unknown(s: &Status) -> bool {
    *s == Status::Unknown
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Link {
    pub id: String,
    pub key: String,
    pub url: String,
    pub site: String,
    pub kind: Kind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub published: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sources: Vec<Source>,
    #[serde(default, skip_serializing_if = "is_unknown")]
    pub status: Status,
    /// Whether the page lets other sites show it in a frame; `None` until fetched.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frame: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fetched_at: Option<String>,
}

impl Link {
    pub fn saved_at(&self) -> Option<&str> {
        self.sources.iter().find_map(|s| match s {
            Source::Reader { saved_at, .. } => Some(saved_at.as_str()),
            _ => None,
        })
    }

    pub fn archived(&self) -> Option<(&str, Option<&str>)> {
        self.sources.iter().find_map(|s| match s {
            Source::Archivebox {
                timestamp,
                archived_at,
            } => Some((timestamp.as_str(), archived_at.as_deref())),
            _ => None,
        })
    }

    /// The text used for this link in a note, as a last-resort title.
    pub fn post_text(&self) -> Option<&str> {
        self.sources.iter().find_map(|s| match s {
            Source::Post { text, .. } if !text.is_empty() => Some(text.as_str()),
            _ => None,
        })
    }

    /// The fetched title, else the first text a note uses for the link that says
    /// what it is ("here" or "this post" don't, and would title a preview "here"),
    /// else the address itself.
    pub fn display_title(&self) -> String {
        let descriptive = self.sources.iter().find_map(|s| match s {
            Source::Post { text, .. } if !is_generic_link_text(text) => Some(text.clone()),
            _ => None,
        });
        self.title.clone().or(descriptive).unwrap_or_else(|| {
            let url = self.url.split_once("://").map_or(self.url.as_str(), |(_, rest)| rest);
            url.strip_prefix("www.").unwrap_or(url).trim_end_matches('/').to_owned()
        })
    }

    fn add_source(&mut self, source: Source) {
        // One source per (type, id): re-importing replaces instead of duplicating.
        let same = |a: &Source, b: &Source| match (a, b) {
            (Source::Reader { .. }, Source::Reader { .. }) => true,
            (Source::Archivebox { timestamp: x, .. }, Source::Archivebox { timestamp: y, .. }) => {
                x == y
            }
            (Source::Post { slug: x, .. }, Source::Post { slug: y, .. }) => x == y,
            _ => false,
        };
        if let Some(existing) = self.sources.iter_mut().find(|s| same(s, &source)) {
            *existing = source;
        } else {
            self.sources.push(source);
        }
    }
}

/// Link text that only points ("here", "this post", "link"): no title for a preview.
fn is_generic_link_text(text: &str) -> bool {
    const GENERIC: &[&str] = &[
        "", "here", "click here", "this", "this one", "that", "link", "this link", "source",
        "this post", "this article", "this video", "this page", "this thread", "this repo",
        "post", "article", "video", "page", "thread", "repo", "more", "read more",
    ];
    let t = text
        .trim()
        .trim_matches(|c: char| c.is_ascii_punctuation() || c == '\u{2060}')
        .to_lowercase();
    GENERIC.contains(&t.as_str())
}

const TRACKING: &[&str] = &[
    "__readwiseLocation",
    "fbclid",
    "gclid",
    "mc_cid",
    "mc_eid",
    "ref",
    "ref_src",
];

/// Removes tracking parameters and the fragment. Returns `None` for anything
/// that is not an http(s) URL.
pub fn clean(raw: &str) -> Option<Url> {
    let mut url = Url::parse(raw.trim()).ok()?;
    if !matches!(url.scheme(), "http" | "https") {
        return None;
    }
    url.set_fragment(None);
    let kept: Vec<(String, String)> = url
        .query_pairs()
        .filter(|(k, _)| !k.starts_with("utm_") && !TRACKING.contains(&k.as_ref()))
        .map(|(k, v)| (k.into_owned(), v.into_owned()))
        .collect();
    if kept.is_empty() {
        url.set_query(None);
    } else {
        url.query_pairs_mut().clear().extend_pairs(kept);
    }
    Some(url)
}

/// The identity of a link: the cleaned URL without scheme, `www.` or trailing slash,
/// so `http://www.example.com/a/` and `https://example.com/a` are the same record.
pub fn key_of(url: &Url) -> String {
    let host = url.host_str().unwrap_or("").trim_start_matches("www.");
    let path = url.path().trim_end_matches('/');
    match url.query() {
        Some(q) => format!("{host}{path}?{q}"),
        None => format!("{host}{path}"),
    }
}

pub fn site_of(url: &Url) -> String {
    url.host_str()
        .unwrap_or("")
        .trim_start_matches("www.")
        .to_owned()
}

pub fn kind_of(url: &Url) -> Kind {
    let host = site_of(url);
    if url.path().to_ascii_lowercase().ends_with(".pdf") {
        return Kind::Pdf;
    }
    match host.as_str() {
        "github.com" | "gist.github.com" => Kind::Repo,
        "wiki.archlinux.org" => Kind::ArchWiki,
        "youtube.com" | "m.youtube.com" | "youtu.be" => Kind::Video,
        "twitter.com" | "x.com" => Kind::Social,
        h if h.ends_with(".wikipedia.org") => Kind::Wiki,
        _ => Kind::Page,
    }
}

/// The logo drawn after a link to a well-known site, from `engine/static/icons/<name>.svg`.
/// `None` keeps the `↗` text mark. A PDF gets the document icon wherever it is hosted.
pub fn icon_of(url: &Url) -> Option<&'static str> {
    if kind_of(url) == Kind::Pdf {
        return Some("pdf");
    }
    let host = site_of(url);
    let h = host.as_str();
    let on = |domain: &str| h == domain || h.ends_with(&format!(".{domain}"));
    Some(match h {
        _ if on("github.com") => "github",
        _ if on("twitter.com") || on("x.com") => "x",
        _ if on("linkedin.com") => "linkedin",
        _ if on("youtube.com") || h == "youtu.be" => "youtube",
        _ if on("wikipedia.org") => "wikipedia",
        _ if on("archlinux.org") => "archlinux",
        _ if on("google.com") => "google",
        "news.ycombinator.com" => "ycombinator",
        "dev.to" => "devdotto",
        _ if h.starts_with("amazon.") => "amazon",
        _ if on("reddit.com") => "reddit",
        _ if on("stackoverflow.com") => "stackoverflow",
        "mastodon.social" | "hachyderm.io" | "fosstodon.org" => "mastodon",
        "bsky.app" => "bluesky",
        _ if on("medium.com") => "medium",
        _ if on("substack.com") => "substack",
        "arxiv.org" => "arxiv",
        _ => return None,
    })
}

/// The address to show in a popup's frame, or `None` to show the card. Every outside
/// page whose site was checked and allows frames gets one (`garden links frames` or
/// `fetch` checks); a site never checked shows the card, since a refused frame shows
/// only the browser's error page. Only https pages: browsers block http ones inside
/// an https page. YouTube videos and posts on X use their sites' embed pages.
pub fn frame_url(link: &Link) -> Option<String> {
    if matches!(link.status, Status::Dead { .. }) {
        return None;
    }
    let url = Url::parse(&link.url).ok()?;
    if let Some(embed) = youtube_embed(&url).or_else(|| tweet_embed(&url)) {
        return Some(embed);
    }
    (url.scheme() == "https" && link.frame == Some(true)).then(|| link.url.clone())
}

fn youtube_embed(url: &Url) -> Option<String> {
    let host = site_of(url);
    let id = if host == "youtu.be" {
        url.path().trim_start_matches('/').to_owned()
    } else if (host == "youtube.com" || host.ends_with(".youtube.com")) && url.path() == "/watch" {
        url.query_pairs().find(|(k, _)| k == "v")?.1.into_owned()
    } else {
        return None;
    };
    let id: String = id
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
        .collect();
    (!id.is_empty()).then(|| format!("https://www.youtube-nocookie.com/embed/{id}"))
}

/// A post on X (or Twitter) through X's own embed page, which allows frames.
fn tweet_embed(url: &Url) -> Option<String> {
    let host = site_of(url);
    if host != "twitter.com" && host != "x.com" && host != "mobile.twitter.com" {
        return None;
    }
    let parts: Vec<&str> = url.path_segments()?.filter(|s| !s.is_empty()).collect();
    match parts.as_slice() {
        [_, "status", id, ..] if id.chars().all(|c| c.is_ascii_digit()) => Some(format!(
            "https://platform.twitter.com/embed/Tweet.html?id={id}&dnt=true"
        )),
        _ => None,
    }
}

/// [`icon_of`] for an address as written.
pub fn icon_for(href: &str) -> Option<&'static str> {
    clean(href).as_ref().and_then(icon_of)
}

pub fn id_of(key: &str) -> String {
    blake3::hash(key.as_bytes()).to_hex()[..12].to_owned()
}

#[derive(Debug, Default)]
pub struct LinkDb {
    /// Keyed by `Link::key`.
    pub links: BTreeMap<String, Link>,
}

impl LinkDb {
    pub fn load(path: &Path) -> Result<Self> {
        let mut db = LinkDb::default();
        if !path.exists() {
            return Ok(db);
        }
        let text = std::fs::read_to_string(path)?;
        for (i, line) in text.lines().enumerate() {
            if line.trim().is_empty() {
                continue;
            }
            let link: Link = serde_json::from_str(line)
                .with_context(|| format!("{}:{}", path.display(), i + 1))?;
            db.links.insert(link.key.clone(), link);
        }
        Ok(db)
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let mut out = std::io::BufWriter::new(std::fs::File::create(path)?);
        for link in self.links.values() {
            serde_json::to_writer(&mut out, link)?;
            out.write_all(b"\n")?;
        }
        out.flush()?;
        Ok(())
    }

    /// Finds or creates the record for `raw`. Returns `None` for non-http URLs.
    pub fn entry(&mut self, raw: &str) -> Option<&mut Link> {
        let url = clean(raw)?;
        let key = key_of(&url);
        Some(self.links.entry(key.clone()).or_insert_with(|| Link {
            id: id_of(&key),
            key,
            site: site_of(&url),
            kind: kind_of(&url),
            url: url.to_string(),
            title: None,
            author: None,
            description: None,
            published: None,
            tags: vec![],
            sources: vec![],
            status: Status::Unknown,
            fetched_at: None,
            frame: None,
        }))
    }

    pub fn add(&mut self, raw: &str, source: Source) -> Option<&mut Link> {
        let link = self.entry(raw)?;
        link.add_source(source);
        Some(link)
    }

    /// Drops `Post` sources whose note no longer links there, and records left with no source.
    pub fn retain_posts(&mut self, live: &std::collections::HashSet<(String, String)>) {
        for link in self.links.values_mut() {
            let key = link.key.clone();
            link.sources.retain(|s| match s {
                Source::Post { slug, .. } => live.contains(&(key.clone(), slug.clone())),
                _ => true,
            });
        }
        self.links.retain(|_, l| !l.sources.is_empty());
    }

    /// Drops Reader sources that are no longer bookmarks (removed in Reader, or
    /// filtered out, like Feed items and podcasts); links left with no source go.
    pub fn retain_reader(&mut self, live: &std::collections::HashSet<String>) {
        for link in self.links.values_mut() {
            link.sources.retain(|s| match s {
                Source::Reader { id, .. } => live.contains(id),
                _ => true,
            });
        }
        self.links.retain(|_, l| !l.sources.is_empty());
    }
}

/// What counts as a bookmark: a document saved to the Reader library, not an item
/// sitting in the Feed from a subscription (podcasts, newsletters, channels), and
/// not a podcast episode, feed entry, email, note or highlight.
fn is_bookmark(category: Option<&str>, location: Option<&str>) -> bool {
    location != Some("feed")
        && !matches!(
            category,
            Some("rss" | "note" | "email" | "highlight" | "podcast")
        )
}

fn set_if_empty(slot: &mut Option<String>, value: Option<String>) {
    if slot.as_deref().is_none_or(str::is_empty) {
        if let Some(v) = value.filter(|v| !v.trim().is_empty()) {
            *slot = Some(v.trim().to_owned());
        }
    }
}

// ---------------------------------------------------------------------------
// Readwise Reader
// ---------------------------------------------------------------------------

/// The shape `scripts/generate-bookmarks-reader.ts` writes today.
#[derive(Deserialize)]
struct BookmarksFile {
    data: Vec<Bookmark>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Bookmark {
    id: String,
    title: String,
    url: String,
    saved_at: String,
    description: Option<String>,
    #[serde(default)]
    tags: Vec<String>,
    /// Written by the script since it learned to tell Feed items and podcasts apart;
    /// older files have neither.
    #[serde(default)]
    category: Option<String>,
    #[serde(default)]
    location: Option<String>,
}

/// Imports `src/content/bookmarks/bookmarks.json`, so the database can be built
/// without a Reader token (and keeps working with the existing daily workflow).
pub fn import_reader_json(db: &mut LinkDb, path: &Path) -> Result<usize> {
    let file: BookmarksFile = serde_json::from_str(&std::fs::read_to_string(path)?)
        .with_context(|| format!("reading {}", path.display()))?;
    let mut n = 0;
    let mut live = std::collections::HashSet::new();
    for b in file.data {
        if !is_bookmark(b.category.as_deref(), b.location.as_deref()) {
            continue;
        }
        live.insert(b.id.clone());
        let Some(link) = db.add(
            &b.url,
            Source::Reader {
                id: b.id,
                saved_at: b.saved_at,
            },
        ) else {
            continue;
        };
        // Reader is the source of truth for what it saved.
        link.title = Some(b.title)
            .filter(|t| !t.trim().is_empty())
            .or(link.title.take());
        set_if_empty(&mut link.description, b.description);
        for t in b.tags {
            if !link.tags.contains(&t) {
                link.tags.push(t);
            }
        }
        n += 1;
    }
    // The file is the whole list: anything Reader no longer has (or that is now
    // filtered out) leaves the database.
    db.retain_reader(&live);
    Ok(n)
}

#[derive(Deserialize)]
struct ReaderPage {
    results: Vec<ReaderDoc>,
    #[serde(rename = "nextPageCursor")]
    next_page_cursor: Option<String>,
}

#[derive(Deserialize)]
struct ReaderDoc {
    id: String,
    title: Option<String>,
    source_url: Option<String>,
    saved_at: Option<String>,
    summary: Option<String>,
    author: Option<String>,
    published_date: Option<serde_json::Value>,
    category: Option<String>,
    location: Option<String>,
    #[serde(default)]
    tags: Option<serde_json::Map<String, serde_json::Value>>,
}

/// Port of `scripts/generate-bookmarks-reader.ts`: pages through Reader's list API
/// and applies the same filters (see [`is_bookmark`], and nothing tagged `newsletter`).
/// Only a complete sync drops bookmarks Reader no longer has.
pub fn sync_reader(db: &mut LinkDb, token: &str) -> Result<usize> {
    let client = crate::fetch::client()?;
    let mut cursor: Option<String> = None;
    let mut n = 0;
    let mut live = std::collections::HashSet::new();
    loop {
        let mut req = client
            .get("https://readwise.io/api/v3/list/")
            .header("Authorization", format!("Token {token}"))
            .query(&[("page_size", "100")]);
        if let Some(c) = &cursor {
            req = req.query(&[("pageCursor", c)]);
        }
        let resp = req.send()?;
        if resp.status().as_u16() == 429 {
            let wait = resp
                .headers()
                .get("Retry-After")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.parse().ok())
                .unwrap_or(30u64);
            eprintln!("reader: rate limited, waiting {wait}s");
            std::thread::sleep(std::time::Duration::from_secs(wait));
            continue;
        }
        let page: ReaderPage = resp.error_for_status()?.json()?;
        for doc in page.results {
            let skip_category = !is_bookmark(doc.category.as_deref(), doc.location.as_deref());
            let tags: Vec<String> = doc
                .tags
                .unwrap_or_default()
                .values()
                .filter_map(|t| t.get("name").and_then(|n| n.as_str()).map(str::to_owned))
                .collect();
            if skip_category || tags.iter().any(|t| t == "newsletter") {
                continue;
            }
            live.insert(doc.id.clone());
            let (Some(url), Some(saved_at)) = (doc.source_url, doc.saved_at) else {
                continue;
            };
            let Some(link) = db.add(
                &url,
                Source::Reader {
                    id: doc.id,
                    saved_at,
                },
            ) else {
                continue;
            };
            link.title = doc
                .title
                .filter(|t| !t.trim().is_empty())
                .or(link.title.take());
            set_if_empty(&mut link.description, doc.summary);
            set_if_empty(&mut link.author, doc.author);
            set_if_empty(
                &mut link.published,
                doc.published_date.map(|v| match v {
                    serde_json::Value::String(s) => s,
                    other => other.to_string(),
                }),
            );
            for t in tags {
                if !link.tags.contains(&t) {
                    link.tags.push(t);
                }
            }
            n += 1;
        }
        match page.next_page_cursor {
            Some(c) => cursor = Some(c),
            None => break,
        }
    }
    db.retain_reader(&live);
    Ok(n)
}

// ---------------------------------------------------------------------------
// ArchiveBox
// ---------------------------------------------------------------------------

/// Imports the output of `archivebox list --json` (or a saved copy of it).
/// Only `url`, `timestamp`, `title` and a date are read, so small differences
/// between ArchiveBox versions do not matter.
pub fn import_archivebox(db: &mut LinkDb, path: &Path) -> Result<usize> {
    let value: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(path)?)
        .with_context(|| format!("reading {}", path.display()))?;
    let items = match &value {
        serde_json::Value::Array(a) => a.clone(),
        serde_json::Value::Object(o) => o
            .get("links")
            .or_else(|| o.get("snapshots"))
            .or_else(|| o.get("items"))
            .and_then(|v| v.as_array().cloned())
            .unwrap_or_default(),
        _ => vec![],
    };
    let s = |item: &serde_json::Value, k: &str| {
        item.get(k)
            .and_then(|v| v.as_str())
            .map(str::to_owned)
            .filter(|v| !v.is_empty())
    };
    let mut n = 0;
    for item in &items {
        let (Some(url), Some(timestamp)) = (s(item, "url"), s(item, "timestamp")) else {
            continue;
        };
        let archived_at = s(item, "updated")
            .or_else(|| s(item, "bookmarked_at"))
            .or_else(|| s(item, "created_at"));
        let title = s(item, "title");
        if let Some(link) = db.add(
            &url,
            Source::Archivebox {
                timestamp,
                archived_at,
            },
        ) {
            set_if_empty(&mut link.title, title);
            n += 1;
        }
    }
    Ok(n)
}
