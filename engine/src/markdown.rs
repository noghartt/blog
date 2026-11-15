//! Markdown: parse with comrak, then two jobs on the tree.
//!
//! * `analyze` reads a note: every link with the sentence around it (for
//!   backlinks), the first paragraph (for previews) and a word count.
//! * `Renderer::render` writes HTML through a custom formatter that turns
//!   footnotes into sidenotes, resolves `[[wikilinks]]`, and marks every link
//!   with where it goes (a note here, or a record in the links database).
//!
//! Every extension stays plain markdown: Obsidian and GitHub still read it.

use crate::links;
use comrak::html::ChildRendering;
use comrak::nodes::{AstNode, NodeValue};
use comrak::options::Plugins;
use comrak::plugins::syntect::{SyntectAdapter, SyntectAdapterBuilder};
use comrak::{create_formatter, parse_document, Arena, Options};
use std::collections::{BTreeSet, HashMap};
use std::fmt::Write;
use std::sync::Arc;

pub const SITE_HOSTS: &[&str] = &["noghartt.dev", "www.noghartt.dev"];

pub fn options() -> Options<'static> {
    let mut o = Options::default();
    o.extension.strikethrough = true;
    o.extension.table = true;
    o.extension.autolink = true;
    o.extension.tasklist = true;
    o.extension.footnotes = true;
    o.extension.header_id_prefix = Some(String::new());
    o.extension.wikilinks_title_after_pipe = true;
    o.extension.math_dollars = true;
    o.parse.smart = true; // curly quotes and dashes, as Astro's SmartyPants does

    o.render.r#unsafe = true; // notes contain hand-written HTML (images, embeds)
    o
}

// ---------------------------------------------------------------------------
// Resolving links
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct NoteRef {
    pub title: String,
    pub url: String,
}

#[derive(Debug, Clone)]
pub struct LinkRef {
    pub id: String,
}

