import { defineCollection } from "astro:content";
import { glob } from "astro/loaders";
import { z } from "astro/zod";
import { GROUPS } from "./lib/site";

const pages = defineCollection({
  loader: glob({ pattern: "*.md", base: "./content" }),
  schema: z.object({
    title: z.string().min(1),
    group: z.enum(GROUPS),
    order: z.number(),
    description: z.string().min(1),
  }),
});

export const collections = { pages };
