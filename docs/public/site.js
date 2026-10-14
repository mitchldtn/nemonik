(function () {
  const $ = (s) => document.querySelector(s);
  const root = document.documentElement;
  const copy = (text, done) => navigator.clipboard?.writeText(text).then(done, () => {});

  // theme (the <head> snippet applies the saved choice before first paint)
  const tb = $("#theme");
  const showTheme = () => (tb.textContent = root.classList.contains("dark") ? "light" : "dark");
  showTheme();
  tb.addEventListener("click", () => {
    const t = root.classList.toggle("dark") ? "dark" : "light";
    try { localStorage.setItem("nk-docs-theme", t); } catch (_) {}
    showTheme();
  });
  $("#menu").addEventListener("click", () => document.body.classList.toggle("nav-open"));

  // copy buttons: drop "$ " prompts and comment lines
  document.querySelectorAll(".cb").forEach((cb) => {
    const btn = cb.querySelector(".lab button");
    btn.addEventListener("click", () => {
      const text = cb.querySelector("pre").textContent
        .split("\n").map((l) => l.replace(/^\$ /, "")).filter((l) => !/^#/.test(l.trim())).join("\n").trim();
      copy(text, () => {
        btn.textContent = "Copied";
        setTimeout(() => (btn.textContent = "Copy"), 1200);
      });
    });
  });

  document.querySelectorAll("article h2 .anchor").forEach((a) => {
    a.addEventListener("click", () => copy(new URL(a.href).href, () => {
      a.classList.add("copied");
      setTimeout(() => a.classList.remove("copied"), 1400);
    }));
  });

  const hs = [...document.querySelectorAll("#content h2")];
  const toc = [...document.querySelectorAll("#toc a")];
  function spy() {
    let on = hs[0];
    hs.forEach((h) => { if (h.getBoundingClientRect().top < 140) on = h; });
    toc.forEach((a) => a.classList.toggle("on", !!on && a.dataset.h === on.id));
  }
  if (hs.length) { window.addEventListener("scroll", spy, { passive: true }); spy(); }

  // search: an empty query lists the pages, anything else goes to Pagefind
  const pages = JSON.parse($("#k-pages").textContent);
  const ov = $("#k"), inp = $("#k-in"), res = $("#k-res");
  let sel = 0, hits = [], pagefind = null, seq = 0;
  const esc = (s) => String(s).replace(/[&<>"]/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" })[c]);

  async function results(q) {
    const n = ++seq;
    q = q.trim();
    let next = pages;
    if (q) {
      pagefind = pagefind || (await import("/pagefind/pagefind.js"));
      const found = await pagefind.search(q);
      const data = await Promise.all(found.results.slice(0, 8).map((r) => r.data()));
      next = data.flatMap((d) => [{ t: d.meta.title, g: d.meta.group || "", href: d.url }].concat(
        d.sub_results.filter((s) => s.url.includes("#")).slice(0, 3).map((s) => ({ t: s.title, g: d.meta.title, href: s.url })),
      ));
    }
    if (n !== seq) return;
    hits = q ? next.slice(0, 20) : next; sel = 0; draw();
  }
  function draw() {
    res.innerHTML = hits.map((x, i) =>
      `<a href="${esc(x.href)}" class="${i === sel ? "sel" : ""}" data-i="${i}">${esc(x.t)}<span>${esc(x.g)}</span></a>`,
    ).join("") || '<div class="k-foot">No match</div>';
    const s = res.querySelector(".sel");
    if (s) s.scrollIntoView({ block: "nearest" });
  }
  function go(x) { closeK(); location.href = x.href; }
  function openK() { ov.classList.add("open"); inp.value = ""; results(""); setTimeout(() => inp.focus(), 20); }
  function closeK() { ov.classList.remove("open"); }
  $("#open-k").addEventListener("click", openK);
  ov.addEventListener("click", (e) => { if (e.target === ov) closeK(); });
  res.addEventListener("click", (e) => { const a = e.target.closest("a"); if (a) { e.preventDefault(); go(hits[+a.dataset.i]); } });
  inp.addEventListener("input", () => results(inp.value));
  document.addEventListener("keydown", (e) => {
    const open = ov.classList.contains("open");
    if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === "k") { e.preventDefault(); open ? closeK() : openK(); return; }
    if (e.key === "/" && !open && !/^(INPUT|TEXTAREA|SELECT)$/.test(document.activeElement.tagName)) { e.preventDefault(); openK(); return; }
    if (!open) return;
    if (e.key === "Escape") closeK();
    if (e.key === "ArrowDown") { sel = Math.min(sel + 1, hits.length - 1); draw(); e.preventDefault(); }
    if (e.key === "ArrowUp") { sel = Math.max(sel - 1, 0); draw(); e.preventDefault(); }
    if (e.key === "Enter" && hits[sel]) { e.preventDefault(); go(hits[sel]); }
  });
})();