/// `%20` and friends in a link's file name, back to the characters.
fn percent_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    let hex = |c: u8| (c as char).to_digit(16).map(|d| d as u8);
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() {
            if let (Some(h), Some(l)) = (hex(b[i + 1]), hex(b[i + 2])) {
                out.push(h * 16 + l);
                i += 3;
                continue;
            }
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Everything a page needs to know about the rest of the garden while rendering.
#[derive(Debug, Default)]
pub struct Env {
    /// Note key (`blog/<slug>`) -> note.
    pub notes: HashMap<String, NoteRef>,
    /// `/blog/<slug>` -> note key.
    by_path: HashMap<String, String>,
    /// Bare slug -> note key, for `[[slug]]`.
    by_slug: HashMap<String, String>,
    /// Source file name (`2024-…-2024-in-review.md`) -> note key.
    by_file: HashMap<String, String>,
    /// Links database key -> record id.
    pub links: HashMap<String, LinkRef>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Target {
    /// A note in this garden: (note key, fragment).
    Note(String, Option<String>),
    /// Another site: links database key, its mark, its logo (if a known site), and the
    /// record id if the database has it.
    External {
        key: String,
        mark: &'static str,
        icon: Option<&'static str>,
        id: Option<String>,
    },
    /// Anything else: pages of this site that are not notes, mailto:, #anchors, assets.
    Other,
}

impl Env {
    pub fn add_note(&mut self, key: String, slug: &str, title: String, url: String) {
        let path = url.trim_end_matches('/').to_owned();
        self.by_path.insert(path, key.clone());
        self.by_slug.entry(slug.to_owned()).or_insert(key.clone());
        self.by_slug.insert(key.clone(), key.clone());
        self.notes.insert(key, NoteRef { title, url });
    }

    /// Lets a link to a note's source file (`./2024-…-2024-in-review.md`, which works
    /// on GitHub and in Obsidian) resolve to the note.
    pub fn add_file(&mut self, file_name: &str, key: String) {
        self.by_file.insert(file_name.to_owned(), key);
    }

    pub fn resolve(&self, href: &str) -> Target {
        let href = href.trim();
        if url::Url::parse(href).is_err() {
            let (file, fragment) = match href.split_once('#') {
                Some((p, f)) => (p, Some(f.to_owned())),
                None => (href, None),
            };
            if file.ends_with(".md") {
                let name = file.rsplit('/').next().unwrap_or(file);
                let name = percent_decode(name);
                if let Some(key) = self.by_file.get(&name) {
                    return Target::Note(key.clone(), fragment);
                }
            }
        }
        let local_path = if href.starts_with('/') && !href.starts_with("//") {
            Some(href.to_owned())
        } else if let Ok(u) = url::Url::parse(href) {
            if u.host_str().is_some_and(|h| SITE_HOSTS.contains(&h)) {
                let mut p = u.path().to_owned();
                if let Some(f) = u.fragment() {
                    p.push('#');
                    p.push_str(f);
                }
                Some(p)
            } else if matches!(u.scheme(), "http" | "https") {
                let Some(clean) = links::clean(href) else {
                    return Target::Other;
                };
                let key = links::key_of(&clean);
                return Target::External {
                    mark: links::kind_of(&clean).mark(),
                    icon: links::icon_of(&clean),
                    id: self.links.get(&key).map(|l| l.id.clone()),
                    key,
                };
            } else {
                None
            }
        } else {
            None
        };
        let Some(path) = local_path else {
            return Target::Other;
        };
        let (path, fragment) = match path.split_once('#') {
            Some((p, f)) => (p.to_owned(), Some(f.to_owned())),
            None => (path, None),
        };
        let path = path.split('?').next().unwrap_or("").trim_end_matches('/');
        match self.by_path.get(path) {
            Some(key) => Target::Note(key.clone(), fragment),
            None => Target::Other,
        }
    }

    /// `[[slug]]`, `[[blog/slug]]` or `[[slug#heading]]`.
    pub fn resolve_wiki(&self, target: &str) -> Option<(String, Option<String>)> {
        let (name, fragment) = match target.split_once('#') {
            Some((n, f)) => (n, Some(f.to_owned())),
            None => (target, None),
        };
        let name = name.trim().trim_matches('/');
        self.by_slug.get(name).map(|k| (k.clone(), fragment))
    }

    pub fn note_href(&self, key: &str, fragment: Option<&str>) -> String {
        let url = &self.notes[key].url;
        match fragment {
            Some(f) if !url.contains('#') => format!("{url}#{f}"),
            _ => url.clone(),
        }
    }
}

// ---------------------------------------------------------------------------
// Analysis
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct OutLink {
    pub target: Target,
    pub href: String,
    pub text: String,
    /// The sentence(s) around the link, for "Linked from".
    pub context: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct Heading {
    pub level: u8,
    pub text: String,
    /// The same id comrak gives the heading when rendering.
    pub id: String,
}

#[derive(Debug, Clone, Default)]
pub struct Analysis {
    pub links: Vec<OutLink>,
    pub headings: Vec<Heading>,
    pub excerpt: String,
    pub words: usize,
}

fn text_of<'a>(node: &'a AstNode<'a>) -> String {
    let mut out = String::new();
    for n in node.descendants() {
        match &n.data().value {
            NodeValue::Text(t) => out.push_str(t),
            NodeValue::Code(c) => out.push_str(&c.literal),
            NodeValue::SoftBreak | NodeValue::LineBreak => out.push(' '),
            // A sidenote's text is not part of the sentence it is attached to.
            NodeValue::FootnoteReference(_) => {}
            _ => {}
        }
    }
    out
}

fn block_of<'a>(node: &'a AstNode<'a>) -> Option<&'a AstNode<'a>> {
    let mut cur = node.parent();
    while let Some(n) = cur {
        if matches!(
            n.data().value,
            NodeValue::Paragraph | NodeValue::Heading(_) | NodeValue::TableCell
        ) {
            return Some(n);
        }
        cur = n.parent();
    }
    None
}

/// Cuts `text` to about `max` characters on a word boundary.
pub fn truncate(text: &str, max: usize) -> String {
    let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if text.chars().count() <= max {
        return text;
    }
    let mut out = String::new();
    for word in text.split(' ') {
        if out.chars().count() + word.chars().count() + 1 > max {
            break;
        }
        if !out.is_empty() {
            out.push(' ');
        }
        out.push_str(word);
    }
    out.trim_end_matches([',', ';', ':', '.']).to_owned() + "…"
}

/// A window of about `radius` characters on each side of `needle` inside `text`.
fn window(text: &str, needle: &str, radius: usize) -> String {
    let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
    let Some(at) = text.find(needle) else {
        return truncate(&text, radius * 2);
    };
    let start = text[..at]
        .char_indices()
        .rev()
        .nth(radius)
        .map(|(i, _)| text[i..at].find(' ').map_or(i, |s| i + s + 1))
        .unwrap_or(0);
    let end_from = at + needle.len();
    let end = text[end_from..]
        .char_indices()
        .nth(radius)
        .map(|(i, _)| {
            let abs = end_from + i;
            text[abs..].find(' ').map_or(text.len(), |s| abs + s)
        })
        .unwrap_or(text.len());
    let mut out = String::new();
    if start > 0 {
        out.push('…');
    }
    out.push_str(text[start..end].trim());
    if end < text.len() {
        out.push('…');
    }
    out
}

pub fn analyze(body: &str, env: &Env) -> Analysis {
    let arena = Arena::new();
    let opts = options();
    let root = parse_document(&arena, body, &opts);
    let mut a = Analysis::default();
    let mut anchorizer = comrak::Anchorizer::new();

    for node in root.descendants() {
        if let NodeValue::Heading(h) = &node.data().value {
            let level = h.level;
            let text = text_of(node);
            // Every heading goes through the anchorizer, in order, so duplicate
            // titles get the same `-1`, `-2` suffixes as in the rendered page.
            let id = anchorizer.anchorize(&text);
            if matches!(level, 2 | 3) {
                a.headings.push(Heading { level, text, id });
            }
        }
        let (target, href) = match &node.data().value {
            NodeValue::Link(nl) => (env.resolve(&nl.url), nl.url.clone()),
            NodeValue::WikiLink(wl) => match env.resolve_wiki(&wl.url) {
                Some((k, f)) => (Target::Note(k, f), wl.url.clone()),
                None => continue,
            },
            NodeValue::Text(t) => {
                a.words += t.split_whitespace().count();
                continue;
            }
            _ => continue,
        };
        if target == Target::Other {
            continue;
        }
        let text = text_of(node);
        let context = block_of(node)
            .map(|b| window(&text_of(b), &text, 110))
            .unwrap_or_default();
        a.links.push(OutLink {
            target,
            href,
            text,
            context,
        });
    }

    if let Some(p) = root
        .children()
        .find(|n| matches!(n.data().value, NodeValue::Paragraph) && !text_of(n).trim().is_empty())
    {
        a.excerpt = truncate(&text_of(p), 260);
    }
    a
}

// ---------------------------------------------------------------------------
// Rendering
// ---------------------------------------------------------------------------

/// What a rendered page points at, so the page can ship exactly those previews.
#[derive(Debug, Default, Clone)]
pub struct Used {
    pub notes: BTreeSet<String>,
    pub links: BTreeSet<String>,
}

impl Used {
    fn merge(&mut self, other: Used) {
        self.notes.extend(other.notes);
        self.links.extend(other.links);
    }
}

#[derive(Default)]
pub struct Fmt {
    env: Arc<Env>,
    sidenotes: HashMap<String, String>,
    numbers: HashMap<String, usize>,
    used: Used,
}

impl Fmt {
    fn new(env: Arc<Env>) -> Self {
        Fmt {
            env,
            sidenotes: HashMap::new(),
            numbers: HashMap::new(),
            used: Used::default(),
        }
    }
}

fn attr(out: &mut String, value: &str) {
    for c in value.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '"' => out.push_str("&quot;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            c => out.push(c),
        }
    }
}

