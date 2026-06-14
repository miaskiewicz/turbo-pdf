// Engine adapter registry. The runner iterates this list, detects availability, and
// skips gracefully whatever is missing — the harness never hard-fails on a missing
// engine (spec §10.2 harness rules).

import type { EngineAdapter } from "../types.ts";
import { GotenbergAdapter } from "./gotenberg.ts";
import { JspdfAdapter } from "./jspdf.ts";
import { PdfkitAdapter } from "./pdfkit.ts";
import { PlaywrightAdapter } from "./playwright.ts";
import { PuppeteerAdapter } from "./puppeteer.ts";
import { ReactPdfAdapter } from "./reactpdf.ts";
import { TurboPdfAdapter } from "./turbopdf.ts";
import { TypstAdapter } from "./typst.ts";
import { WeasyprintAdapter } from "./weasyprint.ts";
import { WkhtmltopdfAdapter } from "./wkhtmltopdf.ts";

/** Construct one fresh instance of every adapter. */
export function allAdapters(): EngineAdapter[] {
  return [
    new TurboPdfAdapter(),
    new PuppeteerAdapter(),
    new PlaywrightAdapter(),
    new GotenbergAdapter(),
    new ReactPdfAdapter(),
    new PdfkitAdapter(),
    new JspdfAdapter(),
    new TypstAdapter(),
    new WkhtmltopdfAdapter(),
    new WeasyprintAdapter(),
  ];
}
