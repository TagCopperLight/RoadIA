// Initialize mermaid and replace code blocks with rendered diagrams (root copy)
function renderMermaidBlocks() {
  try {
    mermaid.initialize({ startOnLoad: false, theme: 'default' });
  } catch (e) {
    console.error('Error initializing mermaid', e);
  }

  const codeBlocks = Array.from(document.querySelectorAll('pre code.language-mermaid, code.language-mermaid'));
  codeBlocks.forEach((codeEl, idx) => {
    const parentPre = codeEl.closest('pre');
    const source = codeEl.textContent || codeEl.innerText;
    const wrapper = document.createElement('div');
    const id = `mermaid-${idx}-${Math.random().toString(36).slice(2,8)}`;
    wrapper.className = 'mermaid';
    wrapper.id = id;
    wrapper.textContent = source;
    if (parentPre && parentPre.parentNode) {
      parentPre.parentNode.replaceChild(wrapper, parentPre);
    } else if (codeEl.parentNode) {
      codeEl.parentNode.replaceChild(wrapper, codeEl);
    }
    try {
      mermaid.init(undefined, `#${id}`);
    } catch (e) {
      console.error('Mermaid render error', e);
    }
  });
}

document.addEventListener("DOMContentLoaded", function () {
  var attempts = 0;
  var maxAttempts = 100; // ~10s
  var interval = setInterval(function () {
    attempts++;
    if (typeof mermaid !== 'undefined') {
      clearInterval(interval);
      renderMermaidBlocks();
    } else if (attempts >= maxAttempts) {
      clearInterval(interval);
      console.warn('Mermaid did not load within timeout');
    }
  }, 100);
});
