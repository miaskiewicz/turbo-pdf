// Gotenberg adapter (Chromium-in-a-container via HTTP). Detects a running Gotenberg
// instance (default http://localhost:3000, override with GOTENBERG_URL). Does NOT
// pull or start any docker image itself — the README documents the one-liner.

import type { Availability, EngineAdapter, Footprint, RenderResult, Workload } from "../types.ts";
import { renderHtml } from "../workloads/html.ts";
import { nowMs } from "./util.ts";

const BASE = process.env.GOTENBERG_URL ?? "http://localhost:3000";
const ROUTE = "/forms/chromium/convert/html";

async function probe(url: string): Promise<Availability> {
  try {
    const res = await fetch(`${url}/health`, { signal: AbortSignal.timeout(2000) });
    if (res.ok) return { available: true, version: url, reason: null };
    return { available: false, version: null, reason: `health ${res.status}` };
  } catch {
    return {
      available: false,
      version: null,
      reason: `no Gotenberg at ${url} (run: docker run --rm -p 3000:3000 gotenberg/gotenberg:8)`,
    };
  }
}

async function convert(w: Workload): Promise<Uint8Array> {
  const form = new FormData();
  form.append("files", new Blob([renderHtml(w)], { type: "text/html" }), "index.html");
  const res = await fetch(`${BASE}${ROUTE}`, { method: "POST", body: form });
  if (!res.ok) throw new Error(`gotenberg ${res.status}`);
  return new Uint8Array(await res.arrayBuffer());
}

export class GotenbergAdapter implements EngineAdapter {
  readonly id = "gotenberg";
  readonly kind = "browser" as const;

  detect(): Promise<Availability> {
    return probe(BASE);
  }

  footprint(): Footprint {
    return {
      installedBytes: null,
      shipsBrowser: true,
      notes: "docker image bundling Chromium + LibreOffice (~1GB); HTTP service",
    };
  }

  async renderCold(w: Workload): Promise<RenderResult> {
    // Service is already warm; "cold" here is request init + render over HTTP.
    const t = nowMs();
    const pdfBytes = await convert(w);
    return { pdfBytes, timings: { renderMs: nowMs() - t, initMs: 0 } };
  }

  async renderWarm(w: Workload): Promise<RenderResult> {
    const t = nowMs();
    const pdfBytes = await convert(w);
    return { pdfBytes, timings: { renderMs: nowMs() - t } };
  }

  async dispose(): Promise<void> {
    // Container lifecycle is external.
  }
}
