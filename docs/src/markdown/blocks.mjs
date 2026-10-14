// Turns the docs' Markdown extensions into the site's blocks:
//   :::note            a margin note (moved to the right rail by the page)
//   :::callout[Label]  a ruled note with a label
//   :::more            a list of "[Title](/page/): description" next-step links
//   :tag[text]         a small label inside a table cell
//   ```<lang> title="Label"   a labelled code block with a Copy button
//   ```output          a command transcript, no Copy button
//   ```diagram title="Label"  a plain diagram block
import { defineMdastPlugin } from "satteri";

const escapeHtml = (s) =>
  s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/"/g, "&quot;");

const span = (cls, text) => `<span class="${cls}">${escapeHtml(text)}</span>`;

// Languages whose double-quoted strings are coloured and whose trailing "  # ..." is a comment.
const STRING_LANGS = new Set(["json", "mcp"]);

function codeLine(line, lang) {
  if (line.trimStart().startsWith("#")) return span("cm", line);
  let prompt = "";
  if (line.startsWith("$ ")) {
    prompt = span("p", "$ ");
    line = line.slice(2);
  }
  if (!STRING_LANGS.has(lang)) return prompt + escapeHtml(line);
  const comment = line.match(/\s{2,}#.*$/);
  const body = comment ? line.slice(0, comment.index) : line;
  const coloured = body
    .split(/("(?:[^"\\]|\\.)*")/)
    .map((part, i) => (i % 2 ? span("s", part) : escapeHtml(part)))
    .join("");
  return prompt + coloured + (comment ? span("cm", comment[0]) : "");
}

function codeBlock(lang, meta, value) {
  const text = value.replace(/\n$/, "");
  if (lang === "output") {
    return `<div class="out"><div class="lab">output</div><pre>${escapeHtml(text)}</pre></div>`;
  }
  const title = meta?.match(/title="([^"]*)"/)?.[1];
  const label = escapeHtml(title ?? (lang === "diagram" ? "diagram" : "code"));
  if (lang === "diagram") {
    return `<div class="diagram"><div class="lab">${label}</div><pre>${escapeHtml(text)}</pre></div>`;
  }
  const lines = text.split("\n").map((l) => codeLine(l, lang)).join("\n");
  return `<div class="cb"><div class="lab"><span>${label}</span><button type="button">Copy</button></div><pre>${lines}</pre></div>`;
}

export const docsBlocks = defineMdastPlugin({
  name: "nemonik-docs-blocks",
  code(node, ctx) {
    ctx.replaceNode(node, { type: "html", value: codeBlock(node.lang, node.meta, node.value) });
  },
  textDirective(node, ctx) {
    if (node.name !== "tag") return;
    ctx.setProperty(node, "data", { hName: "span", hProperties: { className: ["tag"] } });
  },
  containerDirective(node, ctx) {
    if (node.name === "note") {
      ctx.setProperty(node, "data", { hName: "aside", hProperties: { className: ["note"] } });
    } else if (node.name === "callout") {
      const [label, first] = node.children;
      if (label?.data?.directiveLabel && first) {
        ctx.removeNode(label);
        ctx.prependChild(first, { type: "html", value: `<b>${escapeHtml(ctx.textContent(label))}</b>` });
      }
      ctx.setProperty(node, "data", { hName: "div", hProperties: { className: ["callout"] } });
    } else if (node.name === "more") {
      const items = node.children.flatMap((list) => (list.type === "list" ? list.children : []));
      const links = items.map((item) => {
        const para = item.children[0];
        const link = para.children.find((c) => c.type === "link");
        const title = ctx.textContent(link);
        const desc = ctx.textContent(para).replace(title, "").replace(/^\s*:\s*/, "");
        return `<a href="${escapeHtml(link.url)}"><strong>${escapeHtml(title)}</strong><span>${escapeHtml(desc)}</span></a>`;
      });
      ctx.replaceNode(node, { type: "html", value: `<div class="more">${links.join("")}</div>` });
    }
  },
});
