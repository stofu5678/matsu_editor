// Adds a "search" link to the menu bar of every page (ADR-0031). `additional-js` of book.toml loads it.
// Over http(s) (GitHub Pages) it goes to find.html, the Pagefind search page. As a file (the help that comes
// with the program) Pagefind cannot read its index, so it goes to search.html, the one-page version for Ctrl+F.
(function () {
  var buttons = document.querySelector(".right-buttons");
  if (!buttons) {
    return;
  }
  var web = location.protocol === "http:" || location.protocol === "https:";
  var root = typeof path_to_root === "string" ? path_to_root : "";
  var link = document.createElement("a");
  link.href = root + (web ? "find.html" : "search.html");
  link.title = "ヘルプを検索";
  link.setAttribute("aria-label", "ヘルプを検索");
  link.className = "menu-title";
  link.style.cssText = "padding: 0 8px; text-decoration: none; font-size: 1.6rem;";
  link.textContent = "検索";
  buttons.insertBefore(link, buttons.firstChild);
})();
