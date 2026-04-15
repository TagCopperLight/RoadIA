// Inject KaTeX CSS and JS from CDN
(function(){
  if (window.__katex_loader_done) return;
  window.__katex_loader_done = true;

  var link = document.createElement('link');
  link.rel = 'stylesheet';
  link.href = 'https://cdn.jsdelivr.net/npm/katex@0.16.8/dist/katex.min.css';
  document.head.appendChild(link);

  var script = document.createElement('script');
  script.src = 'https://cdn.jsdelivr.net/npm/katex@0.16.8/dist/katex.min.js';
  script.defer = true;
  document.head.appendChild(script);

  var ar = document.createElement('script');
  ar.src = 'https://cdn.jsdelivr.net/npm/katex@0.16.8/dist/contrib/auto-render.min.js';
  ar.defer = true;
  document.head.appendChild(ar);
})();
