//! `garden serve`: build, serve `dist/`, rebuild when content, templates or the
//! links database change. Pages built with `dev` poll `/__garden/version` and
//! reload themselves after a rebuild.

use crate::site::{self, BuildOptions, Paths};
use anyhow::Result;
use axum::http::{header, HeaderValue};
use axum::{routing::get, Router};
use notify::{RecursiveMode, Watcher};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tower_http::{services::ServeDir, set_header::SetResponseHeaderLayer};

fn rebuild(paths: &Paths, opts: BuildOptions) -> bool {
    let started = Instant::now();
    match site::build(paths, opts) {
        Ok(stats) => {
            eprintln!(
                "built {} pages in {} ms",
                stats.pages,
                started.elapsed().as_millis()
            );
            true
        }
        Err(e) => {
            eprintln!("build failed: {e:#}");
            false
        }
    }
}

pub fn run(paths: Paths, port: u16, drafts: bool) -> Result<()> {
    let opts = BuildOptions { drafts, dev: true };
    rebuild(&paths, opts);
    let version = Arc::new(AtomicU64::new(1));

    // Watcher thread: debounce bursts of events (editors write several times).
    let (tx, rx) = std::sync::mpsc::channel();
    let mut watcher = notify::recommended_watcher(tx)?;
    for dir in paths.watched() {
        if dir.exists() {
            watcher.watch(&dir, RecursiveMode::Recursive)?;
        }
    }
    {
        let version = version.clone();
        let paths = Paths::new(paths.root.clone(), Some(paths.out.clone()));
        std::thread::spawn(move || {
            // The build reads every watched file, which emits access events: only
            // creations, writes and removals count as changes.
            let is_change = |e: &notify::Result<notify::Event>| {
                e.as_ref().is_ok_and(|e| {
                    matches!(
                        e.kind,
                        notify::EventKind::Create(_)
                            | notify::EventKind::Modify(_)
                            | notify::EventKind::Remove(_)
                    )
                })
            };
            while let Ok(first) = rx.recv() {
                if !is_change(&first) {
                    continue;
                }
                while rx.recv_timeout(Duration::from_millis(150)).is_ok() {}
                if rebuild(&paths, opts) {
                    version.fetch_add(1, Ordering::SeqCst);
                }
            }
        });
    }

    let out = paths.out.clone();
    tokio::runtime::Runtime::new()?.block_on(async move {
        let v = version.clone();
        let app = Router::new()
            .route(
                "/__garden/version",
                get(move || {
                    let v = v.clone();
                    async move { v.load(Ordering::SeqCst).to_string() }
                }),
            )
            .fallback_service(ServeDir::new(out))
            // Without this, browsers guess a cache lifetime from Last-Modified and
            // keep serving an old garden.css after a rebuild.
            .layer(SetResponseHeaderLayer::overriding(
                header::CACHE_CONTROL,
                HeaderValue::from_static("no-cache"),
            ));
        let listener = tokio::net::TcpListener::bind(("127.0.0.1", port)).await?;
        eprintln!("serving on http://localhost:{port}");
        axum::serve(listener, app).await?;
        // Keep the watcher alive for as long as the server runs.
        drop(watcher);
        anyhow::Ok(())
    })
}
