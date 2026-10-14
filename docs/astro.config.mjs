import { defineConfig } from "astro/config";
import { satteri } from "@astrojs/markdown-satteri";
import { docsBlocks } from "./src/markdown/blocks.mjs";

export default defineConfig({
  site: "https://docs.nemonik.io",
  trailingSlash: "always",
  markdown: {
    syntaxHighlight: false,
    processor: satteri({
      features: { directive: true, smartPunctuation: false },
      mdastPlugins: [docsBlocks],
    }),
  },
});
