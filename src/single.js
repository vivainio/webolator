
(() => {
  const assets = JSON.parse(document.getElementById('webolator-assets').textContent);
  const sections = [...document.querySelectorAll('section.page')];
  const blobUrls = {};

  function assetUrl(key) {
    if (!blobUrls[key]) {
      const [mime, b64] = assets[key];
      const bin = atob(b64);
      const bytes = new Uint8Array(bin.length);
      for (let i = 0; i < bin.length; i++) bytes[i] = bin.charCodeAt(i);
      blobUrls[key] = URL.createObjectURL(new Blob([bytes], { type: mime }));
    }
    return blobUrls[key];
  }

  document.addEventListener('click', (e) => {
    const a = e.target.closest('a[href^="#asset:"]');
    if (!a) return;
    e.preventDefault();
    const key = decodeURIComponent(a.getAttribute('href').slice(7));
    if (assets[key]) window.open(assetUrl(key), '_blank');
  });

  function show() {
    const id = decodeURIComponent(location.hash.slice(1));
    let target = id && document.getElementById(id);
    if (!target || !target.closest('section.page')) target = document.getElementById(WEBOLATOR_FIRST);
    const section = target.closest('section.page');
    for (const s of sections) s.hidden = s !== section;
    document.title = section.dataset.title;
    for (const a of document.querySelectorAll('nav.side a')) {
      a.classList.toggle('active', a.getAttribute('href') === '#' + section.id);
    }
    if (window.webolatorMermaid) window.webolatorMermaid(section);
    if (target !== section) target.scrollIntoView();
    else window.scrollTo(0, 0);
  }

  window.addEventListener('hashchange', show);
  show();
})();