/// The opening `<a>` for a link, with its preview and mark attributes.
fn open_link(fmt: &mut Fmt, href: &str, title: &str, target: &Target) -> String {
    let mut s = String::from("<a href=\"");
    match target {
        Target::Note(key, fragment) => {
            attr(&mut s, &fmt.env.note_href(key, fragment.as_deref()));
            s.push_str("\" data-preview=\"n:");
            attr(&mut s, key);
            fmt.used.notes.insert(key.clone());
        }
        Target::External {
            key,
            mark,
            icon,
            id,
        } => {
            attr(&mut s, href);
            s.push_str("\" data-mark=\"");
            attr(&mut s, mark);
            if let Some(icon) = icon {
                s.push_str("\" data-icon=\"");
                attr(&mut s, icon);
            }
            if let Some(id) = id {
                s.push_str("\" data-preview=\"l:");
                attr(&mut s, id);
                fmt.used.links.insert(key.clone());
            }
        }
        Target::Other => {
            attr(&mut s, href);
            if href.trim_start().starts_with("mailto:") {
                s.push_str("\" data-icon=\"email");
            }
        }
    }
    s.push('"');
    if !title.is_empty() {
        s.push_str(" title=\"");
        attr(&mut s, title);
        s.push('"');
    }
    s.push('>');
    s
}

