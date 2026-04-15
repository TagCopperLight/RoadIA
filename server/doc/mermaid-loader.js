// mermaid-loader.js (root copy)
(function () {
  var url = 'https://unpkg.com/mermaid@10/dist/mermaid.min.js';
  if (typeof document === 'undefined') return;
  var s = document.createElement('script');
  s.src = url;
  s.async = true;
  s.onload = function () {
    console.log('Mermaid loaded from CDN');
  };
  s.onerror = function () {
    console.warn('Failed to load mermaid from CDN:', url);
  };
  document.head.appendChild(s);
})();
