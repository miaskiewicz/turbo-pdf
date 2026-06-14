// Typst adapter (`typst compile`). Typesetting subprocess. Skips if the binary is
// not on PATH. Fonts supplied via --font-path so glyphs match the other engines.

import type { Availability, EngineAdapter, Footprint, RenderResult, Workload } from "../types.ts";
import { FONT_DIR } from "../workloads/fonts.ts";
import { renderTypst } from "../workloads/typst.ts";
import { detectBinary, nowMs } from "./util.ts";
import { runCliEngine } from "./subprocess.ts";

function compile(w: Workload): Promise<Uint8Array> {
  return runCliEngine({
    bin: "typst",
    inputName: "doc.typ",
    inputData: renderTypst(w),
    outputName: "out.pdf",
    argv: (input, output) => ["compile", "--font-path", FONT_DIR, input, output],
  });
}

export class TypstAdapter implements EngineAdapter {
  readonly id = "typst";
  readonly kind = "typesetting" as const;

  detect(): Promise<Availability> {
    return detectBinary("typst", ["--version"]);
  }

  footprint(): Footprint {
    return {
      installedBytes: null,
      shipsBrowser: false,
      notes: "single self-contained Rust binary (~30MB); no browser",
    };
  }

  async renderCold(w: Workload): Promise<RenderResult> {
    const t = nowMs();
    const pdfBytes = await compile(w);
    // Each `typst compile` is a fresh process: cold == warm for this engine.
    return { pdfBytes, timings: { renderMs: nowMs() - t, initMs: 0 } };
  }

  renderWarm(w: Workload): Promise<RenderResult> {
    return this.renderCold(w);
  }

  async dispose(): Promise<void> {
    // No pooled state.
  }
}
