//! Share images (`og.png`, 1200×630): one per page, drawn like the site's header
//! — the bordered N, "Noghartt's garden", a ruled line — with the page's own title,
//! description and address under it.
//!
//! A card is laid out here as SVG (lines are wrapped with the fonts' own advance
//! widths, since SVG text doesn't wrap) and rasterized by resvg. The fonts are
//! compiled in (`engine/fonts`, OFL), so a build never looks at the system's fonts
//! and draws the same image on every machine. Rendered images are cached by a hash
//! of everything that goes into them.

use anyhow::{Context, Result};
use resvg::{tiny_skia, usvg};
use skrifa::instance::{LocationRef, Size};
use skrifa::{FontRef, MetadataProvider};
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub const WIDTH: u32 = 1200;
pub const HEIGHT: u32 = 630;
/// Bump when the drawing changes, so cached images are drawn again.
const VERSION: &str = "2";

const SERIF: &[u8] = include_bytes!("../fonts/SourceSerif4-Regular.ttf");
const SERIF_BOLD: &[u8] = include_bytes!("../fonts/SourceSerif4-Semibold.ttf");
const MONO: &[u8] = include_bytes!("../fonts/IBMPlexMono-Regular.ttf");

// The site's colours (garden.css).
const GROUND: &str = "#FDFDFB";
const INK: &str = "#111111";
const MUTED: &str = "#555555";
const RULE: &str = "#CFCFCF";

const SITE_NAME: &str = "Noghartt's garden";
const LEFT: f32 = 80.0;
const RIGHT: f32 = WIDTH as f32 - 80.0;

/// What one page's image shows. Text is plain (not escaped).
#[derive(Debug, Clone)]
pub struct Card {
    /// Where the image goes in the output, e.g. `blog/some-post/og.png`.
    pub file: PathBuf,
    /// The section, in the header's top right ("Writing", "Bookmarks"…).
    pub section: Option<String>,
    pub title: String,
    pub description: String,
    /// The page's address without the scheme, e.g. `noghartt.dev/blog/some-post/`.
    pub address: String,
    /// A note's date, at the bottom right.
    pub meta: Option<String>,
}

impl Card {
    fn hash(&self) -> String {
        let mut h = blake3::Hasher::new();
        for part in [
            VERSION,
            self.section.as_deref().unwrap_or(""),
            &self.title,
            &self.description,
            &self.address,
            self.meta.as_deref().unwrap_or(""),
        ] {
            h.update(part.as_bytes());
            h.update(&[0]);
        }
        h.finalize().to_hex()[..32].to_owned()
    }
}

/// Draws every card into `out` (reusing cached images from `cache`), several at a
/// time.
pub fn render_all(cards: &[Card], out: &Path, cache: &Path) -> Result<()> {
    if cards.is_empty() {
        return Ok(());
    }
    std::fs::create_dir_all(cache)?;
    let painter = Painter::new()?;
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get());
    let chunk = cards.len().div_ceil(threads);
    std::thread::scope(|s| {
        let jobs: Vec<_> = cards
            .chunks(chunk)
            .map(|part| {
                let painter = &painter;
                s.spawn(move || -> Result<()> {
                    for card in part {
                        let cached = cache.join(format!("{}.png", card.hash()));
                        if !cached.exists() {
                            let png = painter
                                .png(card)
                                .with_context(|| format!("drawing {}", card.file.display()))?;
                            // Write then rename, so a stopped build leaves no half image.
                            let tmp = cached.with_extension("tmp");
                            std::fs::write(&tmp, png)?;
                            std::fs::rename(&tmp, &cached)?;
                        }
                        let dest = out.join(&card.file);
                        if let Some(dir) = dest.parent() {
                            std::fs::create_dir_all(dir)?;
                        }
                        std::fs::copy(&cached, &dest)?;
                    }
                    Ok(())
                })
            })
            .collect();
        jobs.into_iter()
            .try_for_each(|j| j.join().expect("og thread panicked"))
    })
}

struct Painter {
    options: usvg::Options<'static>,
    serif: Metrics,
    serif_bold: Metrics,
    mono: Metrics,
}

impl Painter {
    fn new() -> Result<Self> {
        let mut db = usvg::fontdb::Database::new();
        for data in [SERIF, SERIF_BOLD, MONO] {
            db.load_font_data(data.to_vec());
        }
        Ok(Painter {
            options: usvg::Options {
                fontdb: Arc::new(db),
                ..Default::default()
            },
            serif: Metrics::new(SERIF)?,
            serif_bold: Metrics::new(SERIF_BOLD)?,
            mono: Metrics::new(MONO)?,
        })
    }

