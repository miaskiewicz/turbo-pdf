// CLI entrypoint for the competitive benchmark harness.
//
//   node --experimental-strip-types src/cli.ts [options]
//     --runs <n>        measured runs per (engine × workload)   (default 5; full: 100)
//     --warmup <n>      warmup runs before measuring            (default 1)
//     --workloads <ids> comma list (default: all)
//     --reference <id>  engine used as PNG-equivalence baseline (default: pdfkit)
//
// Defaults are a quick smoke run. Use `pnpm bench:full` for ≥100 runs (spec §10.2).

import { WORKLOAD_IDS } from "./workloads/index.ts";
import type { WorkloadId } from "./types.ts";
import { runMatrix } from "./runner/run.ts";
import { writeOutputs } from "./runner/io.ts";

interface Parsed {
  runs: number;
  warmup: number;
  workloads: WorkloadId[];
  reference: string;
}

const VALID = new Set<string>(WORKLOAD_IDS);

function parseWorkloads(csv: string): WorkloadId[] {
  const ids = csv
    .split(",")
    .map((s) => s.trim())
    .filter(Boolean);
  const bad = ids.filter((id) => !VALID.has(id));
  if (bad.length > 0) throw new Error(`unknown workload(s): ${bad.join(", ")}`);
  return ids as WorkloadId[];
}

function applyArg(opts: Parsed, flag: string, value: string): void {
  if (flag === "--runs") opts.runs = Number(value);
  else if (flag === "--warmup") opts.warmup = Number(value);
  else if (flag === "--workloads") opts.workloads = parseWorkloads(value);
  else if (flag === "--reference") opts.reference = value;
  else throw new Error(`unknown flag: ${flag}`);
}

function parseArgs(argv: readonly string[]): Parsed {
  const opts: Parsed = { runs: 5, warmup: 1, workloads: [...WORKLOAD_IDS], reference: "pdfkit" };
  for (let i = 0; i < argv.length; i += 2) {
    const flag = argv[i];
    const value = argv[i + 1];
    if (flag === undefined || value === undefined) throw new Error(`missing value for ${flag}`);
    applyArg(opts, flag, value);
  }
  return opts;
}

async function main(): Promise<void> {
  const opts = parseArgs(process.argv.slice(2));
  process.stdout.write(
    `bench: ${opts.workloads.length} workloads, ${opts.runs} runs, ref=${opts.reference}\n`,
  );
  const doc = await runMatrix({
    runs: opts.runs,
    warmup: opts.warmup,
    workloads: opts.workloads,
    referenceEngine: opts.reference,
  });
  const { json, md } = await writeOutputs(doc);
  const ran = doc.engines.filter((e) => e.availability.available).map((e) => e.id);
  process.stdout.write(`engines exercised: ${ran.join(", ") || "(none)"}\n`);
  process.stdout.write(`wrote ${json}\nwrote ${md}\n`);
}

main().catch((err: unknown) => {
  process.stderr.write(`bench failed: ${(err as Error).message}\n`);
  process.exitCode = 1;
});
