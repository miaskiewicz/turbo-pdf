// @react-pdf/renderer adapter (Yoga layout, "stay in JS" competitor). No browser.
// Builds the document tree via createElement so this stays a .ts file (no JSX).

import { createElement } from "react";
import type { Availability, EngineAdapter, Footprint, RowData, Workload } from "../types.ts";
import { FONTS, FONT_FAMILY } from "../workloads/fonts.ts";
import { detectPackage, jsFootprint, nowMs, tryImport } from "./util.ts";

interface ReactPdfModule {
  Document: unknown;
  Page: unknown;
  View: unknown;
  Text: unknown;
  Font: { register(opts: { family: string; src: string }): void };
  renderToBuffer(node: unknown): Promise<Buffer>;
}

// Loosely-typed createElement: @react-pdf primitives are opaque to us, and we only
// need a tree of nodes, so children type-checking adds no safety here.
function h(type: unknown, props: unknown, ...children: unknown[]): unknown {
  return createElement(type as never, props as never, ...(children as never[]));
}

function rowNode(rpdf: ReactPdfModule, r: RowData): unknown {
  const cell = (s: string) => h(rpdf.Text, { key: s }, s);
  return h(rpdf.View, { key: r.id, style: { flexDirection: "row" } }, [
    cell(`${r.id}`),
    cell(r.description),
    cell(`${r.qty}`),
    cell(r.unit.toFixed(2)),
    cell(r.amount.toFixed(2)),
  ]);
}

function docNode(rpdf: ReactPdfModule, w: Workload): unknown {
  const rows = w.content.rows.map((r) => rowNode(rpdf, r));
  const paras = w.content.paragraphs.map((p, i) => h(rpdf.Text, { key: `p${i}` }, p));
  const body = h(rpdf.View, {}, [
    h(rpdf.Text, { key: "title", style: { fontSize: 20 } }, w.content.title),
    h(rpdf.Text, { key: "sub", style: { fontSize: 10, color: "#666" } }, w.content.subtitle),
    ...rows,
    ...paras,
  ]);
  const page = h(
    rpdf.Page,
    { size: "A4", style: { fontFamily: FONT_FAMILY, fontSize: 9, padding: w.geometry.marginPt } },
    body,
  );
  return h(rpdf.Document, {}, page);
}

export class ReactPdfAdapter implements EngineAdapter {
  readonly id = "react-pdf";
  readonly kind = "react" as const;
  private mod: ReactPdfModule | null = null;

  async detect(): Promise<Availability> {
    return detectPackage("@react-pdf/renderer");
  }

  footprint(): Footprint {
    return jsFootprint("React + Yoga (flexbox) layout; no HTML/CSS, no browser");
  }

  private async load(): Promise<ReactPdfModule> {
    if (this.mod) return this.mod;
    const mod = await tryImport<ReactPdfModule>("@react-pdf/renderer");
    if (!mod) throw new Error("@react-pdf/renderer not installed");
    mod.Font.register({ family: FONT_FAMILY, src: FONTS.regular });
    this.mod = mod;
    return mod;
  }

  async renderCold(w: Workload) {
    const start = nowMs();
    const rpdf = await this.load();
    const initMs = nowMs() - start;
    const t = nowMs();
    const buf = await rpdf.renderToBuffer(docNode(rpdf, w));
    return { pdfBytes: new Uint8Array(buf), timings: { renderMs: nowMs() - t, initMs } };
  }

  async renderWarm(w: Workload) {
    const rpdf = await this.load();
    const t = nowMs();
    const buf = await rpdf.renderToBuffer(docNode(rpdf, w));
    return { pdfBytes: new Uint8Array(buf), timings: { renderMs: nowMs() - t } };
  }

  async dispose(): Promise<void> {
    this.mod = null;
  }
}