    fn png(&self, card: &Card) -> Result<Vec<u8>> {
        let svg = self.svg(card);
        let tree = usvg::Tree::from_str(&svg, &self.options)?;
        let mut pixmap = tiny_skia::Pixmap::new(WIDTH, HEIGHT).context("allocating the image")?;
        resvg::render(&tree, tiny_skia::Transform::default(), &mut pixmap.as_mut());
        Ok(pixmap.encode_png()?)
    }

    fn svg(&self, card: &Card) -> String {
        let width = RIGHT - LEFT;
        let mut s = format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{WIDTH}\" height=\"{HEIGHT}\" \
             viewBox=\"0 0 {WIDTH} {HEIGHT}\">\
             <rect width=\"100%\" height=\"100%\" fill=\"{GROUND}\"/>"
        );

        // Header: the mark, the name, the section; a rule under them (as on the site).
        let (top, mark) = (64.0, 60.0);
        s += &format!(
            "<rect x=\"{x}\" y=\"{y}\" width=\"{w}\" height=\"{w}\" fill=\"none\" \
             stroke=\"{INK}\" stroke-width=\"3\"/>",
            x = LEFT + 1.5,
            y = top + 1.5,
            w = mark - 3.0,
        );
        s += &text(
            LEFT + mark / 2.0,
            top + mark / 2.0 + 14.0,
            "serif",
            600,
            40.0,
            INK,
            "middle",
            0.0,
            "N",
        );
        s += &text(
            LEFT + mark + 22.0,
            top + mark / 2.0 + 12.0,
            "serif",
            600,
            34.0,
            INK,
            "start",
            0.0,
            SITE_NAME,
        );
        if let Some(section) = &card.section {
            // The nav's all-small-caps, drawn as small capitals with the same spacing.
            s += &text(
                RIGHT,
                top + mark / 2.0 + 8.0,
                "serif",
                400,
                22.0,
                MUTED,
                "end",
                1.6,
                &section.to_uppercase(),
            );
        }
        let rule = top + mark + 20.0;
        s += &format!(
            "<rect x=\"{LEFT}\" y=\"{rule}\" width=\"{width}\" height=\"2\" fill=\"{INK}\"/>"
        );

        // Footer: the address and the date over a light rule.
        let foot = HEIGHT as f32 - 64.0;
        s += &format!(
            "<rect x=\"{LEFT}\" y=\"{y}\" width=\"{width}\" height=\"1\" fill=\"{RULE}\"/>",
            y = foot - 38.0
        );
        let meta_w = card
            .meta
            .as_deref()
            .map_or(0.0, |m| self.mono.width(m, 22.0) + 40.0);
        let address = self.mono.fit(&card.address, 22.0, width - meta_w);
        s += &text(LEFT, foot, "mono", 400, 22.0, MUTED, "start", 0.0, &address);
        if let Some(meta) = &card.meta {
            s += &text(RIGHT, foot, "mono", 400, 22.0, MUTED, "end", 0.0, meta);
        }

        // Title: the largest size that fits in a few lines.
        let body_top = rule + 72.0;
        let body_bottom = foot - 38.0 - 28.0;
        let (size, lines) = [(78.0, 2), (68.0, 3), (58.0, 3), (50.0, 4)]
            .iter()
            .map(|&(size, max)| (size, self.serif_bold.wrap(&card.title, size, width, max)))
            .find(|(_, (_, cut))| !cut)
            .unwrap_or_else(|| (50.0, self.serif_bold.wrap(&card.title, 50.0, width, 4)));
        let lines = lines.0;
        let lead = size * 1.12;
        let mut y = body_top + size * 0.78;
        for line in &lines {
            s += &text(
                LEFT,
                y,
                "serif",
                600,
                size,
                INK,
                "start",
                -0.01 * size,
                line,
            );
            y += lead;
        }

        // Description: whatever room is left, at most three lines.
        let (dsize, dlead) = (29.0, 29.0 * 1.4);
        let mut y = y - lead + size * 0.22 + 28.0 + dsize;
        let room = ((body_bottom - (y - dsize)) / dlead).floor().max(0.0) as usize;
        let (dlines, _) = self
            .serif
            .wrap(&card.description, dsize, width, room.min(3));
        for line in &dlines {
            s += &text(LEFT, y, "serif", 400, dsize, MUTED, "start", 0.0, line);
            y += dlead;
        }

        s + "</svg>"
    }
}