create_formatter!(GardenFormatter<Fmt>, {
    NodeValue::Link(ref nl) => |context, entering| {
        if entering {
            let target = context.user.env.resolve(&nl.url);
            let tag = open_link(&mut context.user, &nl.url, &nl.title, &target);
            context.write_str(&tag)?;
        } else {
            // A link with a mark or logo ends in a word joiner (U+2060): no line break
            // between the text and the mark, nor before the punctuation after it.
            let marked = matches!(context.user.env.resolve(&nl.url), Target::External { .. })
                || nl.url.trim_start().starts_with("mailto:");
            context.write_str(if marked { "\u{2060}</a>" } else { "</a>" })?;
        }
    },
    NodeValue::WikiLink(ref wl) => |context, entering| {
        if entering {
            let tag = match context.user.env.resolve_wiki(&wl.url) {
                Some((key, fragment)) => {
                    open_link(&mut context.user, "", "", &Target::Note(key, fragment))
                }
                None => "<a class=\"wikilink-missing\" title=\"No note with this name yet\">".to_owned(),
            };
            context.write_str(&tag)?;
        } else {
            context.write_str("</a>")?;
        }
    },
    NodeValue::FootnoteReference(ref fr) => |context, entering| {
        if entering {
            let html = context.user.sidenotes.get(&fr.name).cloned().unwrap_or_default();
            let name = fr.name.clone();
            let mut s = String::new();
            if let Some(id) = name.strip_prefix("m-") {
                // `[^m-anything]`: an unnumbered margin note, like Tufte's.
                let _ = write!(
                    s,
                    "<label for=\"mn-{id}\" class=\"margin-toggle\">⊕</label><input type=\"checkbox\" id=\"mn-{id}\" class=\"margin-toggle\"><span class=\"marginnote\">{html}</span>"
                );
            } else if fr.ref_num > 1 {
                let n = context.user.numbers.get(&name).copied().unwrap_or(0);
                let _ = write!(
                    s,
                    "<sup class=\"sidenote-ref\"><a href=\"#sidenote-{n}\" data-sn=\"{n}\">{n}</a></sup>"
                );
            } else {
                // The number in the text links to the note and the note's number links
                // back. On narrow screens the number is instead a label that opens the
                // note inline (CSS shows one or the other). `data-sn` pairs them for the
                // hover highlight in garden.js.
                let next = context.user.numbers.len() + 1;
                let n = *context.user.numbers.entry(name).or_insert(next);
                let _ = write!(
                    s,
                    "<span class=\"sidenote-mark\" id=\"snref-{n}\" data-sn=\"{n}\"><a class=\"sidenote-jump\" href=\"#sidenote-{n}\"><sup>{n}</sup></a><label for=\"sn-{n}\" class=\"margin-toggle sidenote-number\"><sup>{n}</sup></label></span><input type=\"checkbox\" id=\"sn-{n}\" class=\"margin-toggle\"><span class=\"sidenote\" id=\"sidenote-{n}\" data-sn=\"{n}\"><a class=\"sidenote-n\" href=\"#snref-{n}\">{n}</a> {html}</span>"
                );
            }
            context.write_str(&s)?;
        }
        return Ok(ChildRendering::Skip);
    },
});

