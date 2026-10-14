// Post-processes a page's rendered HTML: margin notes come out of the article
// (the layout shows them in the right rail), the first paragraph becomes the
// lead, and every h2 gets a "#" section link.

const NOTE = /<aside class="note">([\s\S]*?)<\/aside>\s*/g;
const H2 = /<h2 id="([^"]+)">([\s\S]*?)<\/h2>/g;

export interface Article {
  body: string;
  notes: string[];
}

export function splitArticle(html: string): Article {
  const notes = [...html.matchAll(NOTE)].map((m) => m[1].trim());
  const body = html
    .replace(NOTE, "")
    .replace(/^\s*<p>/, '<p class="lead">')
    .replace(
      H2,
      (_, id, inner) =>
        `<h2 id="${id}">${inner}<a class="anchor" href="#${id}" data-pagefind-ignore aria-label="Link to this section">#</a></h2>`,
    );
  return { body, notes };
}
