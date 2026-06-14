// Orchestrator: detect engines, measure each available engine over every requested
// workload, score PNG equivalence against the reference, assemble the ResultsDoc.

import type { EngineAdapter, Workload, WorkloadId } from "../types.ts";
import { allAdapters } from "../adapters/index.ts";
import { getWorkload } from "../workloads/index.ts";
import { similarityOf } from "../compare/diff.ts";
import { buildContext, type CliOpts } from "./context.ts";
import { measure } from "./measure.ts";
import type { Cell, EngineInfo, ResultsDoc } from "./results.ts";

export interface RunOpts extends CliOpts {
  readonly workloads: readonly WorkloadId[];
}

function skippedCell(engine: EngineAdapter, w: WorkloadId, error: string): Cell {
  return {
    engine: engine.id,
    workload: w,
    cold: null,
    warm: null,
    initMs: null,
    pdfBytes: null,
    peakRssBytes: null,
    throughputPerSec: null,
    similarity: null,
    error,
  };
}

async function lastWarmPdf(engine: EngineAdapter, w: Workload): Promise<Uint8Array | null> {
  try {
    return (await engine.renderWarm(w)).pdfBytes;
  } catch {
    return null;
  }
}

async function measureCell(engine: EngineAdapter, w: Workload, opts: RunOpts): Promise<Cell> {
  const m = await measure(engine, w, opts);
  return {
    engine: engine.id,
    workload: w.id,
    cold: m.cold,
    warm: m.warm,
    initMs: m.initMs,
    pdfBytes: m.pdfBytes,
    peakRssBytes: m.peakRssBytes,
    throughputPerSec: m.throughputPerSec,
    similarity: null,
    error: null,
  };
}

async function runEngine(engine: EngineAdapter, opts: RunOpts): Promise<Cell[]> {
  const out: Cell[] = [];
  for (const id of opts.workloads) {
    const w = getWorkload(id);
    try {
      out.push(await measureCell(engine, w, opts));
    } catch (err) {
      out.push(skippedCell(engine, id, (err as Error).message));
    }
  }
  await engine.dispose();
  return out;
}

async function detectAll(engines: readonly EngineAdapter[]): Promise<EngineInfo[]> {
  const infos: EngineInfo[] = [];
  for (const e of engines) {
    infos.push({
      id: e.id,
      kind: e.kind,
      availability: await e.detect(),
      footprint: e.footprint(),
    });
  }
  return infos;
}

function refPdfFor(pdfs: Map<string, Uint8Array>, id: WorkloadId, ref: string) {
  return pdfs.get(`${ref}:${id}`) ?? null;
}

async function scoreEquivalence(
  cells: Cell[],
  pdfs: Map<string, Uint8Array>,
  ref: string,
): Promise<Cell[]> {
  const scored: Cell[] = [];
  for (const c of cells) {
    const candidate = pdfs.get(`${c.engine}:${c.workload}`);
    const reference = refPdfFor(pdfs, c.workload, ref);
    const usable = candidate && reference && c.engine !== ref;
    const r = usable ? await similarityOf(candidate, reference) : { similarity: null };
    scored.push({ ...c, similarity: r.similarity });
  }
  return scored;
}

async function collectPdfs(engines: EngineAdapter[], infos: EngineInfo[], opts: RunOpts) {
  const pdfs = new Map<string, Uint8Array>();
  for (const e of engines) {
    const info = infos.find((i) => i.id === e.id);
    if (!info?.availability.available) continue;
    for (const id of opts.workloads) {
      const pdf = await lastWarmPdf(e, getWorkload(id));
      if (pdf) pdfs.set(`${e.id}:${id}`, pdf);
    }
    await e.dispose();
  }
  return pdfs;
}

/** Run the full matrix and return the assembled results document. */
export async function runMatrix(opts: RunOpts): Promise<ResultsDoc> {
  const engines = allAdapters();
  const infos = await detectAll(engines);
  const cells: Cell[] = [];
  for (const e of engines) {
    const info = infos.find((i) => i.id === e.id);
    if (!info?.availability.available) {
      for (const id of opts.workloads)
        cells.push(skippedCell(e, id, info?.availability.reason ?? "unavailable"));
      continue;
    }
    cells.push(...(await runEngine(e, opts)));
  }
  const pdfs = await collectPdfs(allAdapters(), infos, opts);
  const scored = await scoreEquivalence(cells, pdfs, opts.referenceEngine);
  return { context: buildContext(opts), engines: infos, cells: scored };
}
