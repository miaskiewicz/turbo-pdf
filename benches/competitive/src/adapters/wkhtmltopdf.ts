// wkhtmltopdf adapter (legacy QtWebKit HTML→PDF reference). Skips if absent.
// Deprecated upstream; included only as a legacy comparison point per spec §10.2.

import type { Availability, EngineAdapter, Footprint, RenderResult, Workload } from "../types.ts";
import { renderHtml } from "../workloads/html.ts";
import { detectBinary, nowMs } from "./util.ts";
import { runCliEngine } from "./subprocess.ts";

function convert(w: Workload): Promise<Uint8Array> {
  return runCliEngine({
    bin: "wkhtmltopdf",
    inputName: "doc.html",
    inputData: renderHtml(w),
    outputName: "out.pdf",
    argv: (input, output) => ["--enable-local-file-access", "-q", input, output],
  });
}

export class WkhtmltopdfAdapter implements EngineAdapter {
  readonly id = "wkhtmltopdf";
  readonly kind = "reference" as const;

  detect(): Promise<Availability> {
    return detectBinary("wkhtmltopdf", ["--version"]);
  }

  footprint(): Footprint {
    return {
      installedBytes: null,
      shipsBrowser: true,
      notes: "bundles a patched QtWebKit (~50MB); legacy, unmaintained",
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
