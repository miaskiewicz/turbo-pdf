// jsPDF (+ jspdf-autotable) adapter. Draw-API engine, no browser. Uses the bundled
// font via addFileToVFS so glyph coverage matches the other engines.

import { readFileSync } from "node:fs";
import type { Availability, EngineAdapter, Footprint, RenderResult, Workload } from "../types.ts";
import { FONTS, FONT_FAMILY } from "../workloads/fonts.ts";
import { detectPackage, jsFootprint, nowMs, tryImport } from "./util.ts";

interface JsPdfDoc {
  addFileToVFS(name: string, b64: string): void;
  addFont(name: string, family: string, style: string): void;
  setFont(family: string, style?: string): void;
  setFontSize(n: number): void;
  text(s: string, x: number, y: number): void;
  autoTable(opts: Record<string, unknown>): void;
  output(kind: "arraybuffer"): ArrayBuffer;
}
interface JsPdfCtor {
  jsPDF: new (opts: Record<string, unknown>) => JsPdfDoc;
}
/** jspdf-autotable v3 attaches `autoTable` onto jsPDF instances via applyPlugin. */
interface AutoTablePlugin {
  applyPlugin(ctor: unknown): void;
}

function registerFont(doc: JsPdfDoc): void {
  const reg = readFileSync(FONTS.regular).toString("base64");
  doc.addFileToVFS("Evolventa.ttf", reg);
  doc.addFont("Evolventa.ttf", FONT_FAMILY, "normal");
}

function rowsForTable(w: Workload): string[][] {
  return w.content.rows.map((r) => [
    String(r.id),
    r.description,
    String(r.qty),
    r.unit.toFixed(2),
    r.amount.toFixed(2),
  ]);
}

function renderOnce(ctor: JsPdfCtor, w: Workload): Uint8Array {
  const doc = new ctor.jsPDF({ unit: "pt", format: "a4" });
  registerFont(doc);
  doc.setFont(FONT_FAMILY);
  doc.setFontSize(20);
  doc.text(w.content.title, w.geometry.marginPt, w.geometry.marginPt + 10);
  doc.setFontSize(10);
  doc.text(w.content.subtitle, w.geometry.marginPt, w.geometry.marginPt + 28);
  if (w.content.rows.length > 0) {
    doc.autoTable({
      head: [["#", "Description", "Qty", "Unit", "Amount"]],
      body: rowsForTable(w),
      startY: w.geometry.marginPt + 40,
      styles: { font: FONT_FAMILY, fontSize: 9 },
    });
  }
  return new Uint8Array(doc.output("arraybuffer"));
}

export class JspdfAdapter implements EngineAdapter {
  readonly id = "jspdf";
  readonly kind = "draw-api" as const;
  private ctor: JsPdfCtor | null = null;

  async detect(): Promise<Availability> {
    return detectPackage("jspdf");
  }

  footprint(): Footprint {
    return jsFootprint("browser/Node draw API; autotable plugin for tables");
  }

  private async load(): Promise<JsPdfCtor> {
    if (this.ctor) return this.ctor;
    const mod = await tryImport<JsPdfCtor>("jspdf");
    const at = await tryImport<AutoTablePlugin>("jspdf-autotable");
    if (!mod || !at) throw new Error("jspdf/jspdf-autotable not installed");
    at.applyPlugin(mod.jsPDF);
    this.ctor = mod;
    return mod;
  }

  async renderCold(w: Workload): Promise<RenderResult> {
    const start = nowMs();
    const ctor = await this.load();
    const initMs = nowMs() - start;
    const t = nowMs();
    const pdfBytes = renderOnce(ctor, w);
    return { pdfBytes, timings: { renderMs: nowMs() - t, initMs } };
  }

  async renderWarm(w: Workload): Promise<RenderResult> {
    const ctor = await this.load();
    const t = nowMs();
    const pdfBytes = renderOnce(ctor, w);
    return { pdfBytes, timings: { renderMs: nowMs() - t } };
  }

  async dispose(): Promise<void> {
    this.ctor = null;
  }
}
