//! `garden build`: load -> analyze every note -> graph -> render -> write `dist/`.
//!
//! Two passes are needed because a note's "Linked from" depends on every other
//! note, so nothing is rendered until every note has been read.

use crate::content::{self, Kind, Note};
use crate::links::{Link, LinkDb};
use crate::markdown::{self, Analysis, Env, LinkRef, Renderer, Target};
use anyhow::{Context, Result};
use chrono::{DateTime, Datelike, FixedOffset};
use minijinja::{context, path_loader, Environment};
use serde::Serialize;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub struct Paths {
    pub root: PathBuf,
    pub out: PathBuf,
}

impl Paths {
    pub fn new(root: PathBuf, out: Option<PathBuf>) -> Self {
        let out = out.unwrap_or_else(|| root.join("engine/dist"));
        Paths { root, out }
    }
    pub fn db(&self) -> PathBuf {
        self.root.join("data/links.jsonl")
    }
    pub fn bookmarks_json(&self) -> PathBuf {
        self.root.join("src/content/bookmarks/bookmarks.json")
    }
    pub fn templates(&self) -> PathBuf {
        self.root.join("engine/templates")
    }
    pub fn static_dir(&self) -> PathBuf {
        self.root.join("engine/static")
    }
    pub fn public(&self) -> PathBuf {
        self.root.join("public")
    }
    /// Everything `serve` watches.
    pub fn watched(&self) -> Vec<PathBuf> {
        vec![
            self.root.join("src/content"),
            self.root.join("engine/content"),
            self.templates(),
            self.static_dir(),
            self.root.join("data"),
        ]
    }
}

#[derive(Default, Clone, Copy)]
pub struct BuildOptions {
    pub drafts: bool,
    pub dev: bool,
}

#[derive(Debug, Default)]
pub struct Stats {
    pub pages: usize,
    pub notes: usize,
    pub backlinks: usize,
    pub links_out: usize,
    pub links_known: usize,
}

// ---------------------------------------------------------------------------
// Template data
// ---------------------------------------------------------------------------

#[derive(Serialize, Clone)]
struct NoteMeta {
    key: String,
    slug: String,
    title: String,
    url: String,
    kind: Kind,
    date: Option<String>,
    date_iso: Option<String>,
    year: Option<i32>,
    tags: Vec<String>,
    minutes: usize,
    hidden: bool,
}

#[derive(Serialize, Clone)]
struct Backlink {
    key: String,
    title: String,
    url: String,
    date_iso: Option<String>,
    context: String,
}

#[derive(Serialize, Clone)]
struct LinkOut {
    mark: &'static str,
    preview: Option<String>,
    icon: Option<&'static str>,
    title: String,
    url: String,
    site: String,
    times: usize,
}

/// One preview card. Pages ship a `<template>` per card they link to; no request
/// is made while someone reads.
#[derive(Serialize, Clone)]
struct Preview {
    key: String,
    /// The page to show in the popup, when its site allows frames.
    frame: Option<String>,
    is_note: bool,
    path: String,
    mark: &'static str,
    icon: Option<&'static str>,
    title: String,
    meta: Option<String>,
    text: Option<String>,
    foot: Vec<String>,
    archive: Option<String>,
}

#[derive(Serialize)]
struct ReadingItem {
    day: String,
    title: String,
    url: String,
    site: String,
    /// Whether the site has its own page (/bookmarks/site/<site>/).
    site_page: bool,
    tags: Vec<String>,
    preview: String,
}

/// A bookmark tag in the tree at /bookmarks/tags/: `cs` holds `cs/distsys`.
#[derive(Serialize)]
struct TagNode {
    name: String,
    path: String,
    count: usize,
    children: Vec<TagNode>,
}

fn tag_tree(counts: &BTreeMap<String, usize>, parent: Option<&str>) -> Vec<TagNode> {
    counts
        .iter()
        .filter(|(k, _)| match parent {
            None => !k.contains('/'),
            Some(p) => k
                .strip_prefix(p)
                .and_then(|r| r.strip_prefix('/'))
                .is_some_and(|r| !r.contains('/')),
        })
        .map(|(k, c)| TagNode {
            name: k.rsplit('/').next().unwrap_or(k).to_owned(),
            path: k.clone(),
            count: *c,
            children: tag_tree(counts, Some(k)),
        })
        .collect()
}

