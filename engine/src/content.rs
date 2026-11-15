//! Loading notes from `content/**`.

use anyhow::{bail, Context, Result};
use chrono::{DateTime, FixedOffset};
use serde::Serialize;
use serde_yaml::Value;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Post,
    List,
    Til,
    /// Pages that belong to the engine itself, like the home page.
    Page,
}

#[derive(Debug, Clone, Serialize)]
pub struct Note {
    pub kind: Kind,
    pub slug: String,
    pub title: String,
    pub date: Option<DateTime<FixedOffset>>,
    pub tags: Vec<String>,
    pub draft: bool,
    /// Rendered at its URL but never listed (same meaning as in the Astro schema).
    pub hidden: bool,
    pub url: String,
    #[serde(skip)]
    pub body: String,
    #[serde(skip)]
    pub path: PathBuf,
}

impl Note {
    /// Listed on index pages, feeds and backlinks.
    pub fn is_listed(&self) -> bool {
        !self.draft && !self.hidden
    }

    /// A stable key used by wikilinks and previews: `blog/<slug>`, `lists/<slug>`, `til/<slug>`.
    pub fn key(&self) -> String {
        self.url.trim_matches('/').replace("/#", "/")
    }
}

/// Splits `---\nyaml\n---\nbody` into its two halves.
pub fn split_front_matter(text: &str) -> Result<(Value, &str)> {
    let rest = text
        .strip_prefix("---\n")
        .or_else(|| text.strip_prefix("---\r\n"));
    let Some(rest) = rest else {
        return Ok((Value::Null, text));
    };
    let Some(end) = rest.find("\n---") else {
        bail!("front matter is not closed");
    };
    let yaml = &rest[..end];
    let after = &rest[end + 4..];
    let body = after.trim_start_matches(['\r', '\n']);
    let value: Value = serde_yaml::from_str(yaml).context("front matter is not valid YAML")?;
    Ok((value, body))
}

fn str_field(fm: &Value, key: &str) -> Option<String> {
    match fm.get(key)? {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        _ => None,
    }
}

fn bool_field(fm: &Value, key: &str) -> bool {
    fm.get(key).and_then(Value::as_bool).unwrap_or(false)
}

fn tags_field(fm: &Value) -> Vec<String> {
    match fm.get("tags") {
        Some(Value::Sequence(items)) => items
            .iter()
            .filter_map(|v| v.as_str().map(str::to_owned))
            .collect(),
        _ => vec![],
    }
}

fn date_field(fm: &Value, key: &str) -> Option<DateTime<FixedOffset>> {
    let raw = str_field(fm, key)?;
    DateTime::parse_from_rfc3339(raw.trim()).ok().or_else(|| {
        chrono::NaiveDate::parse_from_str(raw.trim(), "%Y-%m-%d")
            .ok()
            .map(|d| d.and_hms_opt(0, 0, 0).unwrap().and_utc().fixed_offset())
    })
}

/// `2025-04-20T02:49:57.614Z-skiplists.md` -> `skiplists`
fn strip_timestamp(stem: &str) -> &str {
    match stem.find("Z-") {
        Some(i) if stem.starts_with(|c: char| c.is_ascii_digit()) => &stem[i + 2..],
        _ => stem,
    }
}

fn load_dir(dir: &Path, kind: Kind) -> Result<Vec<Note>> {
    let mut notes = vec![];
    if !dir.exists() {
        return Ok(notes);
    }
    for entry in WalkDir::new(dir).max_depth(1).sort_by_file_name() {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("md") {
            continue;
        }
        let text = std::fs::read_to_string(path)?;
        let (fm, body) =
            split_front_matter(&text).with_context(|| format!("in {}", path.display()))?;
        let stem = path.file_stem().unwrap().to_string_lossy().to_string();
        let slug = str_field(&fm, "slug").unwrap_or_else(|| strip_timestamp(&stem).to_owned());
        let title = str_field(&fm, "title").unwrap_or_else(|| slug.clone());
        let date = date_field(&fm, "pubDate").or_else(|| date_field(&fm, "createdAt"));
        let url = match kind {
            Kind::Post => format!("/blog/{slug}/"),
            Kind::List => format!("/lists/{slug}/"),
            Kind::Til => format!("/til/#{slug}"),
            Kind::Page => format!("/{slug}/"),
        };
        notes.push(Note {
            kind,
            slug,
            title,
            date,
            tags: tags_field(&fm),
            draft: bool_field(&fm, "draft"),
            hidden: bool_field(&fm, "hidden"),
            url,
            body: body.to_owned(),
            path: path.to_owned(),
        });
    }
    Ok(notes)
}

pub fn load(root: &Path) -> Result<Vec<Note>> {
    let content = root.join("content");
    let mut notes = load_dir(&content.join("blog"), Kind::Post)?;
    notes.extend(load_dir(&content.join("lists"), Kind::List)?);
    notes.extend(load_dir(&content.join("til"), Kind::Til)?);
    Ok(notes)
}

/// Standalone pages (`content/pages/*.md`), e.g. the home page.
pub fn load_pages(root: &Path) -> Result<Vec<Note>> {
    load_dir(&root.join("content/pages"), Kind::Page)
}