pub struct Rendered {
    pub html: String,
    pub used: Used,
    pub sidenotes: usize,
}

pub struct Renderer {
    options: Options<'static>,
    syntect: SyntectAdapter,
}

impl Default for Renderer {
    fn default() -> Self {
        Renderer {
            options: options(),
            // CSS classes instead of inline colours, so code follows the black and white theme.
            syntect: SyntectAdapterBuilder::new().css().build(),
        }
    }
}

impl Renderer {
    pub fn render(&self, body: &str, env: &Arc<Env>) -> anyhow::Result<Rendered> {
        let arena = Arena::new();
        let root = parse_document(&arena, body, &self.options);
        let mut plugins = Plugins::default();
        plugins.render.codefence_syntax_highlighter = Some(&self.syntect);

        // `[[slug]]` without its own text shows the note's title.
        for node in root.descendants() {
            let wiki_url = match &node.data().value {
                NodeValue::WikiLink(wl) => wl.url.clone(),
                _ => continue,
            };
            let Some(child) = node.first_child() else {
                continue;
            };
            let title = env
                .resolve_wiki(&wiki_url)
                .map(|(k, _)| env.notes[&k].title.clone());
            if let (Some(title), NodeValue::Text(t)) = (title, &mut child.data_mut().value) {
                if t.as_ref() == wiki_url {
                    *t = title.into();
                }
            }
        }

        // Render each footnote's body first, then remove the definitions, so they
        // only appear in the margin and not again at the bottom of the page.
        let mut used = Used::default();
        let mut sidenotes = HashMap::new();
        let defs: Vec<_> = root
            .descendants()
            .filter(|n| matches!(n.data().value, NodeValue::FootnoteDefinition(_)))
            .collect();
        for def in &defs {
            let name = match &def.data().value {
                NodeValue::FootnoteDefinition(d) => d.name.clone(),
                _ => unreachable!(),
            };
            let mut html = String::new();
            let mut fmt = Fmt::new(env.clone());
            for child in def.children() {
                fmt = GardenFormatter::format_document_with_plugins(
                    child,
                    &self.options,
                    &mut html,
                    &plugins,
                    fmt,
                )
                .map_err(|e| anyhow::anyhow!("rendering a footnote: {e}"))?;
            }
            used.merge(fmt.used);
            let html = html
                .trim()
                .trim_start_matches("<p>")
                .trim_end_matches("</p>")
                .replace("</p>\n<p>", "<br><br>");
            sidenotes.insert(name, html);
        }
        for def in defs {
            def.detach();
        }

        let mut fmt = Fmt::new(env.clone());
        fmt.sidenotes = sidenotes;
        let mut html = String::new();
        let fmt = GardenFormatter::format_document_with_plugins(
            root,
            &self.options,
            &mut html,
            &plugins,
            fmt,
        )
        .map_err(|e| anyhow::anyhow!("rendering: {e}"))?;
        used.merge(fmt.used);
        Ok(Rendered {
            html,
            used,
            sidenotes: fmt.numbers.len(),
        })
    }
}
