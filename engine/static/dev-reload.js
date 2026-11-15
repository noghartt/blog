// `garden serve` only: reload the page after a rebuild.
(() => {
  let v = null;
  setInterval(async () => {
    try {
      const n = await (await fetch('/__garden/version', { cache: 'no-store' })).text();
      if (v !== null && n !== v) location.reload();
      v = n;
    } catch {}
  }, 1000);
})();
