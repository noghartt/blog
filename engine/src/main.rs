mod content;
mod fetch;
mod links;
mod markdown;
mod og;
mod serve;
mod site;

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use links::{LinkDb, Source};
use site::{BuildOptions, Paths};
use std::collections::HashSet;
use std::path::PathBuf;
use std::time::Instant;

/// A static site generator for noghartt.dev, built around a database of links.
#[derive(Parser)]
#[command(name = "garden", version)]
struct Cli {
    /// Repository root (the folder with content/). Found automatically by default.
    #[arg(long, global = true)]
    root: Option<PathBuf>,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Build the site into engine/dist (or --out).
    Build {
        #[arg(long)]
        out: Option<PathBuf>,
        /// Also render notes marked `draft: true`.
        #[arg(long)]
        drafts: bool,
    },
    /// Build, serve on localhost and rebuild on every change.
    Serve {
        #[arg(long, default_value_t = 4321)]
        port: u16,
        #[arg(long)]
        drafts: bool,
    },
    /// Work with the links database (data/links.jsonl).
    Links {
        #[command(subcommand)]
        command: LinksCommand,
    },
}

#[derive(Subcommand)]
enum LinksCommand {
    /// Record every outside link found in the notes (adds `post` sources).
    Scan,
    /// Import the Reader export the daily workflow writes (data/bookmarks.json).
    ImportReaderJson { path: Option<PathBuf> },
    /// Pull saved documents straight from Readwise Reader (needs READWISE_READER_API_KEY).
    SyncReader,
    /// Import `archivebox list --json` output.
    ImportArchivebox { path: PathBuf },
    /// Fetch titles and descriptions for links that have none yet.
    Fetch {
        #[arg(long)]
        limit: Option<usize>,
        /// Only links cited in a note, not the whole reading list.
        #[arg(long)]
        cited_only: bool,
        /// Fetch again even if fetched before.
        #[arg(long)]
        refresh: bool,
        /// Only links cited by one note, e.g. `on-sidenotes-implementation`.
        #[arg(long)]
        note: Option<String>,
    },
    /// Learn which sites allow being shown in a popup's frame (reads headers only).
    Frames {
        /// Check again links checked before.
        #[arg(long)]
        refresh: bool,
    },
    /// Counts by source and kind.
    Stats,
}

fn find_root() -> Result<PathBuf> {
    let mut dir = std::env::current_dir()?;
    loop {
        if dir.join("content").is_dir() && dir.join("engine").is_dir() {
            return Ok(dir);
        }
        if !dir.pop() {
            bail!("could not find the repository root (a folder with content/ and engine/)");
        }
    }
}

fn scan(paths: &Paths, db: &mut LinkDb) -> Result<usize> {
    let notes = content::load(&paths.root)?;
    let mut env = markdown::Env::default();
    for n in &notes {
        env.add_note(n.key(), &n.slug, n.title.clone(), n.url.clone());
        if let Some(f) = n.path.file_name() {
            env.add_file(&f.to_string_lossy(), n.key());
        }
    }
    let mut live = HashSet::new();
    for n in notes.iter().filter(|n| !n.draft) {
        for l in markdown::analyze(&n.body, &env).links {
            if let markdown::Target::External { key, .. } = &l.target {
                db.add(
                    &l.href,
                    Source::Post {
                        slug: n.key(),
                        text: l.text.clone(),
                    },
                );
                live.insert((key.clone(), n.key()));
            }
        }
    }
    db.retain_posts(&live);
    Ok(live.len())
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let root = match cli.root {
        Some(r) => r,
        None => find_root()?,
    };
    match cli.command {
        Command::Build { out, drafts } => {
            let paths = Paths::new(root, out);
            let started = Instant::now();
            let stats = site::build(&paths, BuildOptions { drafts, dev: false })?;
            eprintln!(
                "built {} pages ({} notes) in {} ms into {}\n  {} backlinks, {} outside links ({} in the links database)",
                stats.pages,
                stats.notes,
                started.elapsed().as_millis(),
                paths.out.display(),
                stats.backlinks,
                stats.links_out,
                stats.links_known,
            );
        }
        Command::Serve { port, drafts } => serve::run(Paths::new(root, None), port, drafts)?,
        Command::Links { command } => {
            let paths = Paths::new(root, None);
            let mut db = LinkDb::load(&paths.db())?;
            let before = db.links.len();
            match command {
                LinksCommand::Scan => {
                    let n = scan(&paths, &mut db)?;
                    eprintln!("{n} links cited in notes");
                }
                LinksCommand::ImportReaderJson { path } => {
                    let path = path.unwrap_or_else(|| paths.bookmarks_json());
                    let n = links::import_reader_json(&mut db, &path)?;
                    eprintln!("{n} bookmarks imported from {}", path.display());
                }
                LinksCommand::SyncReader => {
                    let token = std::env::var("READWISE_READER_API_KEY")
                        .context("READWISE_READER_API_KEY is not set")?;
                    let n = links::sync_reader(&mut db, &token)?;
                    eprintln!("{n} documents synced from Reader");
                }
                LinksCommand::ImportArchivebox { path } => {
                    let n = links::import_archivebox(&mut db, &path)?;
                    eprintln!("{n} snapshots imported from {}", path.display());
                }
                LinksCommand::Fetch {
                    limit,
                    cited_only,
                    refresh,
                    note,
                } => {
                    let (ok, failed) = fetch::run(
                        &mut db,
                        &fetch::Options {
                            limit,
                            cited_only,
                            refresh,
                            note,
                        },
                    )?;
                    eprintln!("fetched {ok}, failed {failed}");
                }
                LinksCommand::Frames { refresh } => {
                    let (yes, no, failed) = fetch::frames(&mut db, refresh)?;
                    eprintln!("frames allowed {yes}, refused {no}, unreachable {failed}");
                }
                LinksCommand::Stats => {
                    print_stats(&db);
                    return Ok(());
                }
            }
            db.save(&paths.db())?;
            eprintln!(
                "{} links in {} ({:+})",
                db.links.len(),
                paths.db().display(),
                db.links.len() as i64 - before as i64
            );
        }
    }
    Ok(())
}

fn print_stats(db: &LinkDb) {
    use std::collections::BTreeMap;
    let mut by_source: BTreeMap<&str, usize> = BTreeMap::new();
    let mut by_kind: BTreeMap<String, usize> = BTreeMap::new();
    let mut fetched = 0;
    for l in db.links.values() {
        for s in &l.sources {
            let name = match s {
                Source::Reader { .. } => "reader",
                Source::Archivebox { .. } => "archivebox",
                Source::Post { .. } => "post",
            };
            *by_source.entry(name).or_default() += 1;
        }
        *by_kind
            .entry(format!("{:?} {}", l.kind, l.kind.mark()))
            .or_default() += 1;
        if l.fetched_at.is_some() {
            fetched += 1;
        }
    }
    println!("{} links, {fetched} fetched", db.links.len());
    println!("by source: {by_source:?}");
    println!("by kind:   {by_kind:?}");
}
