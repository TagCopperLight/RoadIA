// Poll for katex and auto-render then render math blocks
(function(){
  function tryRender() {
    if (window.katex && window.renderMathInElement) {
      try {
        renderMathInElement(document.body, {
          delimiters: [
            {left: "$$", right: "$$", display: true},
            {left: "$", right: "$", display: false}
          ],
          throwOnError: false
        });
      } catch (e) {
        console.warn('KaTeX render failed', e);
      }
      return true;
    }
    return false;
  }

  var attempts = 0;
  var id = setInterval(function(){
    attempts++;
    if (tryRender() || attempts > 40) {
      clearInterval(id);
    }
  }, 200);
})();
