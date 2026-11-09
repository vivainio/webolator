
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

  // ---- image zoom (gallery tiles: full image, arrow keys step through the folder)
  document.addEventListener('click', (e) => {
    const diagram = e.target.closest('article pre.mermaid svg');
    if (diagram && !e.target.closest('a')) {
      const overlay = document.createElement('div');
      overlay.className = 'zoom zoom-diagram';
      overlay.setAttribute('role', 'dialog');
      overlay.setAttribute('aria-label', 'Enlarged diagram');
      const closeButton = document.createElement('button');
      closeButton.type = 'button';
      closeButton.textContent = 'Close';
      const controls = document.createElement('div');
      controls.className = 'zoom-diagram-controls';
      const zoomOut = document.createElement('button');
      zoomOut.type = 'button';
      zoomOut.textContent = '−';
      zoomOut.setAttribute('aria-label', 'Zoom out');
      const zoomIn = document.createElement('button');
      zoomIn.type = 'button';
      zoomIn.textContent = '+';
      zoomIn.setAttribute('aria-label', 'Zoom in');
      const reset = document.createElement('button');
      reset.type = 'button';
      reset.textContent = 'Reset';
      controls.append(zoomOut, zoomIn, reset, closeButton);
      const content = document.createElement('div');
      content.className = 'zoom-diagram-content';
      const copy = diagram.cloneNode(true);
      copy.removeAttribute('id');
      content.append(copy);
      overlay.append(controls, content);
      let scale = 1, x = 0, y = 0;
      const pointers = new Map();
      let gesture;
      const draw = () => { copy.style.transform = `translate(${x}px, ${y}px) scale(${scale})`; };
      const setScale = (value) => { scale = Math.max(0.25, Math.min(8, value)); draw(); };
      zoomIn.addEventListener('click', () => setScale(scale * 1.25));
      zoomOut.addEventListener('click', () => setScale(scale / 1.25));
      reset.addEventListener('click', () => { scale = 1; x = 0; y = 0; draw(); });
      const measure = () => {
        const points = [...pointers.values()];
        const a = points[0], b = points[1] || a;
        return { x: (a.x + b.x) / 2, y: (a.y + b.y) / 2, distance: Math.hypot(b.x - a.x, b.y - a.y) };
      };
      content.addEventListener('pointerdown', (event) => {
        if (event.button !== 0 || event.target.closest('a')) return;
        content.setPointerCapture(event.pointerId);
        pointers.set(event.pointerId, { x: event.clientX, y: event.clientY });
        gesture = measure();
        content.classList.add('dragging');
      });
      content.addEventListener('pointermove', (event) => {
        if (!pointers.has(event.pointerId)) return;
        pointers.set(event.pointerId, { x: event.clientX, y: event.clientY });
        const next = measure();
        x += next.x - gesture.x;
        y += next.y - gesture.y;
        if (pointers.size === 2 && gesture.distance) scale = Math.max(0.25, Math.min(8, scale * next.distance / gesture.distance));
        gesture = next;
        draw();
      });
      const endPointer = (event) => {
        pointers.delete(event.pointerId);
        gesture = pointers.size ? measure() : null;
        if (!pointers.size) content.classList.remove('dragging');
      };
      content.addEventListener('pointerup', endPointer);
      content.addEventListener('pointercancel', endPointer);
      content.addEventListener('wheel', (event) => {
        event.preventDefault();
        setScale(scale * Math.exp(-event.deltaY * 0.002));
      }, { passive: false });
      const close = () => { overlay.remove(); removeEventListener('keydown', onKey); };
      const onKey = (event) => { if (event.key === 'Escape') close(); };
      closeButton.addEventListener('click', close);
      overlay.addEventListener('click', (event) => { if (event.target === overlay) close(); });
      addEventListener('keydown', onKey);
      document.body.append(overlay);
      closeButton.focus();
      return;
    }
    const img = e.target.closest('article img');
    if (!img || img.closest('a')) return;
    const group = img.closest('.gallery');
    const imgs = group ? [...group.querySelectorAll('img')] : [img];
    let i = imgs.indexOf(img);
    const overlay = document.createElement('div');
    overlay.className = 'zoom';
    const big = document.createElement('img');
    const cap = document.createElement('div');
    cap.className = 'zoom-caption';
    overlay.append(big, cap);
    const show = () => {
      const cur = imgs[i];
      big.src = cur.dataset.full || cur.currentSrc || cur.src;
      big.alt = cur.alt;
      cap.textContent = group ? (cur.closest('figure')?.querySelector('figcaption')?.textContent || cur.alt) : '';
    };
    const close = () => { overlay.remove(); removeEventListener('keydown', onKey); };
    const onKey = (k) => {
      if (k.key === 'Escape') close();
      else if (k.key === 'ArrowRight') { i = (i + 1) % imgs.length; show(); }
      else if (k.key === 'ArrowLeft') { i = (i - 1 + imgs.length) % imgs.length; show(); }
    };
    show();
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
