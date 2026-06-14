// Puppeteer adapter (headless Chromium → page.pdf()). The fidelity baseline.
// Cold = launch a browser per render (the real "Chromium launch is the killer"
// cost, AC-10.9). Warm = reuse one pooled browser across renders.

import type { Availability, EngineAdapter, Footprint, RenderResult, Workload } from "../types.ts";
import { renderHtml } from "../workloads/html.ts";
import { detectPackage, nowMs, tryImport } from "./util.ts";

interface Page {
  setContent(html: string, opts: Record<string, unknown>): Promise<void>;
  pdf(opts: Record<string, unknown>): Promise<Uint8Array>;
  close(): Promise<void>;
}
interface Browser {
  newPage(): Promise<Page>;
  close(): Promise<void>;
  version(): Promise<string>;
}
interface Puppeteer {
  launch(opts: Record<string, unknown>): Promise<Browser>;
}

const PDF_OPTS = { printBackground: true, preferCSSPageSize: true };

async function renderWith(browser: Browser, w: Workload): Promise<Uint8Array> {
  const page = await browser.newPage();
  await page.setContent(renderHtml(w), { waitUntil: "load" });
  const bytes = await page.pdf(PDF_OPTS);
  await page.close();
  return bytes;
}

export class PuppeteerAdapter implements EngineAdapter {
  readonly id = "puppeteer";
  readonly kind = "browser" as const;
  private pup: Puppeteer | null = null;
  private pool: Browser | null = null;

  async detect(): Promise<Availability> {
    const pkg = await detectPackage("puppeteer");
    if (!pkg.available) return pkg;
    const mod = await tryImport<Puppeteer>("puppeteer");
    if (!mod) return { available: false, version: null, reason: "puppeteer import failed" };
    try {
      const b = await mod.launch({ headless: true });
      const version = await b.version();
      await b.close();
      return { available: true, version, reason: null };
    } catch (err) {
      const reason = `chromium launch failed: ${(err as Error).message}`;
      return { available: false, version: pkg.version, reason };
    }
  }

  footprint(): Footprint {
    return {
      installedBytes: null,
      shipsBrowser: true,
      notes: "downloads a full Chromium (~150-300MB) on install",
    };
  }

  private async load(): Promise<Puppeteer> {
    if (this.pup) return this.pup;
    const mod = await tryImport<Puppeteer>("puppeteer");
    if (!mod) throw new Error("puppeteer not installed");
    this.pup = mod;
    return mod;
  }

  async renderCold(w: Workload): Promise<RenderResult> {
    const start = nowMs();
    const pup = await this.load();
    const browser = await pup.launch({ headless: true });
    const initMs = nowMs() - start;
    const t = nowMs();
    const pdfBytes = await renderWith(browser, w);
    const renderMs = nowMs() - t;
    await browser.close();
    return { pdfBytes, timings: { renderMs, initMs } };
  }

  async renderWarm(w: Workload): Promise<RenderResult> {
    const pup = await this.load();
    if (!this.pool) this.pool = await pup.launch({ headless: true });
    const t = nowMs();
    const pdfBytes = await renderWith(this.pool, w);
    return { pdfBytes, timings: { renderMs: nowMs() - t } };
  }

  async dispose(): Promise<void> {
    if (this.pool) await this.pool.close();
    this.pool = null;
  }
}
