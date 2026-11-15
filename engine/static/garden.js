// Popups. Hovering (or tapping) a link opens a small window over the page.
//
// Every page ships a <template> per link it can preview (built with the site), so a
// popup shows at once. For a note, the popup then fetches that note's page and swaps
// the excerpt for the whole text, which scrolls inside the popup. Links inside a popup
// open their own popups (the fetched page's templates are added to ours), so you can
// follow a chain without leaving the page.
//
// A link to an outside page whose site allows frames shows the page itself; those
// frames load ahead, once this page has loaded, so they open ready.
//
// The title bar: close, maximize, collapse, pin. A pinned popup stays open until
// closed; dragging the title bar or resizing from the corner grip pins it too. A
// popup opened from a maximized one opens maximized. Esc closes the top popup.
//
// Links work as plain links without this script; only the popups need it.
// Sidenotes: hovering a note or its number in the text lights up both, so you can see
// where in the text a note belongs (and which note a number opens).
(() => {
  let lit = [];
  document.addEventListener('mouseover', (e) => {
    const el = e.target.closest && e.target.closest('[data-sn]');
    if (el && lit.includes(el)) return;
    lit.forEach((x) => x.classList.remove('is-paired'));
    lit = [];
    if (!el) return;
    const root = el.closest('.prose') || document;
    lit = [...root.querySelectorAll(`[data-sn="${el.dataset.sn}"]`)];
    lit.forEach((x) => x.classList.add('is-paired'));
  });
})();

