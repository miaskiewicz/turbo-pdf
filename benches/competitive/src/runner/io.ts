// Output IO: write the raw JSON artifact + the regenerated RESULTS.md. The results/
// dir is gitignored at the repo root (/benches/**/results/); RESULTS.md is committed.

import { mkdir, writeFile } from "node:fs/promises";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { renderMarkdown } from "./report.ts";
import type { ResultsDoc } from "./results.ts";

const HERE = dirname(fileURLToPath(import.meta.url));
const ROOT = resolve(HERE, "../..");

/** Write results.json (gitignored) and RESULTS.md (committed). Returns paths. */
export async function writeOutputs(doc: ResultsDoc): Promise<{ json: string; md: string }> {
  const resultsDir = join(ROOT, "results");
  await mkdir(resultsDir, { recursive: true });
  const json = join(resultsDir, "results.json");
  const md = join(ROOT, "RESULTS.md");
  await writeFile(json, `${JSON.stringify(doc, null, 2)}\n`);
  await writeFile(md, renderMarkdown(doc));
  return { json, md };
}
