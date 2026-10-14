import type { APIRoute } from "astro";
import { getCollection } from "astro:content";
import { SITE, grouped, sortPages } from "../lib/site";

export const GET: APIRoute = async () => {
  const pages = sortPages((await getCollection("pages")).map((e) => ({ id: e.id, ...e.data })));
  const sections = grouped(pages).map(
    (g) =>
      `## ${g.group}\n\n` +
      g.pages.map((p) => `- [${p.title}](${SITE}/${p.id}.md): ${p.description}`).join("\n"),
  );
  const body = [
    "# nemonik docs",
    "> nemonik packages how your team works into a kit: a system prompt, skills, MCP servers, hooks and a workflow. Running a kit starts Claude Code or Codex with all of it in place.",
    ...sections,
  ].join("\n\n");
  return new Response(body + "\n", { headers: { "Content-Type": "text/plain; charset=utf-8" } });
};
