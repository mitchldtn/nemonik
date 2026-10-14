import type { APIRoute, GetStaticPaths } from "astro";
import { getCollection } from "astro:content";

export const getStaticPaths = (async () =>
  (await getCollection("pages")).map((entry) => ({ params: { page: entry.id }, props: { entry } }))) satisfies GetStaticPaths;

export const GET: APIRoute = ({ props }) => {
  const { title, description } = props.entry.data;
  const body = `# ${title}\n\n${description}\n\n${(props.entry.body ?? "").trim()}\n`;
  return new Response(body, { headers: { "Content-Type": "text/markdown; charset=utf-8" } });
};
