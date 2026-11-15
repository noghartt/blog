//! `garden links fetch`: fills in titles and descriptions for links that lack them.
//! Run on purpose, never during `build`, so builds stay offline and reproducible.

use crate::links::{Kind, Link, LinkDb, Status};
use anyhow::Result;
use reqwest::blocking::Client;
use scraper::{Html, Selector};
use std::time::Duration;

/// A generic client name: no personal or site details are sent to other sites.
const USER_AGENT: &str = "garden-link-fetcher/0.1";
const MAX_BYTES: usize = 2 * 1024 * 1024;

pub fn client() -> Result<Client> {
    Ok(Client::builder()
        .user_agent(USER_AGENT)
        .timeout(Duration::from_secs(15))
        .redirect(reqwest::redirect::Policy::limited(5))
        .build()?)
}

#[derive(Default)]
pub struct Options {
    pub limit: Option<usize>,
    /// Only links that a note cites (skip the thousands of reading-list links).
    pub cited_only: bool,
    pub refresh: bool,
    /// Only links cited by this note (`blog/<slug>` or just `<slug>`).
    pub note: Option<String>,
}

fn cited_by(link: &Link, note: &str) -> bool {
    link.sources.iter().any(|s| match s {
        crate::links::Source::Post { slug, .. } => {
            slug == note || slug.rsplit('/').next() == Some(note)
        }
        _ => false,
    })
}

pub fn run(db: &mut LinkDb, opts: &Options) -> Result<(usize, usize)> {
    let client = client()?;
    let todo: Vec<String> = db
        .links
        .values()
        .filter(|l| opts.refresh || l.fetched_at.is_none())
        .filter(|l| !opts.cited_only || l.post_text().is_some())
        .filter(|l| opts.note.as_deref().is_none_or(|n| cited_by(l, n)))
        .filter(|l| l.kind != Kind::Pdf)
        .map(|l| l.key.clone())
        .take(opts.limit.unwrap_or(usize::MAX))
        .collect();
    let (mut ok, mut failed) = (0, 0);
    for key in todo {
        let link = db.links.get_mut(&key).unwrap();
        let result = match link.kind {
            Kind::Wiki => fetch_wikipedia(&client, link),
            _ => fetch_page(&client, link),
        };
        link.fetched_at = Some(chrono::Utc::now().to_rfc3339());
        match result {
            Ok(()) => {
                ok += 1;
                eprintln!("ok    {}", link.url);
            }
            Err(e) => {
                failed += 1;
                eprintln!("fail  {} ({e})", link.url);
            }
        }
        std::thread::sleep(Duration::from_millis(300));
    }
    Ok((ok, failed))
}

fn set_if_empty(slot: &mut Option<String>, value: Option<String>) {
    if slot.as_deref().is_none_or(|s| s.trim().is_empty()) {
        if let Some(v) = value.map(|v| collapse(&v)).filter(|v| !v.is_empty()) {
            *slot = Some(v);
        }
    }
}

