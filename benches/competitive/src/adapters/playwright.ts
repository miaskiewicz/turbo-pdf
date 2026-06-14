// Playwright adapter (Chromium → page.pdf()). Same fidelity baseline as Puppeteer,
// different driver. Cold launches a browser per render; warm reuses one.

import type { Availability, EngineAdapter, Footprint, RenderResult, Workload } from "../types.ts";
import { renderHtml } from "../workloads/html.ts";
import { detectPackage, nowMs, tryImport } from "./util.ts";

interface Page {
  setContent(html: string, opts: Record<string, unknown>): Promise<void>;
  pdf(opts: Record<string, unknown>): Promise<Buffer>;
  close(): Promise<void>;
}
interface Browser {
  newPage(): Promise<Page>;
  close(): Promise<void>;
}
interface BrowserType {
  launch(opts: Record<string, unknown>): Promise<Browser>;
}
interface Playwright {
  chromium: BrowserType;
}

const PDF_OPTS = { printBackground: true, preferCSSPageSize: true };

async function renderWith(browser: Browser, w: Workload): Promise<Uint8Array> {
  const page = await browser.newPage();
  await page.setContent(renderHtml(w), { waitUntil: "load" });
  const bytes = await page.pdf(PDF_OPTS);
  await page.close();
  return new Uint8Array(bytes);
}

export class PlaywrightAdapter implements EngineAdapter {
  readonly id = "playwright";
  readonly kind = "browser" as const;
  private pw: Playwright | null = null;
  private pool: Browser | null = null;

  async detect(): Promise<Availability> {
    const pkg = await detectPackage("playwright");
    if (!pkg.available) return pkg;
    const mod = await tryImport<Playwright>("playwright");
    if (!mod) return { available: false, version: null, reason: "playwright import failed" };
    try {
      const b = await mod.chromium.launch({ headless: true });
      await b.close();
      return { available: true, version: pkg.version, reason: null };
    } catch (err) {
      const reason = `chromium launch failed (run: npx playwright install chromium): ${(err as Error).message}`;
      return { available: false, version: pkg.version, reason };
    }
  }

  footprint(): Footprint {
    return {
      installedBytes: null,
      shipsBrowser: true,
      notes: "installs Chromium via `playwright install` (~150-300MB)",
    };
  }

  private async load(): Promise<Playwright> {
    if (this.pw) return this.pw;
    const mod = await tryImport<Playwright>("playwright");
    if (!mod) throw new Error("playwright not installed");
    this.pw = mod;
    return mod;
  }

  async renderCold(w: Workload): Promise<RenderResult> {
    const start = nowMs();
    const pw = await this.load();
    const browser = await pw.chromium.launch({ headless: true });
    const initMs = nowMs() - start;
    const t = nowMs();
    const pdfBytes = await renderWith(browser, w);
    const renderMs = nowMs() - t;
    await browser.close();
    return { pdfBytes, timings: { renderMs, initMs } };
  }

  async renderWarm(w: Workload): Promise<RenderResult> {
    const pw = await this.load();
    if (!this.pool) this.pool = await pw.chromium.launch({ headless: true });
    const t = nowMs();
    const pdfBytes = await renderWith(this.pool, w);
    return { pdfBytes, timings: { renderMs: nowMs() - t } };
  }

  async dispose(): Promise<void> {
    if (this.pool) await this.pool.close();
    this.pool = null;
  }
}
