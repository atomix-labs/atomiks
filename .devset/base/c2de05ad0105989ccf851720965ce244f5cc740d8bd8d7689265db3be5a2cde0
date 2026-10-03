// mdBook has no config for an extra menu-bar link, only the repository, edit and print buttons,
// and forking index.hbs to add one would mean owning a template that changes across releases. So
// the link is appended to the same button row at load, using the `path_to_root` mdBook already
// defines per page so it is depth-correct everywhere.
// The id carries mdBook 0.5's `mdbook-` prefix; the 0.4 name silently matches nothing.
(() => {
  const buttons = document.querySelector("#mdbook-menu-bar .right-buttons");
  if (!buttons) return;
  const link = document.createElement("a");
  link.href = `${path_to_root}api/index.html`;
  link.title = "API reference for every crate in the workspace";
  link.className = "api-reference";
  link.textContent = "API Reference";
  buttons.prepend(link);
})();