fn collapse(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Wikipedia's REST summary: the lead sentence(s) of the article.
fn fetch_wikipedia(client: &Client, link: &mut Link) -> Result<()> {
    let url = url::Url::parse(&link.url)?;
    let host = url.host_str().unwrap_or("en.wikipedia.org");
    let title = url.path().trim_start_matches("/wiki/");
    let api = format!("https://{host}/api/rest_v1/page/summary/{title}");
    let resp = client.get(api).send()?;
    if !resp.status().is_success() {
        link.status = Status::Dead {
            code: resp.status().as_u16(),
        };
        anyhow::bail!("status {}", resp.status());
    }
    let v: serde_json::Value = resp.json()?;
    let s = |k: &str| v.get(k).and_then(|x| x.as_str()).map(str::to_owned);
    set_if_empty(&mut link.title, s("title"));
    set_if_empty(&mut link.description, s("extract"));
    link.status = Status::Ok;
    // The summary API says nothing about the article page's own headers.
    if let Ok(page) = client.head(&link.url).send() {
        link.frame = Some(page.status().is_success() && frame_allowed(page.headers()));
    }
    Ok(())
}

/// `garden links frames`: learns which sites allow frames by reading only each page's
/// headers, several pages at a time, for every https link not checked yet (or all,
/// with `refresh`). Much faster than `fetch`, which reads whole pages one by one.
/// Returns (allowed, refused, failed).
pub fn frames(db: &mut LinkDb, refresh: bool) -> Result<(usize, usize, usize)> {
    use std::sync::atomic::{AtomicUsize, Ordering};
    const WORKERS: usize = 12;
    let client = client()?;
    let todo: Vec<(String, String)> = db
        .links
        .values()
        .filter(|l| refresh || l.frame.is_none())
        .filter(|l| l.url.starts_with("https://"))
        .map(|l| (l.key.clone(), l.url.clone()))
        .collect();
    let next = AtomicUsize::new(0);
    let done = AtomicUsize::new(0);
    let results = std::sync::Mutex::new(Vec::with_capacity(todo.len()));
    std::thread::scope(|s| {
        for _ in 0..WORKERS {
            s.spawn(|| loop {
                let i = next.fetch_add(1, Ordering::Relaxed);
                let Some((key, url)) = todo.get(i) else { break };
                // The response body is never read: dropping it closes the connection.
                let allowed = client
                    .get(url)
                    .send()
                    .ok()
                    .map(|r| r.status().is_success() && frame_allowed(r.headers()));
                results.lock().unwrap().push((key.clone(), allowed));
                let n = done.fetch_add(1, Ordering::Relaxed) + 1;
                if n % 100 == 0 || n == todo.len() {
                    eprintln!("{n}/{}", todo.len());
                }
            });
        }
    });
    let (mut yes, mut no, mut failed) = (0, 0, 0);
    for (key, allowed) in results.into_inner().unwrap() {
        match allowed {
            Some(a) => {
                if a {
                    yes += 1;
                } else {
                    no += 1;
                }
                if let Some(l) = db.links.get_mut(&key) {
                    l.frame = Some(a);
                }
            }
            None => failed += 1,
        }
    }
    Ok((yes, no, failed))
}

/// Whether a page lets other sites show it in a frame: no `X-Frame-Options`, and no
/// `frame-ancestors` in its Content-Security-Policy unless it allows any site. A
/// page can still refuse from its own scripts; this only reads the headers.
fn frame_allowed(headers: &reqwest::header::HeaderMap) -> bool {
    let values = |name: &str| {
        headers
            .get_all(name)
            .iter()
            .filter_map(|v| v.to_str().ok())
            .map(str::to_ascii_lowercase)
            .collect::<Vec<_>>()
    };
    if values("x-frame-options")
        .iter()
        .any(|v| matches!(v.trim(), "deny" | "sameorigin"))
    {
        return false;
    }
    values("content-security-policy").iter().all(|csp| {
        csp.split(';').all(|d| {
            let mut parts = d.split_whitespace();
            parts.next() != Some("frame-ancestors") || parts.any(|s| s == "*" || s == "https:")
        })
    })
}

fn fetch_page(client: &Client, link: &mut Link) -> Result<()> {
    let resp = client.get(&link.url).send()?;
    let status = resp.status();
    let final_url = resp.url().to_string();
    if !status.is_success() {
        link.status = Status::Dead {
            code: status.as_u16(),
        };
        anyhow::bail!("status {status}");
    }
    link.status = if crate::links::clean(&final_url).map(|u| crate::links::key_of(&u))
        != Some(link.key.clone())
    {
        Status::Redirected { to: final_url }
    } else {
        Status::Ok
    };
    let is_html = resp
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v.contains("html"));
    link.frame = Some(is_html && frame_allowed(resp.headers()));
    if !is_html {
        return Ok(());
    }
    let bytes = resp.bytes()?;
    let text = String::from_utf8_lossy(&bytes[..bytes.len().min(MAX_BYTES)]);
    let doc = Html::parse_document(&text);
    let meta = |names: &[&str]| -> Option<String> {
        for name in names {
            let sel = Selector::parse(&format!("meta[property=\"{name}\"], meta[name=\"{name}\"]"))
                .ok()?;
            if let Some(c) = doc
                .select(&sel)
                .find_map(|e| e.value().attr("content").map(str::to_owned))
                .filter(|c| !c.trim().is_empty())
            {
                return Some(c);
            }
        }
        None
    };
    let title_tag = Selector::parse("title").ok().and_then(|sel| {
        doc.select(&sel)
            .next()
            .map(|t| t.text().collect::<String>())
    });
    set_if_empty(
        &mut link.title,
        meta(&["og:title", "twitter:title"]).or(title_tag),
    );
    set_if_empty(
        &mut link.description,
        meta(&["og:description", "description", "twitter:description"]),
    );
    set_if_empty(&mut link.author, meta(&["author", "article:author"]));
    set_if_empty(
        &mut link.published,
        meta(&[
            "article:published_time",
            "date",
            "citation_publication_date",
        ]),
    );
    Ok(())
}