#[derive(Serialize)]
struct ReadingMonth {
    name: String,
    total: usize,
    items: Vec<ReadingItem>,
    more: usize,
}

fn long_date(d: &DateTime<FixedOffset>) -> String {
    d.format("%-d %B %Y").to_string()
}

fn meta_of(n: &Note, a: Option<&Analysis>) -> NoteMeta {
    NoteMeta {
        key: n.key(),
        slug: n.slug.clone(),
        title: n.title.clone(),
        url: n.url.clone(),
        kind: n.kind,
        date: n.date.as_ref().map(long_date),
        date_iso: n.date.as_ref().map(|d| d.format("%Y-%m-%d").to_string()),
        year: n.date.as_ref().map(|d| d.year()),
        tags: n.tags.clone(),
        minutes: a.map_or(1, |a| (a.words / 220).max(1)),
        hidden: n.hidden,
    }
}

fn archive_url(timestamp: &str) -> Option<String> {
    let base = std::env::var("GARDEN_ARCHIVE_URL").ok()?;
    Some(format!(
        "{}/archive/{timestamp}/index.html",
        base.trim_end_matches('/')
    ))
}

fn link_preview(l: &Link, cited_in: usize) -> Preview {
    let mut foot = vec![];
    if let Some(saved) = l.saved_at() {
        foot.push(format!("saved {}", &saved[..10.min(saved.len())]));
    }
    let mut archive = None;
    if let Some((ts, at)) = l.archived() {
        foot.push(match at {
            Some(at) => format!("archived {}", &at[..10.min(at.len())]),
            None => "archived".to_owned(),
        });
        archive = archive_url(ts);
    }
    if cited_in > 0 {
        foot.push(format!(
            "cited in {cited_in} note{}",
            if cited_in == 1 { "" } else { "s" }
        ));
    }
    if foot.is_empty() {
        foot.push(l.kind.label().to_owned());
    }
    let meta = match (&l.author, &l.published) {
        (Some(a), Some(p)) => Some(format!("{a} · {}", &p[..10.min(p.len())])),
        (Some(a), None) => Some(a.clone()),
        (None, Some(p)) => Some(p[..10.min(p.len())].to_owned()),
        (None, None) => None,
    };
    Preview {
        key: format!("l:{}", l.id),
        frame: crate::links::frame_url(l),
        is_note: false,
        path: l.site.clone(),
        mark: l.kind.mark(),
        icon: crate::links::icon_for(&l.url),
        title: l.display_title(),
        meta,
        text: l.description.as_deref().map(|d| markdown::truncate(d, 300)),
        foot,
        archive,
    }
}

// ---------------------------------------------------------------------------
// Build
// ---------------------------------------------------------------------------

/// Builds into a folder beside the output and swaps it in when done, so `serve`
/// never serves a half-built site (or a 404 while a build runs).
pub fn build(paths: &Paths, opts: BuildOptions) -> Result<Stats> {
    let name = paths
        .out
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "dist".to_owned());
    let next = paths.out.with_file_name(format!("{name}.next"));
    let old = paths.out.with_file_name(format!("{name}.old"));
    let staged = Paths {
        root: paths.root.clone(),
        out: next.clone(),
    };
    let stats = build_into(&staged, opts)?;
    if old.exists() {
        std::fs::remove_dir_all(&old)?;
    }
    if paths.out.exists() {
        std::fs::rename(&paths.out, &old)?;
    }
    std::fs::rename(&next, &paths.out)?;
    if old.exists() {
        std::fs::remove_dir_all(&old)?;
    }
    Ok(stats)
}

