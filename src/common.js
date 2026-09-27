
(() => {
  const root = document.documentElement;
  const isDark = () =>
    (root.dataset.theme || (matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light')) === 'dark';

  // ---- mermaid: render, and re-render when the theme changes
  window.webolatorMermaidRun = (scope) => {
    if (!window.mermaid) return;
    for (const p of document.querySelectorAll('pre.mermaid:not([data-src])')) p.dataset.src = p.textContent;
    mermaid.initialize({ startOnLoad: false, theme: isDark() ? 'dark' : 'default' });
    mermaid.run({ nodes: scope.querySelectorAll('pre.mermaid:not([data-processed])') });
  };
  function rethemeMermaid() {
    if (!window.mermaid) return;
    for (const p of document.querySelectorAll('pre.mermaid[data-processed]')) {
      p.removeAttribute('data-processed');
      p.textContent = p.dataset.src;
    }
    // In single-file output only the visible section can be measured; others render when shown.
    webolatorMermaidRun(document.querySelector('section.page:not([hidden])') || document);
  }

  // ---- light/dark toggle (remembered per browser)
  for (const b of document.querySelectorAll('.theme-toggle')) {
    b.addEventListener('click', () => {
      const next = isDark() ? 'light' : 'dark';
      root.dataset.theme = next;
      try { localStorage.setItem('webolator-theme', next); } catch (e) {}
      rethemeMermaid();
    });
  }

  // ---- mobile menu
  const nav = document.querySelector('nav.side');
  const menu = document.querySelector('.menu-toggle');
  if (nav && menu) {
    menu.addEventListener('click', () => {
      const open = nav.classList.toggle('open');
      menu.setAttribute('aria-expanded', String(open));
    });
    nav.addEventListener('click', (e) => {
      if (e.target.closest('a')) nav.classList.remove('open');
    });
  }

  // ---- copy buttons on code blocks
  for (const pre of document.querySelectorAll('article pre')) {
    const code = pre.querySelector('code');
    if (!code || pre.dataset.mathStyle) continue;
    const wrap = document.createElement('div');
    wrap.className = 'code-wrap';
    pre.replaceWith(wrap);
    wrap.append(pre);
    const btn = document.createElement('button');
    btn.className = 'copy';
    btn.type = 'button';
    btn.textContent = 'Copy';
    btn.addEventListener('click', async () => {
      const text = code.innerText;
      try {
        await navigator.clipboard.writeText(text);
      } catch (e) {
        const ta = document.createElement('textarea');
        ta.value = text;
        document.body.append(ta);
        ta.select();
        document.execCommand('copy');
        ta.remove();
      }
      btn.textContent = 'Copied';
      setTimeout(() => (btn.textContent = 'Copy'), 1200);
    });
    wrap.append(btn);
  }

  // ---- "On this page": highlight the section being read
  for (const toc of document.querySelectorAll('aside.toc')) {
    const links = [...toc.querySelectorAll('a')];
    const heads = links.map((a) => document.getElementById(decodeURIComponent(a.hash.slice(1)))).filter(Boolean);
    const update = () => {
      let current = heads[0];
      for (const h of heads) if (h.getBoundingClientRect().top < 120) current = h;
      for (const a of links) a.classList.toggle('active', current && a.hash.slice(1) === current.id);
    };
    addEventListener('scroll', update, { passive: true });
    addEventListener('hashchange', update);
    update();
  }

  // ---- image zoom
  document.addEventListener('click', (e) => {
    const img = e.target.closest('article img');
    if (!img || img.closest('a')) return;
    const overlay = document.createElement('div');
    overlay.className = 'zoom';
    const big = img.cloneNode();
    big.removeAttribute('width');
    big.removeAttribute('height');
    overlay.append(big);
    const close = () => { overlay.remove(); removeEventListener('keydown', onKey); };
    const onKey = (k) => { if (k.key === 'Escape') close(); };
    overlay.addEventListener('click', close);
    addEventListener('keydown', onKey);
    document.body.append(overlay);
  });

  // ---- math (KaTeX is only loaded on pages that contain math)
  function renderMath() {
    for (const el of document.querySelectorAll('[data-math-style]')) {
      const display = el.dataset.mathStyle === 'display';
      const target = document.createElement(display ? 'div' : 'span');
      target.className = display ? 'math-display' : 'math-inline';
      try {
        katex.render(el.textContent, target, { displayMode: display, throwOnError: false });
        el.replaceWith(target);
      } catch (e) {}
    }
  }
  if (document.querySelector('[data-math-style]')) {
    if (window.katex) renderMath();
    else addEventListener('DOMContentLoaded', () => window.katex && renderMath());
    addEventListener('load', () => window.katex && renderMath());
  }
})();
