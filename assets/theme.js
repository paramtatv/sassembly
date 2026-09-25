/* ── Theme: light by default, dark on request ─────────────────────────────────
   LIGHT IS THE DEFAULT AND SYSTEM PREFERENCE IS NOT CONSULTED. The stylesheet
   carries the light palette on `:root` and the dark one on
   `:root[data-theme="dark"]`, with no `prefers-color-scheme` rule, so a visitor
   on a dark OS still lands on light. Dark is a choice this file records.

   Loaded from <head> WITHOUT defer, on purpose: the stamp has to be on the root
   element before the first paint, or a returning visitor who chose dark sees a
   flash of light first.

   Every storage access is wrapped — localStorage throws in a private window and
   can be blocked outright, and a theme toggle is not worth a broken page.
   ──────────────────────────────────────────────────────────────────────────── */
(function () {
  var KEY = 'sassembly-theme';

  function stored() {
    try { return localStorage.getItem(KEY); } catch (e) { return null; }
  }
  function remember(v) {
    try { localStorage.setItem(KEY, v); } catch (e) { /* private window: honour it for this page only */ }
  }
  function apply(v) {
    if (v === 'dark') { document.documentElement.setAttribute('data-theme', 'dark'); }
    else { document.documentElement.removeAttribute('data-theme'); }
  }

  // before first paint
  apply(stored() === 'dark' ? 'dark' : 'light');

  // wire the button once the masthead exists
  document.addEventListener('DOMContentLoaded', function () {
    var btns = document.querySelectorAll('.themetoggle');
    function sync() {
      var dark = document.documentElement.getAttribute('data-theme') === 'dark';
      for (var i = 0; i < btns.length; i++) {
        btns[i].setAttribute('aria-pressed', dark ? 'true' : 'false');
        btns[i].setAttribute('aria-label', dark ? 'Switch to light theme' : 'Switch to dark theme');
      }
    }
    for (var i = 0; i < btns.length; i++) {
      btns[i].addEventListener('click', function () {
        var next = document.documentElement.getAttribute('data-theme') === 'dark' ? 'light' : 'dark';
        apply(next); remember(next); sync();
      });
    }
    sync();
  });
})();