fn build_into(paths: &Paths, opts: BuildOptions) -> Result<Stats> {
    let mut stats = Stats::default();
    let all = content::load(&paths.root)?;
    let pages = content::load_pages(&paths.root)?;
    let db = LinkDb::load(&paths.db())?;

    // Every note gets its page, drafts and hidden ones too (as on the Astro site);
    // only listed ones appear in indexes, feeds and backlinks. Drafts have their own
    // index at /blog/drafts/.
    let _ = opts.drafts;
    let notes: Vec<&Note> = all.iter().collect();

    // Resolver shared by every page.
    let mut env = Env::default();
    for n in &notes {
        env.add_note(n.key(), &n.slug, n.title.clone(), n.url.clone());
        if let Some(f) = n.path.file_name() {
            env.add_file(&f.to_string_lossy(), n.key());
        }
    }
    for l in db.links.values() {
        env.links
            .insert(l.key.clone(), LinkRef { id: l.id.clone() });
    }
    let env = Arc::new(env);

    // Pass 1: read everything.
    let mut analyses: HashMap<String, Analysis> = HashMap::new();
    for n in notes.iter().copied().chain(pages.iter()) {
        analyses.insert(n.key(), markdown::analyze(&n.body, &env));
    }

    // Graph: backlinks between notes, and which notes cite each outside link.
    let by_key: HashMap<String, &Note> = notes.iter().map(|n| (n.key(), *n)).collect();
    let mut backlinks: HashMap<String, Vec<Backlink>> = HashMap::new();
    let mut cited: HashMap<String, HashSet<String>> = HashMap::new();
    for n in notes.iter().filter(|n| n.is_listed()) {
        let mut seen = HashSet::new();
        for l in &analyses[&n.key()].links {
            match &l.target {
                Target::Note(target, _) if *target != n.key() && seen.insert(target.clone()) => {
                    backlinks.entry(target.clone()).or_default().push(Backlink {
                        key: n.key(),
                        title: n.title.clone(),
                        url: n.url.clone(),
                        date_iso: n.date.as_ref().map(|d| d.format("%Y-%m-%d").to_string()),
                        context: l.context.clone(),
                    });
                    stats.backlinks += 1;
                }
                Target::External { key, .. } => {
                    cited.entry(key.clone()).or_default().insert(n.key());
                }
                _ => {}
            }
        }
    }
    for list in backlinks.values_mut() {
        list.sort_by(|a, b| b.date_iso.cmp(&a.date_iso));
    }

    let note_preview = |key: &str| -> Option<Preview> {
        let n = by_key.get(key)?;
        let a = analyses.get(key);
        let meta = meta_of(n, a);
        let mut line = meta.date.clone().unwrap_or_default();
        if !meta.tags.is_empty() {
            if !line.is_empty() {
                line.push_str(" · ");
            }
            line.push_str(&meta.tags.join(", "));
        }
        let back = backlinks.get(key).map_or(0, Vec::len);
        Some(Preview {
            frame: None,
            key: format!("n:{key}"),
            is_note: true,
            path: n.url.clone(),
            mark: "",
            icon: None,
            title: n.title.clone(),
            meta: Some(line).filter(|l| !l.is_empty()),
            text: a.map(|a| a.excerpt.clone()).filter(|e| !e.is_empty()),
            foot: match back {
                0 => vec![],
                1 => vec!["Linked from 1 note".to_owned()],
                k => vec![format!("Linked from {k} notes")],
            },
            archive: None,
        })
    };
    let previews_for = |used: &markdown::Used| -> Vec<Preview> {
        let mut out: Vec<Preview> = used.notes.iter().filter_map(|k| note_preview(k)).collect();
        out.extend(used.links.iter().filter_map(|k| {
            db.links
                .get(k)
                .map(|l| link_preview(l, cited.get(k).map_or(0, HashSet::len)))
        }));
        out
    };

    // Templates.
    let mut tpl = Environment::new();
    tpl.set_loader(path_loader(paths.templates()));
    // Tags with a page (/tag/<tag>/): those of published posts and lists. A tag only
    // drafts use is shown as plain text rather than linking to a page that isn't there.
    let tag_pages: std::collections::BTreeSet<String> = notes
        .iter()
        .filter(|n| n.is_listed() && matches!(n.kind, Kind::Post | Kind::List))
        .flat_map(|n| n.tags.iter().cloned())
        .collect();
    let site = context! {
        dev => opts.dev,
        tag_pages => &tag_pages,
        notes_total => notes.iter().filter(|n| n.is_listed() && n.kind == Kind::Post).count(),
        reading_total => db.links.values().filter(|l| l.saved_at().is_some()).count(),
    };

    if paths.out.exists() {
        std::fs::remove_dir_all(&paths.out)?;
    }
    std::fs::create_dir_all(&paths.out)?;
    let renderer = Renderer::default();

    // Posts, ordered for older/newer.
    let mut posts: Vec<&Note> = notes
        .iter()
        .copied()
        .filter(|n| n.kind == Kind::Post && n.is_listed())
        .collect();
    posts.sort_by_key(|n| n.date);

    // Pass 2: note pages.
    let mut til_entries = vec![];
    for n in &notes {
        let key = n.key();
        let analysis = &analyses[&key];
        let rendered = renderer
            .render(&n.body, &env)
            .with_context(|| format!("rendering {}", n.path.display()))?;

        let mut outs: BTreeMap<String, LinkOut> = BTreeMap::new();
        for l in &analysis.links {
            if let Target::External {
                key: lk,
                mark,
                icon,
                ..
            } = &l.target
            {
                let rec = db.links.get(lk);
                outs.entry(lk.clone())
                    .and_modify(|o| o.times += 1)
                    .or_insert_with(|| LinkOut {
                        mark,
                        preview: rec.map(|r| format!("l:{}", r.id)),
                        icon: *icon,
                        title: rec
                            .and_then(|r| r.title.clone())
                            .unwrap_or_else(|| l.text.clone()),
                        url: l.href.clone(),
                        site: rec.map_or_else(
                            || {
                                crate::links::clean(&l.href)
                                    .map(|u| crate::links::site_of(&u))
                                    .unwrap_or_default()
                            },
                            |r| r.site.clone(),
                        ),
                        times: 1,
                    });
            }
        }
        stats.links_out += outs.len();
        stats.links_known += outs.keys().filter(|k| db.links.contains_key(*k)).count();

        let meta = meta_of(n, Some(analysis));
        if n.kind == Kind::Til {
            if n.is_listed() {
                til_entries.push((meta, rendered.html, previews_for(&rendered.used)));
            }
            continue;
        }
        let pos = posts.iter().position(|p| p.key() == key);
        let older = pos
            .and_then(|i| i.checked_sub(1))
            .map(|i| meta_of(posts[i], None));
        let newer = pos.and_then(|i| posts.get(i + 1)).map(|p| meta_of(p, None));
        let also_tagged: Vec<NoteMeta> = notes
            .iter()
            .filter(|o| {
                o.is_listed() && o.key() != key && o.tags.iter().any(|t| n.tags.contains(t))
            })
            .take(5)
            .map(|o| meta_of(o, None))
            .collect();

        // Cards for every note this page links to, in the body or around it.
        let mut used = rendered.used.clone();
        let back = backlinks.get(&key).cloned().unwrap_or_default();
        used.notes.extend(back.iter().map(|b| b.key.clone()));
        used.notes.extend(
            older
                .iter()
                .chain(newer.iter())
                .chain(also_tagged.iter())
                .map(|m| m.key.clone()),
        );

        let html = tpl.get_template("note.html")?.render(context! {
            site => &site,
            note => &meta,
            body => &rendered.html,
            sidenotes => rendered.sidenotes,
            contents => if analysis.headings.len() >= 2 { analysis.headings.clone() } else { vec![] },
            backlinks => back,
            links_out => outs.into_values().collect::<Vec<_>>(),
            older => older,
            newer => newer,
            also_tagged => also_tagged,
            previews => previews_for(&used),
        })?;
        write_page(&paths.out, &n.url, &html)?;
        stats.notes += 1;
        stats.pages += 1;
    }

    // TIL: one page with every entry.
    til_entries.sort_by(|a, b| b.0.date_iso.cmp(&a.0.date_iso));
    let til_previews: Vec<Preview> = til_entries.iter().flat_map(|e| e.2.clone()).collect();
    let html = tpl.get_template("til.html")?.render(context! {
        site => &site,
        entries => til_entries.iter().map(|(m, h, _)| context! { note => m, body => h }).collect::<Vec<_>>(),
        previews => dedup(til_previews),
    })?;
    write_page(&paths.out, "/til/", &html)?;
    stats.pages += 1;

    // Writing: every listed post by year.
    let post_metas: Vec<NoteMeta> = posts
        .iter()
        .rev()
        .map(|p| meta_of(p, analyses.get(&p.key())))
        .collect();
    let years = years_of(&post_metas);
    let previews_of = |metas: &[NoteMeta]| -> Vec<Preview> {
        dedup(metas.iter().filter_map(|m| note_preview(&m.key)).collect())
    };
    let all_previews = previews_of(&post_metas);

    // Tags: /tags/ and /tag/<tag>/, for published posts (as on the Astro site) and
    // lists (their tags linked nowhere before).
    let list_metas: Vec<NoteMeta> = notes
        .iter()
        .filter(|n| n.kind == Kind::List && n.is_listed())
        .map(|n| meta_of(n, analyses.get(&n.key())))
        .collect();
    let mut by_tag: BTreeMap<String, Vec<NoteMeta>> = BTreeMap::new();
    for m in post_metas.iter().chain(list_metas.iter()) {
        for t in &m.tags {
            by_tag.entry(t.clone()).or_default().push(m.clone());
        }
    }
    let tag_counts: Vec<(String, usize)> =
        by_tag.iter().map(|(t, v)| (t.clone(), v.len())).collect();

    let html = tpl.get_template("writing.html")?.render(context! {
        site => &site, years => &years, previews => &all_previews, tags => &tag_counts,
    })?;
    write_page(&paths.out, "/blog/", &html)?;
    stats.pages += 1;

    let html = tpl
        .get_template("tags.html")?
        .render(context! { site => &site, tags => &tag_counts })?;
    write_page(&paths.out, "/tags/", &html)?;
    stats.pages += 1;
    for (t, metas) in &by_tag {
        // Posts by year; lists (undated) in a section of their own.
        let (lists_here, posts_here): (Vec<NoteMeta>, Vec<NoteMeta>) =
            metas.iter().cloned().partition(|m| m.kind == Kind::List);
        let html = tpl.get_template("writing.html")?.render(context! {
            site => &site, heading => t, crumbs => vec![("/tags/", "tags")], crumb => t,
            years => years_of(&posts_here), lists => &lists_here, previews => previews_of(metas),
        })?;
        write_page(&paths.out, &format!("/tag/{t}/"), &html)?;
        stats.pages += 1;
    }

    // Drafts: built at their own address, listed only here (as on the Astro site).
    let mut drafts: Vec<&Note> = all
        .iter()
        .filter(|n| n.kind == Kind::Post && n.draft && !n.hidden)
        .collect();
    drafts.sort_by_key(|n| std::cmp::Reverse(n.date));
    let draft_metas: Vec<NoteMeta> = drafts
        .iter()
        .map(|n| meta_of(n, analyses.get(&n.key())))
        .collect();
    let html = tpl.get_template("writing.html")?.render(context! {
        site => &site, heading => "Drafts", crumbs => vec![("/blog/", "writing")], crumb => "drafts",
        years => years_of(&draft_metas), previews => previews_of(&draft_metas),
    })?;
    write_page(&paths.out, "/blog/drafts/", &html)?;
    stats.pages += 1;

    // Pages written in engine/content (besides the home page), e.g. /now/.
    for page in pages.iter().filter(|p| p.slug != "home") {
        let rendered = renderer.render(&page.body, &env)?;
        let html = tpl.get_template("page.html")?.render(context! {
            site => &site, title => &page.title, slug => &page.slug, body => &rendered.html,
            previews => dedup(previews_for(&rendered.used)),
        })?;
        write_page(&paths.out, &format!("/{}/", page.slug), &html)?;
        stats.pages += 1;
    }

    // Changelog, from src/content/changelog/changelog.json (as on the Astro site).
    let changelog = paths.root.join("src/content/changelog/changelog.json");
    if changelog.exists() {
        #[derive(serde::Deserialize)]
        struct Change {
            time: String,
            title: String,
        }
        let mut changes: Vec<Change> = serde_json::from_str(&std::fs::read_to_string(&changelog)?)
            .with_context(|| format!("reading {}", changelog.display()))?;
        changes.sort_by(|a, b| b.time.cmp(&a.time));
        let mut previews = vec![];
        let mut by_year: Vec<(String, Vec<minijinja::Value>)> = vec![];
        for c in &changes {
            let r = renderer.render(&c.title, &env)?;
            previews.extend(previews_for(&r.used));
            let html = r.html.trim();
            let html = html
                .strip_prefix("<p>")
                .and_then(|h| h.strip_suffix("</p>"))
                .unwrap_or(html);
            let year = c.time.get(..4).unwrap_or("").to_owned();
            if by_year.last().is_none_or(|(y, _)| *y != year) {
                by_year.push((year, vec![]));
            }
            by_year
                .last_mut()
                .unwrap()
                .1
                .push(context! { date => c.time.get(5..10).unwrap_or(""), html => html });
        }
        let html = tpl.get_template("changelog.html")?.render(context! {
            site => &site,
            years => by_year.into_iter().map(|(y, items)| context! { year => y, items => items }).collect::<Vec<_>>(),
            previews => dedup(previews),
        })?;
        write_page(&paths.out, "/changelog/", &html)?;
        stats.pages += 1;
    }

    // Lists index.
    let lists: Vec<NoteMeta> = notes
        .iter()
        .filter(|n| n.kind == Kind::List && n.is_listed())
        .map(|n| meta_of(n, None))
        .collect();
    let html = tpl
        .get_template("lists.html")?
        .render(context! { site => &site, lists => &lists })?;
    write_page(&paths.out, "/lists/", &html)?;
    stats.pages += 1;

    // Bookmarks: the links database, from the Reader side.
    stats.pages += build_reading(paths, &tpl, &site, &db, &cited)?;

    // Home: an index of every kind of note, with the intro from engine/content/home.md.
    if let Some(home) = pages.iter().find(|p| p.slug == "home") {
        // home.md is copied from the Astro home page; `{{ age }}` is computed there too.
        let age = chrono::Utc::now().year() - 2002;
        let body = tpl.render_str(&home.body, context! { age })?;
        let rendered = renderer.render(&body, &env)?;
        let tils: Vec<NoteMeta> = til_entries.iter().map(|e| e.0.clone()).collect();
        let mut saved: Vec<&Link> = db
            .links
            .values()
            .filter(|l| l.saved_at().is_some())
            .collect();
        saved.sort_by(|a, b| b.saved_at().cmp(&a.saved_at()));
        let reading: Vec<_> = saved
            .iter()
            .take(5)
            .map(|l| {
                context! {
                    title => l.display_title(), url => &l.url, site => &l.site,
                    preview => format!("l:{}", l.id),
                }
            })
            .collect();
        let mut previews = all_previews;
        previews.extend(previews_for(&rendered.used));
        previews.extend(
            lists
                .iter()
                .chain(tils.iter())
                .filter_map(|m| note_preview(&m.key)),
        );
        previews.extend(
            saved
                .iter()
                .take(5)
                .map(|l| link_preview(l, cited.get(&l.key).map_or(0, HashSet::len))),
        );
        let html = tpl.get_template("home.html")?.render(context! {
            site => &site,
            title => &home.title,
            body => &rendered.html,
            years => &years,
            lists => &lists,
            tils => &tils,
            reading => reading,
            previews => dedup(previews),
        })?;
        write_page(&paths.out, "/", &html)?;
        stats.pages += 1;
    }

    // Feed.
    let feed_items: Vec<_> = posts
        .iter()
        .rev()
        .take(30)
        .map(|p| {
            context! {
                title => &p.title,
                url => format!("https://noghartt.dev{}", p.url),
                date => p.date.as_ref().map(|d| d.to_rfc2822()),
                excerpt => analyses.get(&p.key()).map(|a| a.excerpt.clone()),
            }
        })
        .collect();
    let rss = tpl
        .get_template("rss.xml")?
        .render(context! { items => feed_items })?;
    std::fs::write(paths.out.join("rss.xml"), rss)?;

    copy_dir(&paths.public(), &paths.out)?;
    copy_dir(&paths.static_dir(), &paths.out.join("static"))?;
    Ok(stats)
}