(() => {
  const OPEN_DELAY = 300;
  const WARM_DELAY = 80; // when another popup is open or just closed
  const CLOSE_DELAY = 250;
  const EDGE = 8; // keep popups this far from the window's edges
  const GAP = 8; // between a link and its popup
  const SHEET = 720; // below this width popups are sheets at the bottom of the screen
  const templates = new Map();
  const addTemplates = (root) => {
    root.querySelectorAll('#previews template').forEach((t) => {
      if (!templates.has(t.dataset.previewFor)) templates.set(t.dataset.previewFor, t);
    });
  };
  addTemplates(document);

  // Fetched note pages, by path, so each is fetched once.
  const pages = new Map();
  const fetchPage = (path) => {
    if (!pages.has(path)) {
      pages.set(
        path,
        fetch(path)
          .then((r) => (r.ok ? r.text() : null))
          .then((html) => html && new DOMParser().parseFromString(html, 'text/html'))
          .catch(() => null),
      );
    }
    return pages.get(path);
  };

  // Pages with thousands of links (the bookmarks) keep their cards in one shared
  // file, loaded the first time one of those links is hovered.
  // Any page can need them: a bookmarks page shown inside a popup has none of its own.
  const cardsUrl = document.body.dataset.cards || '/bookmarks/cards.html';
  let cardsLoaded = null;
  const needsCards = (a) =>
    cardsUrl && (a.dataset.preview || '').startsWith('l:') && !templates.has(a.dataset.preview);
  const loadCards = () =>
    cardsLoaded || (cardsLoaded = fetchPage(cardsUrl).then((doc) => doc && addTemplates(doc)));
  async function openLazy(a) {
    if (needsCards(a)) await loadCards();
    open(a);
  }

  const popups = []; // open popups: { el, body, link, parent, pinned }
  const byLink = new WeakMap();
  let z = 10;
  let over = null; // the element under the pointer
  let openTimer = 0;
  let sweepTimer = 0;
  let lastClose = 0;

  const clamp = (v, lo, hi) => Math.max(lo, Math.min(v, hi));
  const popOf = (node) => {
    const el = node && node.closest && node.closest('.pop');
    return el ? popups.find((p) => p.el === el) : null;
  };
  const chain = (p) => {
    const out = [];
    for (; p; p = p.parent) out.push(p);
    return out;
  };

  const ICONS = {
    close: '<path d="M4 4l8 8M12 4l-8 8"/>',
    max: '<rect x="3" y="3" width="10" height="10"/>',
    collapse: '<path d="M3 12.5h10"/>',
    pin: '<path d="M5.5 2.5h5l-1 4 2.5 2.5h-8L6.5 6.5zM8 9v4.5"/>',
  };
  const button = (act, label) =>
    `<button type="button" data-act="${act}" aria-label="${label}" title="${label}">` +
    `<svg viewBox="0 0 16 16" width="14" height="14" fill="none" stroke="currentColor" stroke-width="1.5">${ICONS[act]}</svg></button>`;

  function shell() {
    const el = document.createElement('div');
    el.className = 'pop';
    el.setAttribute('role', 'dialog');
    el.innerHTML =
      `<div class="pop-bar">${button('close', 'Close')}${button('max', 'Maximize')}` +
      `${button('collapse', 'Collapse')}${button('pin', 'Pin')}<a class="pop-title" draggable="false"></a></div>` +
      '<div class="pop-body"></div><div class="pop-grip" title="Resize" aria-hidden="true"></div>';
    return el;
  }

  // A section: its heading and everything after it up to the next heading of the same
  // level or higher.
  function sectionOf(h) {
    const level = +h.tagName[1];
    const out = document.createElement('div');
    out.className = 'prose';
    out.append(h.cloneNode(true));
    for (let n = h.nextElementSibling; n; n = n.nextElementSibling) {
      if (/^H[1-6]$/.test(n.tagName) && +n.tagName[1] <= level) break;
      out.append(n.cloneNode(true));
    }
    return out;
  }

  // The heading a `#…` link points to: on this page, or in the note shown in the
  // popup the link is in.
  // What a `#…` link points to, on this page or in the note shown in the popup the
  // link is in: a heading (its section is shown), or the note or paragraph holding
  // the target (a sidenote's number, the way back from a sidenote).
  function headingFor(a) {
    const href = a.getAttribute('href') || '';
    if (href.length < 2 || !href.startsWith('#') || a.classList.contains('anchor')) return null;
    const p = popOf(a);
    let h = null;
    try {
      h = (p ? p.body : document).querySelector(`#${CSS.escape(decodeURIComponent(href.slice(1)))}`);
    } catch {}
    // A permalink (a link inside what it points to) would preview what is under it.
    if (!h || h.contains(a)) return null;
    if (/^H[2-4]$/.test(h.tagName)) return h;
    return h.closest('.sidenote, .marginnote, li, p, blockquote, pre, table, figure') || h;
  }

  // Every link gets a popup of some kind. Besides the cards built with the site:
  // `s:` a part of this page, `p:` another page of this site, `f:` one of its feeds,
  // `u:` any other address (a small card, or GitHub's view of it).
  function markLinks(root) {
    root.querySelectorAll('a[href]:not([data-preview])').forEach((a) => {
      const href = a.getAttribute('href');
      if (a.closest('.pop-bar')) return;
      if (href.startsWith('#')) {
        if (headingFor(a)) a.dataset.preview = `s:${href.slice(1)}`;
        return;
      }
      // An email link opens the mail app; a popup would only repeat its address.
      if (href.startsWith('mailto:')) return;
      let u;
      try {
        u = new URL(href, location.href);
      } catch {
        return;
      }
      if (u.origin === location.origin) {
        if (u.pathname.endsWith('.xml')) a.dataset.preview = `f:${u.pathname}`;
        else if (!/\.[a-z0-9]+$/i.test(u.pathname)) a.dataset.preview = `p:${u.pathname}`;
      } else if (u.protocol === 'https:' || u.protocol === 'http:') {
        a.dataset.preview = `u:${u.href}`;
      }
    });
  }
  markLinks(document);

  // An address with no card from the site: what can be read from the link itself.
  function buildOutside(a) {
    const el = shell();
    const title = el.querySelector('.pop-title');
    title.textContent = new URL(a.href).hostname;
    title.href = a.href;
    const card = document.createElement('div');
    card.className = 'card';
    const name = document.createElement('span');
    name.className = 'card-title';
    name.textContent = a.textContent.trim() || a.href;
    const addr = document.createElement('span');
    addr.className = 'card-meta mono';
    addr.textContent = a.href;
    card.append(name, addr);
    el.querySelector('.pop-body').append(card);
    el.setAttribute('aria-label', name.textContent);
    return el;
  }

  // One of this site's feeds: its latest items.
  function buildFeed(a) {
    const el = buildPage(a);
    el.classList.add('is-feed');
    return el;
  }
  async function fillFeed(p) {
    let xml;
    try {
      const r = await fetch(new URL(p.link.href).pathname);
      xml = new DOMParser().parseFromString(await r.text(), 'application/xml');
    } catch {
      return;
    }
    if (!popups.includes(p)) return;
    const box = document.createElement('div');
    box.className = 'pop-main';
    const h = document.createElement('h1');
    h.textContent = xml.querySelector('channel > title')?.textContent || '';
    box.append(h);
    for (const item of [...xml.querySelectorAll('item')].slice(0, 100)) {
      const row = document.createElement('div');
      row.className = 'dated';
      const date = document.createElement('span');
      date.className = 'mono';
      const when = new Date(item.querySelector('pubDate')?.textContent || '');
      date.textContent = Number.isNaN(+when) ? '' : when.toISOString().slice(0, 10);
      const link = document.createElement('a');
      link.href = safeHref(item.querySelector('link')?.textContent || '') || '#';
      link.textContent = item.querySelector('title')?.textContent || link.href;
      row.append(date, link);
      box.append(row);
    }
    p.body.querySelector('.card-text')?.replaceWith(box);
    markLinks(p.body);
  }

  function buildSection(a) {
    const h = headingFor(a);
    if (!h) return null;
    const el = shell();
    const p = popOf(a);
    const page = p ? new URL(p.el.querySelector('.pop-title').href).pathname : location.pathname;
    const title = el.querySelector('.pop-title');
    title.textContent = `${page}${a.getAttribute('href')}`;
    title.href = `${page}${a.getAttribute('href')}`;
    const card = document.createElement('div');
    card.className = 'card';
    if (/^H[2-4]$/.test(h.tagName)) card.append(sectionOf(h));
    else {
      const block = document.createElement('div');
      block.className = 'prose';
      block.append(h.cloneNode(true));
      card.append(block);
    }
    el.querySelector('.pop-body').append(card);
    el.setAttribute('aria-label', h.textContent.trim().slice(0, 80));
    return el;
  }

  // A page of this site from the header (Writing, Bookmarks, Now…): filled with the
  // page itself once fetched.
  function buildPage(a) {
    const el = shell();
    const path = new URL(a.href).pathname;
    const title = el.querySelector('.pop-title');
    title.textContent = path;
    title.href = path;
    const card = document.createElement('div');
    card.className = 'card';
    card.append(Object.assign(document.createElement('span'), { className: 'card-text' }));
    el.querySelector('.pop-body').append(card);
    el.setAttribute('aria-label', a.textContent.trim());
    return el;
  }

  // A popup's window. An outside page whose site allows frames (the links database
  // records this when `garden links fetch` runs) is shown in a frame; anything else
  // shows its card.
  function build(key) {
    const t = templates.get(key);
    const el = shell();
    const body = el.querySelector('.pop-body');
    body.appendChild(t.content.cloneNode(true));

    // The card's path (and its mark) goes in the title bar, as a link to the page.
    const title = el.querySelector('.pop-title');
    const path = body.querySelector('.card-path');
    if (path) title.append(...path.childNodes), path.remove();
    const heading = body.querySelector('.card-title');
    el.setAttribute('aria-label', heading ? heading.textContent : '');

    if (t.dataset.frame) {
      const frame = document.createElement('iframe');
      frame.className = 'pop-frame';
      frame.addEventListener('load', () => (frame.dataset.loaded = '1'), { once: true });
      frame.src = t.dataset.frame;
      frame.title = heading ? heading.textContent : t.dataset.frame;
      // Scripts run (in the site's own origin), but the page can't navigate this tab,
      // submit forms (no fake logins in a preview) or use the camera, microphone or
      // location (nothing is delegated with `allow`). Links open in a new tab.
      frame.setAttribute('sandbox', 'allow-scripts allow-same-origin allow-popups allow-popups-to-escape-sandbox');
      frame.referrerPolicy = 'strict-origin-when-cross-origin';
      body.replaceChildren(frame);
      el.classList.add('is-frame');
    }
    return el;
  }

  // GitHub refuses frames, but its API and its raw files can be read from any site,
  // so a GitHub link shows what it points at: a file's code (with the linked lines
  // marked), a commit's, pull request's or comparison's diff, a repository's README,
  // an issue, a gist. Until that arrives, or if GitHub says no (rate limit, private
  // repository), the popup shows the card.
  const GH = 'https://api.github.com';
  const get = (url, accept) =>
    fetch(url, accept ? { headers: { Accept: accept } } : {}).then((r) =>
      r.ok ? r : Promise.reject(new Error(`${r.status}`)),
    );
  const ghJSON = (path, accept = 'application/vnd.github+json') => get(GH + path, accept).then((r) => r.json());
  const ghHTML = 'application/vnd.github.html+json';

  function github(href) {
    let u;
    try {
      u = new URL(href);
    } catch {
      return null;
    }
    const parts = u.pathname.split('/').filter(Boolean);
    if (u.hostname === 'gist.github.com' && parts.length) {
      const id = parts[parts.length - 1];
      return { label: `gist ${id.slice(0, 8)}`, load: () => ghJSON(`/gists/${id}`).then(gistView) };
    }
    if (u.hostname !== 'github.com' && u.hostname !== 'www.github.com') return null;
    const [o, r, kind, ...rest] = parts;
    if (!o || !r) return null;
    const repo = `${o}/${r}`;
    const web = `https://github.com/${repo}/`;
    if (!kind) {
      return {
        label: repo,
        load: () =>
          Promise.all([
            ghJSON(`/repos/${repo}`),
            ghJSON(`/repos/${repo}/contents/`).catch(() => []),
            get(`${GH}/repos/${repo}/readme`, 'application/vnd.github.html')
              .then((x) => x.text())
              .catch(() => ''),
          ]).then(([info, files, readme]) => repoView(info, files, readme, `${web}blob/HEAD/`)),
      };
    }
    if (kind === 'tree' && rest.length >= 1) {
      const [ref, ...path] = rest;
      return {
        label: `${repo}/${decodeURIComponent(path.join('/'))}`,
        load: () => ghJSON(`/repos/${repo}/contents/${path.join('/')}?ref=${encodeURIComponent(ref)}`).then(filesView),
      };
    }
    if (kind === 'blob' && rest.length >= 2) {
      const [ref, ...path] = rest;
      const m = u.hash.match(/^#L(\d+)(?:-L(\d+))?$/);
      const lines = m ? [+m[1], +(m[2] || m[1])] : null;
      const name = decodeURIComponent(path.join('/'));
      return {
        label: `${repo}/${name}`,
        load: () => get(`https://raw.githubusercontent.com/${repo}/${ref}/${path.join('/')}`).then((x) => x.text()).then((t) => codeView(t, lines)),
      };
    }
    if (kind === 'commit' && rest[0]) {
      return {
        label: `${repo}@${rest[0].slice(0, 7)}`,
        load: () => ghJSON(`/repos/${repo}/commits/${rest[0]}`).then((c) => diffView(c.files, c.commit.message.split('\n')[0])),
      };
    }
    if (kind === 'pull' && rest[0]) {
      return {
        label: `${repo}#${rest[0]}`,
        load: () =>
          Promise.all([ghJSON(`/repos/${repo}/pulls/${rest[0]}`), ghJSON(`/repos/${repo}/pulls/${rest[0]}/files?per_page=100`)]).then(
            ([pr, files]) => diffView(files, pr.title),
          ),
      };
    }
    if (kind === 'compare' && rest.length) {
      return { label: `${repo} ${rest.join('/')}`, load: () => ghJSON(`/repos/${repo}/compare/${rest.join('/')}`).then((c) => diffView(c.files)) };
    }
    if (kind === 'issues' && rest[0]) {
      return {
        label: `${repo}#${rest[0]}`,
        load: () => ghJSON(`/repos/${repo}/issues/${rest[0]}`, ghHTML).then((i) => htmlView(`<h1>${esc(i.title)}</h1>${i.body_html || ''}`, web)),
      };
    }
    return null;
  }

  // Addresses from outside data (GitHub's API, a feed) become links only if they are
  // http(s): a `javascript:` address would run in this site if clicked.
  const safeHref = (url) => {
    try {
      const u = new URL(url, location.href);
      return u.protocol === 'https:' || u.protocol === 'http:' ? u.href : null;
    } catch {
      return null;
    }
  };

  const esc = (s) => s.replace(/[&<>"]/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;' })[c]);

  function codeView(text, lines) {
    const pre = document.createElement('pre');
    pre.className = 'pop-code';
    text
      .replace(/\n$/, '')
      .split('\n')
      .slice(0, 3000)
      .forEach((line, i) => {
        const row = document.createElement('span');
        row.className = 'ln';
        row.dataset.n = i + 1;
        row.textContent = `${line}\n`;
        if (lines && i + 1 >= lines[0] && i + 1 <= lines[1]) row.classList.add('is-hl');
        pre.append(row);
      });
    return pre;
  }

  function diffView(files, title) {
    const box = document.createElement('div');
    box.className = 'pop-diff';
    if (title) {
      const h = document.createElement('div');
      h.className = 'pop-diff-title';
      h.textContent = title;
      box.append(h);
    }
    for (const f of (files || []).slice(0, 60)) {
      const head = document.createElement('div');
      head.className = 'pop-diff-file';
      head.textContent = f.filename;
      const n = document.createElement('span');
      n.className = 'mono';
      n.textContent = `+${f.additions} −${f.deletions}`;
      head.append(' ', n);
      box.append(head);
      if (!f.patch) continue;
      const pre = document.createElement('pre');
      pre.className = 'pop-code is-diff';
      for (const line of f.patch.split('\n')) {
        const row = document.createElement('span');
        row.className = line.startsWith('+') ? 'add' : line.startsWith('-') ? 'del' : line.startsWith('@@') ? 'hunk' : '';
        row.textContent = `${line}\n`;
        pre.append(row);
      }
      box.append(pre);
    }
    return box;
  }

  function gistView(g) {
    const box = document.createElement('div');
    box.className = 'pop-diff';
    for (const f of Object.values(g.files || {})) {
      const head = document.createElement('div');
      head.className = 'pop-diff-file';
      head.textContent = f.filename;
      box.append(head, codeView(f.content || '', null));
    }
    return box;
  }

  // GitHub's own rendering (READMEs, issues), already sanitized by GitHub, shown in a
  // frame that may not run scripts.
  // GitHub's rendered HTML in this site's type (its own anchor icons hidden).
  const ghDoc = (html, base) =>
    `<!doctype html><meta charset="utf-8"><base href="${esc(base)}" target="_blank">` +
    '<link rel="stylesheet" href="https://fonts.googleapis.com/css2?family=Source+Serif+4:ital,opsz,wght@0,8..60,400;0,8..60,600;1,8..60,400&family=IBM+Plex+Mono:wght@400;500&display=swap">' +
    '<style>body{font:16px/1.6 "Source Serif 4",Georgia,serif;color:#111;background:#fff;max-width:720px;margin:0 auto;padding:14px 18px}' +
    'img{max-width:100%;height:auto}a{color:#111;text-decoration-color:#8a8a8a;text-underline-offset:3px}h1,h2,h3{line-height:1.25;font-weight:600}h1{font-size:26px}h2{font-size:21px}' +
    'a.anchor,.octicon{display:none}.markdown-heading{display:block}' +
    'pre{font:13px/1.5 "IBM Plex Mono",ui-monospace,monospace;background:#f6f6f3;border:1px solid #cfcfcf;padding:12px 14px;overflow:auto}' +
    'code{font-family:"IBM Plex Mono",ui-monospace,monospace;font-size:.86em}table{border-collapse:collapse}td,th{border:1px solid #cfcfcf;padding:4px 8px}</style>' +
    html;

  // An issue: a frame of its own, which may not run scripts.
  function htmlView(html, base) {
    const frame = document.createElement('iframe');
    frame.className = 'pop-frame';
    frame.setAttribute('sandbox', 'allow-popups allow-popups-to-escape-sandbox');
    frame.srcdoc = ghDoc(html, base);
    return frame;
  }

  // A repository, like its page on GitHub: name, description, numbers, the files at
  // its root, then the README (in a frame that may not run scripts, as tall as its
  // text so the popup scrolls as one page).
  function repoView(info, files, readme, base) {
    const box = document.createElement('div');
    box.className = 'pop-repo';
    const card = document.createElement('div');
    card.className = 'card';
    const name = Object.assign(document.createElement('span'), { className: 'card-title', textContent: info.full_name });
    card.append(name);
    if (info.description) {
      card.append(Object.assign(document.createElement('span'), { className: 'card-text', textContent: info.description }));
    }
    const facts = [
      `★ ${info.stargazers_count}`,
      `${info.forks_count} forks`,
      info.language,
      info.license && info.license.spdx_id !== 'NOASSERTION' ? info.license.spdx_id : null,
      info.pushed_at ? `updated ${info.pushed_at.slice(0, 10)}` : null,
    ].filter(Boolean);
    card.append(Object.assign(document.createElement('span'), { className: 'card-meta mono', textContent: facts.join(' · ') }));
    if (info.topics && info.topics.length) {
      card.append(Object.assign(document.createElement('span'), { className: 'mono', textContent: info.topics.join(' · ') }));
    }
    box.append(card);
    if (Array.isArray(files) && files.length) box.append(filesView(files));
    if (readme) {
      const frame = document.createElement('iframe');
      frame.className = 'pop-readme';
      // Same origin so the popup can size it to its text; no scripts run in it.
      frame.setAttribute('sandbox', 'allow-same-origin allow-popups allow-popups-to-escape-sandbox');
      frame.srcdoc = ghDoc(readme, base);
      box.append(frame);
    }
    return box;
  }

  // A folder: its folders, then its files, each linking to GitHub (and so to its own
  // popup: a folder lists its files, a file shows its code).
  function filesView(files) {
    const list = document.createElement('ul');
    list.className = 'pop-files';
    const sorted = [...(Array.isArray(files) ? files : [])].sort(
      (a, b) => (a.type === 'dir' ? 0 : 1) - (b.type === 'dir' ? 0 : 1) || a.name.localeCompare(b.name),
    );
    for (const f of sorted) {
      const li = document.createElement('li');
      const href = safeHref(f.html_url);
      const a = document.createElement(href ? 'a' : 'span');
      if (href) a.href = href;
      a.textContent = f.type === 'dir' ? `${f.name}/` : f.name;
      li.append(a);
      list.append(li);
    }
    return list;
  }

  function fitReadme(frame) {
    const fit = () => {
      try {
        frame.style.height = `${frame.contentDocument.documentElement.scrollHeight}px`;
      } catch {}
    };
    frame.addEventListener('load', () => {
      fit();
      // Images and fonts arrive after the text.
      setTimeout(fit, 800);
      setTimeout(fit, 2500);
    });
  }

  // What each outside link shows, loaded once per address and copied into each popup.
  const shown = new Map();
  const providerFor = (a) => /^[lu]:/.test(a.dataset.preview || '') && github(a.href);
  function content(a) {
    const prov = providerFor(a);
    if (!prov) return null;
    if (!shown.has(a.href)) shown.set(a.href, prov.load().catch(() => null));
    return shown.get(a.href);
  }

  async function enrich(p) {
    const prov = providerFor(p.link);
    if (!prov) return;
    const title = p.el.querySelector('.pop-title');
    title.replaceChildren(...[...title.childNodes].filter((n) => n.nodeType === 1), ` ${prov.label}`);
    const node = await content(p.link);
    if (!node || !popups.includes(p)) return;
    const view = node.cloneNode(true);
    view.querySelectorAll?.('iframe.pop-readme').forEach(fitReadme);
    p.body.replaceChildren(view);
    markLinks(p.body);
    p.el.classList.add(view.tagName === 'IFRAME' ? 'is-frame' : 'is-code');
    if (!p.pinned) place(p);
    const hl = view.querySelector && view.querySelector('.is-hl');
    if (hl) p.body.scrollTop = hl.offsetTop - 40;
  }

  // Frames are loaded ahead, one at a time, once the page has loaded, so their popups
  // open ready. A loaded frame's window is kept (hidden) and reused: moving an iframe
  // in the document would reload it. GitHub views are fetched ahead too (a few, to
  // stay well inside GitHub's limit of 60 API requests an hour).
  const ready = new Map();
  function preload() {
    // Pages with hundreds of outside links (the bookmarks) load nothing ahead: a
    // reader's browser contacts a site only when they hover its link.
    if (document.body.dataset.previews === 'lazy') return;
    const keys = new Set();
    let github = 0;
    document.querySelectorAll('a[data-preview]').forEach((a) => {
      const t = templates.get(a.dataset.preview);
      if (t && t.dataset.frame) keys.add(a.dataset.preview);
      else if (providerFor(a) && !shown.has(a.href) && github < 6) {
        github += 1;
        content(a);
      }
    });
    const next = (list) => {
      const key = list.shift();
      if (!key) return;
      if (ready.has(key)) return next(list);
      const el = build(key);
      el.classList.add('is-hidden');
      ready.set(key, el);
      document.body.appendChild(el);
      const done = () => next(list);
      el.querySelector('iframe').addEventListener('load', done, { once: true });
      setTimeout(done, 10000);
    };
    next([...keys]);
  }
  // A page shown inside a popup leaves preloading to the page around it, and its
  // links open in the whole window.
  const embedded = document.documentElement.classList.contains('embedded');
  if (embedded) {
    document.addEventListener('click', (e) => {
      const a = e.target.closest && e.target.closest('a[href]');
      if (a && !a.getAttribute('href').startsWith('#')) a.target = '_top';
    });
  } else {
    window.addEventListener('load', () => {
      (window.requestIdleCallback || ((f) => setTimeout(f, 500)))(preload);
    });
  }

  function open(a, byPointer = false) {
    let key = a.dataset.preview;
    // A link whose card never arrived still gets one built from the link itself.
    if (key.startsWith('l:') && !templates.has(key)) key = `u:${a.href}`;
    const section = key.startsWith('s:');
    const page = key.startsWith('p:');
    const feed = key.startsWith('f:');
    const outside = key.startsWith('u:');
    if (!section && !page && !feed && !outside && !templates.has(key)) return;
    const existing = byLink.get(a);
    if (existing && popups.includes(existing)) return raise(existing);

    // Opening from the page closes the other unpinned popups; opening from inside a
    // popup closes only that popup's other children.
    const parent = popOf(a);
    const keep = new Set(chain(parent));
    for (const p of [...popups]) if (!p.pinned && !keep.has(p)) close(p);

    let el = section ? null : ready.get(key);
    if (el && popups.some((p) => p.el === el)) close(popups.find((p) => p.el === el));
    if (el) el.classList.remove('is-hidden');
    else {
      el = section
        ? buildSection(a)
        : page
          ? buildPage(a)
          : feed
            ? buildFeed(a)
            : outside
              ? buildOutside(a)
              : build(key);
      if (!el) return;
      if (el.classList.contains('is-frame')) ready.set(key, el);
    }
    markLinks(el.querySelector('.pop-body'));
    const title = el.querySelector('.pop-title');
    if (!title.getAttribute('href')) title.href = a.href;
    if (!el.getAttribute('aria-label')) el.setAttribute('aria-label', a.textContent);

    el.classList.remove('is-leaving');
    const p = { el, body: el.querySelector('.pop-body'), link: a, parent, pinned: false, byPointer };
    popups.push(p);
    byLink.set(a, p);
    if (!el.isConnected) document.body.appendChild(el);
    raise(p);
    place(p);
    // Inside a maximized popup, a popup opens maximized too, over its parent. It
    // isn't pinned, so it closes when the pointer leaves it, showing the parent again.
    if (parent && parent.el.classList.contains('is-max')) setMax(p, true);
    a.classList.add('is-previewed');

    // A thin bar runs under the title while the popup's content is on its way.
    const loading = (job) => {
      el.classList.add('is-loading');
      Promise.resolve(job).finally(() => el.classList.remove('is-loading'));
    };
    if (key.startsWith('n:') || page) loading(fill(p));
    else if (feed) loading(fillFeed(p));
    else if (el.classList.contains('is-frame')) {
      const frame = el.querySelector('iframe');
      if (frame && !frame.dataset.loaded) {
        loading(new Promise((done) => {
          frame.addEventListener('load', done, { once: true });
          setTimeout(done, 15000);
        }));
      }
    } else if (providerFor(a)) loading(enrich(p));
  }

  function close(p) {
    for (const c of popups.filter((c) => c.parent === p)) {
      if (c.pinned) c.parent = null;
      else close(c);
    }
    const i = popups.indexOf(p);
    if (i < 0) return;
    popups.splice(i, 1);
    lastClose = performance.now();
    if (aim && aim.p === p) aim = null;
    p.link.classList.remove('is-previewed');
    if ([...ready.values()].includes(p.el)) {
      // Keep a loaded frame for next time.
      p.el.classList.add('is-hidden');
      setMax(p, false);
      p.el.classList.remove('is-collapsed', 'is-pinned', 'is-sized');
      p.el.style.width = '';
      p.el.style.height = '';
    } else {
      // Fade out, then go (it no longer counts as open).
      p.el.classList.add('is-leaving');
      setTimeout(() => p.el.remove(), 120);
    }
  }

  function raise(p) {
    p.el.style.zIndex = ++z;
  }

  function setMax(p, on) {
    p.el.classList.toggle('is-max', on);
    const b = p.el.querySelector('[data-act="max"]');
    b.title = on ? 'Restore' : 'Maximize';
    b.setAttribute('aria-label', b.title);
    // A maximized note shows the note's own page (without the site's header and
    // footer), so it is exactly the page: margin, contents, sidenotes and all.
    const note = /^[np]:/.test(p.link.dataset.preview || '');
    if (on && note && !p.el.querySelector('.pop-page')) {
      const page = document.createElement('iframe');
      page.className = 'pop-page';
      page.src = p.link.href;
      page.title = p.el.getAttribute('aria-label') || '';
      p.el.querySelector('.pop-body').after(page);
      p.el.classList.add('has-page');
    }
  }

  function pin(p, on = !p.pinned) {
    p.pinned = on;
    if (on) p.parent = null;
    p.el.classList.toggle('is-pinned', on);
  }

  // Below the link, or above it when there is more room there; level with the start
  // of the link, kept inside the window. A popup never covers its own link. On a
  // narrow screen it spans the width of the screen instead.
  // Opened by the mouse, a popup sits just under (or over) the line of the link the
  // pointer is on, starting a little left of the pointer, so it is a short move away.
  // Otherwise (keyboard) it lines up with the start of the link.
  function place(p) {
    const { el, link } = p;
    const sheet = window.innerWidth < SHEET;
    el.classList.toggle('is-sheet', sheet);
    const rects = [...link.getClientRects()];
    let line = rects[0] || link.getBoundingClientRect();
    if (p.byPointer && pointer) {
      line = rects.find((q) => pointer.y >= q.top - 2 && pointer.y <= q.bottom + 2) || line;
    }
    const x = p.byPointer && pointer ? pointer.x - 40 : line.left - 24;
    const vw = document.documentElement.clientWidth;
    const vh = window.innerHeight;
    const bar = el.querySelector('.pop-bar').offsetHeight + 2;
    const frame = el.classList.contains('is-frame');
    const want = sheet ? vh * 0.7 : Math.min(vh * (frame ? 0.7 : 0.6), frame ? 680 : 560);
    const below = vh - line.bottom - GAP - EDGE;
    const above = line.top - GAP - EDGE;
    el.style.left = sheet ? '' : `${clamp(x, EDGE, vw - el.offsetWidth - EDGE)}px`;
    const down = below >= want + bar || below >= above;
    el.classList.toggle('is-above', !down);
    if (down) {
      el.style.top = `${line.bottom + GAP}px`;
      el.style.bottom = '';
      el.style.setProperty('--pop-h', `${Math.min(want, below - bar)}px`);
    } else {
      el.style.top = '';
      el.style.bottom = `${vh - line.top + GAP}px`;
      el.style.setProperty('--pop-h', `${Math.min(want, above - bar)}px`);
    }
  }

  // Hover triangle (https://mayank.co/blog/hover-triangles/): moving from a link to
  // its popup often crosses other links. While the pointer stays inside the triangle
  // between where it left the link and the popup's near edge, and keeps moving,
  // nothing else opens and the popup doesn't close.
  let pointer = null;
  let aim = null; // { p, x, y, t }
  const AIM_IDLE = 400; // ms without moving inside the triangle before it lapses
  // The point is inside when it is on the same side of all three edges.
  const inTriangle = (px, py, [a, b, c]) => {
    const side = (p, q) => (px - q[0]) * (p[1] - q[1]) - (p[0] - q[0]) * (py - q[1]);
    const s = [side(a, b), side(b, c), side(c, a)];
    return !(s.some((v) => v < 0) && s.some((v) => v > 0));
  };
  function aiming() {
    if (!aim || !pointer || !popups.includes(aim.p)) return false;
    if (performance.now() - aim.t > AIM_IDLE) return false;
    const r = aim.p.el.getBoundingClientRect();
    if (pointer.x >= r.left && pointer.x <= r.right && pointer.y >= r.top && pointer.y <= r.bottom) return false;
    // Toward the popup's near edge: its side for a popup beside the link, else its
    // top or bottom.
    let a;
    let b;
    if (r.left >= aim.x) [a, b] = [[r.left, r.top], [r.left, r.bottom]];
    else if (r.right <= aim.x) [a, b] = [[r.right, r.top], [r.right, r.bottom]];
    else {
      const edge = r.top >= aim.y ? r.top : r.bottom;
      [a, b] = [[r.left, edge], [r.right, edge]];
    }
    return inTriangle(pointer.x, pointer.y, [[aim.x, aim.y], a, b]);
  }
  document.addEventListener(
    'mousemove',
    (e) => {
      pointer = { x: e.clientX, y: e.clientY };
      if (aim && aiming()) aim.t = performance.now();
    },
    { passive: true },
  );

  // Swap a note's excerpt for its whole text.
  async function fill(p) {
    const url = new URL(p.link.href, location.href);
    const doc = await fetchPage(url.pathname);
    if (!doc || !popups.includes(p)) return;
    addTemplates(doc);
    // A header link (`p:`) shows the whole page; a note shows its text.
    const whole = (p.link.dataset.preview || '').startsWith('p:');
    const prose = doc.querySelector(whole ? 'main' : '.note .prose');
    if (!prose) return;
    let text = document.importNode(prose, true);
    if (whole) {
      const box = document.createElement('div');
      box.className = 'pop-main';
      box.append(...text.childNodes);
      text = box;
    }
    const base = new URL(url.pathname, location.origin);
    text.querySelectorAll('[src]').forEach((e) => {
      e.setAttribute('src', new URL(e.getAttribute('src'), base).href);
    });
    text.querySelectorAll('a[href]').forEach((e) => {
      const h = e.getAttribute('href');
      if (!h.startsWith('#')) e.setAttribute('href', new URL(h, base).href);
    });
    text.querySelectorAll('input.margin-toggle').forEach((e) => e.remove());
    // A link to a section of the note shows just that section.
    let section = false;
    if (url.hash) {
      let h = null;
      try {
        h = text.querySelector(`#${CSS.escape(decodeURIComponent(url.hash.slice(1)))}`);
      } catch {}
      if (h && /^H[2-4]$/.test(h.tagName)) {
        text = sectionOf(h);
        section = true;
      }
    }
    const excerpt = p.body.querySelector('.card-text');
    if (excerpt) excerpt.replaceWith(text);
    else p.body.querySelector('.card')?.appendChild(text);
    p.el.classList.add('is-full');
    markLinks(p.body);
    if (url.hash && !section) scrollTo(p, url.hash.slice(1));
  }

  function scrollTo(p, id) {
    let target = null;
    try {
      target = p.body.querySelector(`#${CSS.escape(decodeURIComponent(id))}`);
    } catch {}
    if (!target) return;
    const top = target.getBoundingClientRect().top - p.body.getBoundingClientRect().top;
    p.body.scrollTop += top - 8;
  }

  // Close every unpinned popup that is not under the pointer, not an ancestor of the
  // one under it, and not opened from the link under it.
  function sweep() {
    if (aiming()) return scheduleSweep();
    const keep = new Set(chain(popOf(over)));
    const a = over && over.closest && over.closest('a[data-preview]');
    if (a && byLink.has(a)) chain(byLink.get(a)).forEach((p) => keep.add(p));
    for (const p of [...popups]) if (!p.pinned && !keep.has(p) && popups.includes(p)) close(p);
  }
  const scheduleSweep = () => {
    clearTimeout(sweepTimer);
    sweepTimer = setTimeout(sweep, CLOSE_DELAY);
  };

  // `data-preview="none"` opts a link out (the footer's Source, which opens a tab).
  const target = (e) => e.target.closest && e.target.closest('a[data-preview]:not([data-preview="none"])');

  document.addEventListener('mouseover', (e) => {
    over = e.target;
    scheduleSweep();
    const a = target(e);
    clearTimeout(openTimer);
    if (a && !(byLink.has(a) && popups.includes(byLink.get(a)))) {
      if (needsCards(a)) loadCards(); // start now, during the delay
      // Moving on from one popup to the next link opens it quickly (the reader is
      // already browsing); the first one waits, so passing links don't flash.
      const warm = popups.some((p) => !p.pinned) || performance.now() - lastClose < 600;
      const attempt = async () => {
        if (aiming()) {
          openTimer = setTimeout(attempt, 120); // on the way to a popup: wait
          return;
        }
        if (needsCards(a)) await loadCards();
        if (over && a.contains(over)) open(a, true);
      };
      openTimer = setTimeout(attempt, warm ? WARM_DELAY : OPEN_DELAY);
    }
  });
  document.addEventListener('mouseout', (e) => {
    if (!e.relatedTarget) {
      over = null;
      scheduleSweep();
    }
    // Leaving a link toward its popup starts the hover triangle.
    const a = target(e);
    const p = a && byLink.get(a);
    if (p && popups.includes(p) && !(e.relatedTarget && p.el.contains(e.relatedTarget))) {
      aim = { p, x: e.clientX, y: e.clientY, t: performance.now() };
    }
  });
  document.addEventListener('focusin', (e) => {
    const a = target(e);
    if (a && !matchMedia('(hover: none)').matches) openLazy(a);
  });
  document.addEventListener('keydown', (e) => {
    if (e.key !== 'Escape' || !popups.length) return;
    const top = popups.reduce((a, b) => (+a.el.style.zIndex > +b.el.style.zIndex ? a : b));
    close(top);
  });

  document.addEventListener('click', (e) => {
    const p = popOf(e.target);
    if (p) raise(p);

    const act = e.target.closest && e.target.closest('.pop-bar button');
    if (act && p) {
      const what = act.dataset.act;
      if (what === 'close') close(p);
      if (what === 'pin') pin(p);
      // Maximize and collapse undo each other: a collapsed popup is only its title
      // bar, back where it was before it was maximized.
      if (what === 'collapse') {
        const on = p.el.classList.toggle('is-collapsed');
        if (on) setMax(p, false);
      }
      if (what === 'max') {
        const on = !p.el.classList.contains('is-max');
        setMax(p, on);
        if (on) {
          p.el.classList.remove('is-collapsed');
          pin(p, true);
        }
      }
      return;
    }

    // Links to a heading of the note shown in a popup scroll the popup.
    const hash = e.target.closest && e.target.closest('.pop-body a[href^="#"]');
    if (hash && p) {
      e.preventDefault();
      scrollTo(p, hash.getAttribute('href').slice(1));
      return;
    }

    // Touch: the first tap opens the popup, a second tap on the same link follows it.
    const a = target(e);
    const touch = matchMedia('(hover: none)').matches;
    if (!a) {
      if (touch && !p) for (const q of [...popups]) if (!q.pinned) close(q);
      return;
    }
    if (touch && !(byLink.has(a) && popups.includes(byLink.get(a)))) {
      e.preventDefault();
      openLazy(a);
    }
  });

  // Drag a popup by anywhere on its title bar (the title too: a press that moves
  // doesn't follow the link), or resize it from the grip in its corner. Either one
  // pins it, so it stays where it is put.
  let swallowClick = false;
  document.addEventListener(
    'click',
    (e) => {
      if (!swallowClick) return;
      swallowClick = false;
      e.preventDefault();
      e.stopPropagation();
    },
    true,
  );
  document.addEventListener('dragstart', (e) => {
    if (e.target.closest && e.target.closest('.pop-bar')) e.preventDefault();
  });
  document.addEventListener('pointerdown', (e) => {
    swallowClick = false;
    if (e.button !== 0 || !e.target.closest || e.target.closest('button')) return;
    const grip = e.target.closest('.pop-grip');
    const handle = grip || e.target.closest('.pop-bar');
    const p = handle && popOf(handle);
    if (!p || p.el.classList.contains('is-max') || p.el.classList.contains('is-sheet')) return;
    const start = p.el.getBoundingClientRect();
    const sx = e.clientX;
    const sy = e.clientY;
    let moved = false;
    handle.setPointerCapture(e.pointerId);
    const move = (m) => {
      const dx = m.clientX - sx;
      const dy = m.clientY - sy;
      if (!moved) {
        if (Math.abs(dx) + Math.abs(dy) < 4) return; // a click, not a drag (yet)
        moved = true;
        pin(p, true);
        raise(p);
        p.el.style.top = `${start.top}px`;
        p.el.style.bottom = '';
        // Frames would swallow the pointer while it passes over them.
        document.documentElement.classList.add(grip ? 'pop-resizing' : 'pop-dragging');
      }
      const vw = document.documentElement.clientWidth;
      const vh = window.innerHeight;
      if (grip) {
        p.el.classList.add('is-sized');
        p.el.style.width = `${clamp(start.width + dx, 280, vw - start.left - EDGE)}px`;
        p.el.style.height = `${clamp(start.height + dy, 140, vh - start.top - EDGE)}px`;
      } else {
        p.el.style.left = `${clamp(start.left + dx, 60 - start.width, vw - 60)}px`;
        p.el.style.top = `${clamp(start.top + dy, 0, vh - 30)}px`;
      }
    };
    const up = () => {
      handle.removeEventListener('pointermove', move);
      document.documentElement.classList.remove('pop-dragging', 'pop-resizing');
      swallowClick = moved;
    };
    handle.addEventListener('pointermove', move);
    handle.addEventListener('pointerup', up, { once: true });
    handle.addEventListener('pointercancel', up, { once: true });
  });

  // Popups stay put while the page scrolls under them, so scrolling the page closes
  // the unpinned ones (unless the pointer is on one). A sheet on a phone stays until
  // its link scrolls out of sight.
  window.addEventListener(
    'scroll',
    () => {
      for (const p of [...popups]) {
        if (p.pinned || !popups.includes(p) || p.el.contains(over)) continue;
        const r = p.link.getBoundingClientRect();
        const gone = r.bottom < 0 || r.top > window.innerHeight;
        if (gone || !p.el.classList.contains('is-sheet')) close(p);
      }
    },
    { passive: true },
  );
  window.addEventListener('resize', () => {
    for (const p of [...popups]) if (!p.pinned && popups.includes(p)) place(p);
  });
})();
