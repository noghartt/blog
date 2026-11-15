//! `garden compare <astro-dist>`: for every listed post and list, checks that the
//! words in Astro's `<article>` also appear in the engine's page, so nothing
//! written is lost in the move. Word order and markup are ignored on purpose.

use crate::content::{self, Kind};
use anyhow::Result;
use scraper::{Html, Selector};
use std::collections::HashMap;
use std::path::Path;

fn words(html: &str, selector: &str) -> HashMap<String, usize> {
    let doc = Html::parse_document(html);
    let sel = Selector::parse(selector).unwrap();
    let skip = Selector::parse("script, style").unwrap();
    let mut out = HashMap::new();
    let Some(root) = doc.select(&sel).next() else {
        return out;
    };
    let skipped: Vec<_> = root.select(&skip).map(|e| e.id()).collect();
    for node in root.descendants() {
        if node.ancestors().any(|a| skipped.contains(&a.id())) {
            continue;
        }
        if let Some(text) = node.value().as_text() {
            for w in text.split(|c: char| !c.is_alphanumeric()) {
                if !w.is_empty() {
                    *out.entry(w.to_lowercase()).or_default() += 1;
                }
            }
        }
    }
    out
}

pub fn run(root: &Path, astro: &Path, engine: &Path) -> Result<()> {
    let notes = content::load(root)?;
    let mut rows = vec![];
    for n in notes
        .iter()
        .filter(|n| n.is_listed() && matches!(n.kind, Kind::Post | Kind::List))
    {
        let rel = n.url.trim_matches('/');
        let a = std::fs::read_to_string(astro.join(rel).join("index.html"));
        let e = std::fs::read_to_string(engine.join(rel).join("index.html"));
        let (Ok(a), Ok(e)) = (a, e) else {
            rows.push((
                n.url.clone(),
                0.0,
                0,
                vec!["page missing on one side".to_owned()],
            ));
            continue;
        };
        let aw = words(&a, "article");
        let ew = words(&e, "article");
        let total: usize = aw.values().sum();
        let mut missing: Vec<(String, usize)> = aw
            .iter()
            .filter_map(|(w, c)| {
                let have = ew.get(w).copied().unwrap_or(0);
                (*c > have).then(|| (w.clone(), c - have))
            })
            .collect();
        missing.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        let lost: usize = missing.iter().map(|m| m.1).sum();
        let coverage = if total == 0 {
            1.0
        } else {
            1.0 - lost as f64 / total as f64
        };
        rows.push((
            n.url.clone(),
            coverage,
            total,
            missing
                .iter()
                .take(8)
                .map(|(w, c)| format!("{w}×{c}"))
                .collect(),
        ));
    }
    rows.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
    let mean = rows.iter().map(|r| r.1).sum::<f64>() / rows.len().max(1) as f64;
    println!(
        "{} pages compared, mean word coverage {:.2}%",
        rows.len(),
        mean * 100.0
    );
    println!("lowest:");
    for (url, cov, total, missing) in rows.iter().take(10) {
        println!(
            "  {:6.2}%  {url} ({total} words)  missing: {}",
            cov * 100.0,
            missing.join(" ")
        );
    }
    Ok(())
}