fn build_reading(
    paths: &Paths,
    tpl: &Environment,
    site: &minijinja::Value,
    db: &LinkDb,
    cited: &HashMap<String, HashSet<String>>,
) -> Result<usize> {
    /// A site with at least this many bookmarks gets its own page.
    const SITE_PAGE: usize = 3;
    let mut pages = 0;
    let mut saved: Vec<(&Link, DateTime<FixedOffset>)> = db
        .links
        .values()
        .filter_map(|l| {
            let at = DateTime::parse_from_rfc3339(l.saved_at()?).ok()?;
            Some((l, at))
        })
        .collect();
    saved.sort_by_key(|s| std::cmp::Reverse(s.1));

    // Every tag holds its subtags' bookmarks too (`cs` holds `cs/distsys`), and
    // untagged bookmarks go under `without-tags`, as on the Astro site.
    let mut by_tag: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    let mut by_site: HashMap<String, Vec<usize>> = HashMap::new();
    for (i, (l, _)) in saved.iter().enumerate() {
        let tags = if l.tags.is_empty() {
            vec!["without-tags".to_owned()]
        } else {
            l.tags.clone()
        };
        let mut seen = HashSet::new();
        for t in tags {
            // A tag becomes a folder (/bookmarks/tags/<tag>/): parts that could step
            // outside it are dropped.
            let parts: Vec<&str> = t
                .split('/')
                .filter(|p| !p.is_empty() && *p != "." && *p != ".." && !p.contains('\\'))
                .collect();
            for k in 1..=parts.len() {
                let path = parts[..k].join("/");
                if seen.insert(path.clone()) {
                    by_tag.entry(path).or_default().push(i);
                }
            }
        }
        by_site.entry(l.site.clone()).or_default().push(i);
    }
    let site_pages: HashSet<&str> = by_site
        .iter()
        .filter(|(_, v)| v.len() >= SITE_PAGE)
        .map(|(s, _)| s.as_str())
        .collect();

    // Bookmarks by month, newest first. Their cards are in one shared file
    // (/bookmarks/cards.html) that the page loads on the first hover.
    let listing = |items: &[usize]| -> Vec<ReadingMonth> {
        let mut month_totals: BTreeMap<(i32, u32), usize> = BTreeMap::new();
        for &i in items {
            let at = saved[i].1;
            *month_totals.entry((at.year(), at.month())).or_default() += 1;
        }
        let mut months: Vec<ReadingMonth> = vec![];
        for &i in items {
            let (l, at) = saved[i];
            let name = at.format("%B %Y").to_string();
            if months.last().is_none_or(|m| m.name != name) {
                months.push(ReadingMonth {
                    name: name.clone(),
                    total: month_totals[&(at.year(), at.month())],
                    items: vec![],
                    more: 0,
                });
            }
            months.last_mut().unwrap().items.push(ReadingItem {
                day: at.format("%d").to_string(),
                title: l.display_title(),
                url: l.url.clone(),
                site: l.site.clone(),
                site_page: site_pages.contains(l.site.as_str()),
                tags: l.tags.clone(),
                preview: format!("l:{}", l.id),
            });
        }
        months
    };
    let since = |items: &[usize]| {
        items
            .last()
            .map(|&i| saved[i].1.format("%B %Y").to_string())
    };
    let top = |mut v: Vec<(String, usize)>, k: usize| {
        v.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        v.truncate(k);
        v
    };

    // /bookmarks/
    let all: Vec<usize> = (0..saved.len()).collect();
    let months = listing(&all);
    let tags = top(
        by_tag
            .iter()
            .filter(|(t, _)| !t.contains('/') && *t != "without-tags")
            .map(|(t, v)| (t.clone(), v.len()))
            .collect(),
        6,
    );
    let sites = top(
        by_site.iter().map(|(s, v)| (s.clone(), v.len())).collect(),
        5,
    );
    let html = tpl.get_template("reading.html")?.render(context! {
        site => site,
        heading => "Bookmarks",
        crumb => "bookmarks",
        total => saved.len(),
        since => since(&all),
        months => months,
        tags => tags,
        sites => sites,
    })?;
    write_page(&paths.out, "/bookmarks/", &html)?;
    pages += 1;

    // Every bookmark's card, in one file the bookmarks pages load on the first hover:
    // thousands of cards would otherwise more than double each page.
    let cards: Vec<Preview> = saved
        .iter()
        .map(|(l, _)| link_preview(l, cited.get(&l.key).map_or(0, HashSet::len)))
        .collect();
    let html = tpl
        .get_template("cards.html")?
        .render(context! { previews => cards })?;
    std::fs::write(paths.out.join("bookmarks/cards.html"), html)?;

    // /bookmarks/tags/ and a page per tag.
    let counts: BTreeMap<String, usize> =
        by_tag.iter().map(|(t, v)| (t.clone(), v.len())).collect();
    let html = tpl.get_template("bookmark_tags.html")?.render(context! {
        site => site, total => saved.len(), tree => tag_tree(&counts, None),
    })?;
    write_page(&paths.out, "/bookmarks/tags/", &html)?;
    pages += 1;
    for (tag, items) in &by_tag {
        let months = listing(items);
        let mut crumbs = vec![
            ("/bookmarks/".to_owned(), "bookmarks".to_owned()),
            ("/bookmarks/tags/".to_owned(), "tags".to_owned()),
        ];
        let parts: Vec<&str> = tag.split('/').collect();
        for k in 1..parts.len() {
            let path = parts[..k].join("/");
            crumbs.push((format!("/bookmarks/tags/{path}/"), parts[k - 1].to_owned()));
        }
        let html = tpl.get_template("reading.html")?.render(context! {
            site => site,
            heading => tag,
            crumbs => crumbs,
            crumb => parts.last(),
            total => items.len(),
            since => since(items),
            months => months,
            subtags => tag_tree(&counts, Some(tag)),
        })?;
        write_page(&paths.out, &format!("/bookmarks/tags/{tag}/"), &html)?;
        pages += 1;
    }

    // A page per site with a few bookmarks.
    for (s, items) in by_site
        .iter()
        .filter(|(s, _)| site_pages.contains(s.as_str()))
    {
        let months = listing(items);
        let html = tpl.get_template("reading.html")?.render(context! {
            site => site,
            heading => s,
            crumbs => vec![("/bookmarks/", "bookmarks")],
            crumb => s,
            total => items.len(),
            since => since(items),
            months => months,
        })?;
        write_page(&paths.out, &format!("/bookmarks/site/{s}/"), &html)?;
        pages += 1;
    }

    // /bookmarks/rss.xml: every bookmark, newest first, as on the Astro site.
    let items: Vec<_> = saved
        .iter()
        .map(|(l, at)| {
            context! {
                title => l.display_title(),
                url => &l.url,
                date => at.to_rfc2822(),
                excerpt => l.description.as_deref(),
            }
        })
        .collect();
    let rss = tpl.get_template("rss.xml")?.render(context! {
        title => "noghartt | Bookmarks", link => "https://noghartt.dev/bookmarks/", items => items,
    })?;
    std::fs::write(paths.out.join("bookmarks/rss.xml"), rss)?;
    Ok(pages)
}