#[allow(clippy::too_many_arguments)]
fn text(
    x: f32,
    y: f32,
    face: &str,
    weight: u16,
    size: f32,
    fill: &str,
    anchor: &str,
    spacing: f32,
    content: &str,
) -> String {
    let family = match face {
        "mono" => "'IBM Plex Mono'",
        // Quoted: a name with a bare number in it isn't a valid CSS identifier.
        _ => "'Source Serif 4'",
    };
    format!(
        "<text x=\"{x}\" y=\"{y}\" font-family=\"{family}\" font-weight=\"{weight}\" \
         font-size=\"{size}\" fill=\"{fill}\" text-anchor=\"{anchor}\" letter-spacing=\"{spacing}\" \
         xml:space=\"preserve\">{}</text>",
        xml_escape(content)
    )
}

fn xml_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            c if c.is_control() => out.push(' '),
            c => out.push(c),
        }
    }
    out
}

/// Advance widths of one font, to wrap lines the way resvg will draw them
/// (kerning aside, which only ever makes a line a little shorter).
struct Metrics {
    data: &'static [u8],
}

impl Metrics {
    fn new(data: &'static [u8]) -> Result<Self> {
        FontRef::new(data).context("reading a compiled-in font")?;
        Ok(Metrics { data })
    }

    fn width(&self, s: &str, size: f32) -> f32 {
        let font = FontRef::new(self.data).expect("checked in Metrics::new");
        let charmap = font.charmap();
        let glyphs = font.glyph_metrics(Size::new(size), LocationRef::default());
        s.chars()
            .map(|c| {
                let gid = charmap.map(c).unwrap_or_default();
                glyphs.advance_width(gid).unwrap_or(size * 0.5)
            })
            .sum()
    }

    /// `s` cut to `max` wide, ending in "…" when cut.
    fn fit(&self, s: &str, size: f32, max: f32) -> String {
        if self.width(s, size) <= max {
            return s.to_owned();
        }
        let mut out: String = s.to_owned();
        while !out.is_empty() && self.width(&format!("{out}…"), size) > max {
            out.pop();
        }
        format!("{}…", out.trim_end())
    }

    /// Words in lines at most `max` wide, at most `lines` of them; the last ends in
    /// "…" when the text didn't fit. Returns the lines and whether it was cut.
    fn wrap(&self, s: &str, size: f32, max: f32, lines: usize) -> (Vec<String>, bool) {
        let mut out: Vec<String> = vec![];
        if lines == 0 {
            return (out, !s.trim().is_empty());
        }
        let mut words = s.split_whitespace().peekable();
        let mut cut = false;
        while let Some(word) = words.next() {
            let fits = out
                .last()
                .is_some_and(|line| self.width(&format!("{line} {word}"), size) <= max);
            if fits {
                let line = out.last_mut().expect("checked");
                line.push(' ');
                line.push_str(word);
            } else if out.len() == lines {
                cut = true;
                break;
            } else {
                out.push(word.to_owned());
            }
            // A single word wider than the line (a long URL, say) is cut to fit.
            let last = out.last_mut().expect("just pushed");
            if self.width(last, size) > max {
                *last = self.fit(last, size, max);
                if words.peek().is_some() && out.len() == lines {
                    cut = true;
                    break;
                }
            }
        }
        if cut {
            if let Some(last) = out.last_mut() {
                let trimmed = last.trim_end_matches(|c: char| c.is_ascii_punctuation() || c == '…');
                *last = self.fit(&format!("{trimmed}…"), size, max);
            }
        }
        (out, cut)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wraps_and_cuts() {
        let m = Metrics::new(SERIF).unwrap();
        let (lines, cut) = m.wrap("a short title", 60.0, 1000.0, 3);
        assert_eq!(lines, vec!["a short title"]);
        assert!(!cut);
        let long = "word ".repeat(200);
        let (lines, cut) = m.wrap(&long, 60.0, 1000.0, 2);
        assert_eq!(lines.len(), 2);
        assert!(cut);
        assert!(lines[1].ends_with('…'));
        assert!(lines.iter().all(|l| m.width(l, 60.0) <= 1000.0));
    }

    #[test]
    fn draws_a_card() {
        let card = Card {
            file: "og.png".into(),
            section: Some("Writing".into()),
            title: "Notes about logical & vector clocks <and> \"quotes\"".into(),
            description: "Some words about it.".into(),
            address: "noghartt.dev/blog/x/".into(),
            meta: Some("20 July 2024".into()),
        };
        let png = Painter::new().unwrap().png(&card).unwrap();
        assert!(png.starts_with(b"\x89PNG"));
    }
}
