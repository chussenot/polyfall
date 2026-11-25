// mkdocs renders ```mermaid fences as <pre><code class="language-mermaid">; turn them into diagrams.
document.querySelectorAll("pre > code.language-mermaid").forEach(function (code) {
  var div = document.createElement("div");
  div.className = "mermaid";
  div.textContent = code.textContent;
  code.parentNode.replaceWith(div);
});
mermaid.initialize({ startOnLoad: false });
mermaid.run();
