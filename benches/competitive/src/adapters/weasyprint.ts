// WeasyPrint adapter (Python HTML/CSS→PDF). The closest architectural sibling and a
// correctness/speed reference (spec §10.2). Skips if the `weasyprint` CLI is absent.

import type { Availability, EngineAdapter, Footprint, RenderResult, Workload } from "../types.ts";
import { renderHtml } from "../workloads/html.ts";
import { detectBinary, nowMs } from "./util.ts";
import { runCliEngine } from "./subprocess.ts";

function convert(w: Workload): Promise<Uint8Array> {
  return runCliEngine({
    bin: "weasyprint",
    inputName: "doc.html",
    inputData: renderHtml(w),
    outputName: "out.pdf",
    argv: (input, output) => [input, output],
  });
}

export class WeasyprintAdapter implements EngineAdapter {
  readonly id = "weasyprint";
  readonly kind = "reference" as const;

  detect(): Promise<Availability> {
    return detectBinary("weasyprint", ["--version"]);
  }

  footprint(): Footprint {
    return {
      installedBytes: null,
      shipsBrowser: false,
      notes: "Python + Cairo/Pango stack; no browser, but heavy native deps",
    };
  }

  async renderCold(w: Workload): Promise<RenderResult> {
    const t = nowMs();
    const pdfBytes = await convert(w);
    return { pdfBytes, timings: { renderMs: nowMs() - t, initMs: 0 } };
  }

  renderWarm(w: Workload): Promise<RenderResult> {
    return this.renderCold(w);
  }

  async dispose(): Promise<void> {
    // No pooled state.
  }
}
