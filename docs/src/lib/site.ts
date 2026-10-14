import { readFileSync } from "node:fs";
import { resolve } from "node:path";

export const SITE = "https://docs.nemonik.io";
export const GITHUB = "https://github.com/mitchldtn/nemonik";

export const GROUPS = ["Getting started", "Reference", "Kits", "Context", "Guides"] as const;
export type Group = (typeof GROUPS)[number];

export const TAB_LABEL: Record<Group, string> = {
  "Getting started": "Guide",
  Reference: "Reference",
  Kits: "Kits",
  Context: "Context",
  Guides: "Guides",
};

export const HOME = "introduction";

export interface PageMeta {
  id: string;
  title: string;
  group: Group;
  order: number;
  description: string;
}

export function sortPages<P extends PageMeta>(pages: readonly P[]): P[] {
  return [...pages].sort(
    (a, b) => GROUPS.indexOf(a.group) - GROUPS.indexOf(b.group) || a.order - b.order,
  );
}

export function pageHref(id: string): string {
  return id === HOME ? "/" : `/${id}/`;
}

export function grouped<P extends PageMeta>(sorted: readonly P[]): { group: Group; pages: P[] }[] {
  return GROUPS.map((group) => ({ group, pages: sorted.filter((p) => p.group === group) })).filter(
    (g) => g.pages.length > 0,
  );
}

function cliVersion(cargoToml: string): string {
  const table = cargoToml.split(/^\[/m).find((t) => t.startsWith("workspace.package]"));
  const m = table?.match(/^version\s*=\s*"([^"]+)"/m);
  if (!m) throw new Error("version not found in [workspace.package] of Cargo.toml");
  return m[1];
}

export function readCliVersion(): string {
  return cliVersion(readFileSync(resolve(process.cwd(), "../Cargo.toml"), "utf8"));
}
