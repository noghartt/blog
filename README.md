# Noghartt's garden

The source of [noghartt.dev](https://noghartt.dev), built by `garden`, a Rust
static site generator in [`engine/`](engine/README.md).

```
content/
  blog/            posts (blog/<slug>/)
  lists/           lists (lists/<slug>/)
  til/             today-I-learned notes (til/)
  pages/           standalone pages: home.md, now.md, any <name>.md -> /<name>/
  changelog.json   /changelog/
data/
  links.jsonl      the links database
  bookmarks.json   Readwise Reader export, updated daily by a workflow
public/            copied into the site as-is (images under assets/)
scripts/           helpers: pnpm create:article | create:til | create:changelog
```

Build with `cargo run --release --manifest-path engine/Cargo.toml -- build`
(or `-- serve` for a live preview), inside `nix develop`.
