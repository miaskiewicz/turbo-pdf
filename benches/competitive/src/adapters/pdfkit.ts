// PDFKit adapter (Node draw-API). Installed → exercisable directly. No browser.

import type {
  Availability,
  EngineAdapter,
  Footprint,
  RenderResult,
  RowData,
  Workload,
} from "../types.ts";
import { FONTS } from "../workloads/fonts.ts";
import { detectPackage, jsFootprint, nowMs, tryImport } from "./util.ts";

interface PdfDoc {
  font(path: string): PdfDoc;
  fontSize(n: number): PdfDoc;
  text(s: string, opts?: Record<string, unknown>): PdfDoc;
  moveDown(n?: number): PdfDoc;
  on(ev: string, cb: (chunk: Buffer) => void): PdfDoc;
  end(): void;
}
type PdfCtor = new (opts: Record<string, unknown>) => PdfDoc;

function drawRow(doc: PdfDoc, r: RowData): void {
  doc.text(`${r.id}  ${r.description}  ${r.qty} x ${r.unit.toFixed(2)} = ${r.amount.toFixed(2)}`);
}

function drawBody(doc: PdfDoc, w: Workload): void {
  doc.font(FONTS.bold).fontSize(20).text(w.content.title);
  doc.font(FONTS.regular).fontSize(10).text(w.content.subtitle).moveDown(0.5);
  doc.fontSize(9);
  for (const r of w.content.rows) drawRow(doc, r);
  for (const p of w.content.paragraphs) doc.moveDown(0.3).text(p);
  for (let i = 0; i < w.content.footnotes.length; i++) {
    doc.fontSize(7.5).text(`${i + 1}. ${w.content.footnotes[i] ?? ""}`);
  }
}

function renderOnce(Ctor: PdfCtor, w: Workload): Promise<Uint8Array> {
  return new Promise((res) => {
    const doc = new Ctor({
      size: w.geometry.size,
      margin: w.geometry.marginPt,
      autoFirstPage: true,
    });
    const chunks: Buffer[] = [];
    doc.on("data", (c) => chunks.push(c));
    doc.on("end", () => res(new Uint8Array(Buffer.concat(chunks))));
    drawBody(doc, w);
    doc.end();
  });
}

export class PdfkitAdapter implements EngineAdapter {
  readonly id = "pdfkit";
  readonly kind = "draw-api" as const;
  private ctor: PdfCtor | null = null;

  async detect(): Promise<Availability> {
    return detectPackage("pdfkit");
  }

  footprint(): Footprint {
    return jsFootprint("pure-JS draw API; manual layout, no HTML/CSS engine");
  }

  private async load(): Promise<PdfCtor> {
    if (this.ctor) return this.ctor;
    const mod = await tryImport<{ default: PdfCtor }>("pdfkit");
    if (!mod) throw new Error("pdfkit not installed");
    this.ctor = mod.default;
    return this.ctor;
  }

  async renderCold(w: Workload): Promise<RenderResult> {
    const start = nowMs();
    const Ctor = await this.load();
    const initMs = nowMs() - start;
    const t = nowMs();
    const pdfBytes = await renderOnce(Ctor, w);
    return { pdfBytes, timings: { renderMs: nowMs() - t, initMs } };
  }

  async renderWarm(w: Workload): Promise<RenderResult> {
    const Ctor = await this.load();
    const t = nowMs();
    const pdfBytes = await renderOnce(Ctor, w);
    return { pdfBytes, timings: { renderMs: nowMs() - t } };
  }

  async dispose(): Promise<void> {
    this.ctor = null;
  }
}