fn years_of(metas: &[NoteMeta]) -> Vec<minijinja::Value> {
    let mut by_year: BTreeMap<i32, Vec<NoteMeta>> = BTreeMap::new();
    for m in metas {
        by_year
            .entry(m.year.unwrap_or(0))
            .or_default()
            .push(m.clone());
    }
    by_year
        .into_iter()
        .rev()
        .map(|(y, notes)| context! { year => y, notes => notes })
        .collect()
}

fn dedup(previews: Vec<Preview>) -> Vec<Preview> {
    let mut seen = HashSet::new();
    previews
        .into_iter()
        .filter(|p| seen.insert(p.key.clone()))
        .collect()
}

fn write_page(out: &Path, url: &str, html: &str) -> Result<()> {
    let rel = url.split('#').next().unwrap_or("").trim_matches('/');
    // Addresses are built from tags and site names that come from Reader: none may
    // step outside the output folder.
    anyhow::ensure!(
        rel.split('/')
            .all(|seg| seg != ".." && seg != "." && !seg.contains('\\')),
        "refusing to write {url}: it leaves the output folder"
    );
    let dir = out.join(rel);
    std::fs::create_dir_all(&dir)?;
    std::fs::write(dir.join("index.html"), html)?;
    Ok(())
}

fn copy_dir(from: &Path, to: &Path) -> Result<()> {
    if !from.exists() {
        return Ok(());
    }
    for entry in walkdir::WalkDir::new(from) {
        let entry = entry?;
        let rel = entry.path().strip_prefix(from)?;
        let dest = to.join(rel);
        if entry.file_type().is_dir() {
            std::fs::create_dir_all(&dest)?;
        } else {
            std::fs::copy(entry.path(), &dest)?;
        }
    }
    Ok(())
}
