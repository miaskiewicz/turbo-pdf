// Shared conformance harness: locate + load the BUILT binding packages, expose a
// uniform `Binding` adapter so one capability assertion runs against both napi
// and wasm, and provide structural PDF checks (plus an optional qpdf --check).
//
// The whole point of this suite is to assert against the *shipped* packages, not
// the core crate: if a binding silently drops a capability, loading its real
// entry point and exercising it here turns that into a hard failure.

import { execFileSync } from "node:child_process";
import { existsSync, readFileSync, writeFileSync } from "node:fs";
import { createRequire } from "node:module";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { makePng } from "./png.mjs";

const require = createRequire(import.meta.url);
const here = dirname(fileURLToPath(import.meta.url));
export const repoRoot = join(here, "..");

const NAPI_DIR = join(repoRoot, "crates", "turbo-pdf-napi");
const WASM_PKG = join(repoRoot, "crates", "turbo-pdf-wasm", "pkg-node", "turbo_pdf_wasm.js");
const FONT_PATH = join(NAPI_DIR, "assets", "fonts", "Go-Regular.ttf");

export const FONT_BYTES = readFileSync(FONT_PATH);
export const PNG_BYTES = makePng();

// A small page so multi-line content paginates cheaply.
export const CSS = "@page { size: 320px 240px; margin: 24px } p { font-size: 13px }";

/** True when `qpdf` is on PATH (then outputs are additionally `--check`ed). */
export function qpdfAvailable() {
  try {
    execFileSync("qpdf", ["--version"], { stdio: "ignore" });
    return true;
  } catch {
    return false;
  }
}

/** Assert `pdf` (Buffer | Uint8Array) is structurally a PDF; run qpdf if present. */
export function assertValidPdf(assert, pdf, label) {
  const buf = Buffer.isBuffer(pdf) ? pdf : Buffer.from(pdf);
  assert.ok(buf.length > 0, `${label}: pdf is non-empty`);
  assert.equal(buf.subarray(0, 5).toString("latin1"), "%PDF-", `${label}: PDF magic`);
  assert.ok(buf.subarray(-1024).toString("latin1").includes("%%EOF"), `${label}: %%EOF trailer`);
  if (qpdfAvailable()) {
    const path = join(tmpdir(), `turbo-pdf-conformance-${label.replace(/\W+/g, "_")}.pdf`);
    writeFileSync(path, buf);
    execFileSync("qpdf", ["--check", path], { stdio: "ignore" }); // throws on structural error
  }
  return buf;
}

/** Count `/Subtype /Image` XObjects in a PDF body (image-embedding probe). */
export function countImageXObjects(pdf) {
  const buf = Buffer.isBuffer(pdf) ? pdf : Buffer.from(pdf);
  const s = buf.toString("latin1");
  const m = s.match(/\/Subtype\s*\/Image/g);
  return m ? m.length : 0;
}

// --- napi adapter ------------------------------------------------------------
// Fonts cross as raw Buffer[]; the one-shot `render(html, opts, fonts?)` exists;
// the warm handle is `Fonts.load(Buffer[])`.
function napiBinding() {
  if (!existsSync(join(NAPI_DIR, "index.js"))) return null;
  let lib;
  try {
    lib = require(join(NAPI_DIR, "index.js"));
  } catch {
    return null; // native addon not built — caller reports a skip
  }
  return {
    name: "napi",
    lib,
    compile: (html, opts) => lib.compile(html, opts),
    // Render through a freshly compiled program (exercises program.render).
    render(html, opts) {
      const { fonts, fontsHandle, ...rest } = opts ?? {};
      const program = lib.compile(html, rest.compileOpts);
      const renderOpts = { ...rest, fonts: (fonts ?? []).map((f) => f.data) };
      return fontsHandle ? program.render(renderOpts, fontsHandle) : program.render(renderOpts);
    },
    oneShot(html, opts) {
      const { fonts, ...rest } = opts ?? {};
      return lib.render(html, { ...rest, fonts: (fonts ?? []).map((f) => f.data) });
    },
    loadFonts: (fonts) => lib.Fonts.load(fonts.map((f) => f.data)),
    pdfOf: (result) => result.pdf,
    pageCountOf: (result) => result.pageCount,
    diagnosticsOf: (result) => result.diagnostics,
  };
}

// --- wasm adapter ------------------------------------------------------------
// Fonts cross as { data: Uint8Array, family, weight?, italic? }; the warm path is
// `program.renderWithFonts(args, handle)`; there is no one-shot `render`.
function wasmBinding() {
  if (!existsSync(WASM_PKG)) return null;
  let wasm;
  try {
    wasm = require(WASM_PKG);
  } catch {
    return null; // pkg not built — caller reports a skip
  }
  const toFace = (f) => ({
    data: new Uint8Array(f.data),
    family: f.family ?? "conformance",
    weight: f.weight ?? 400,
    italic: f.italic ?? false,
  });
  return {
    name: "wasm",
    lib: wasm,
    compile: (html, opts) => wasm.compile(html, opts),
    render(html, opts) {
      const { fonts, fontsHandle, compileOpts, ...rest } = opts ?? {};
      const program = wasm.compile(html, compileOpts);
      const args = { ...rest, fonts: (fonts ?? []).map(toFace) };
      return fontsHandle ? program.renderWithFonts(args, fontsHandle) : program.render(args);
    },
    oneShot: null, // wasm exposes no one-shot render — documented gap, asserted as skip
    loadFonts: (fonts) => wasm.Fonts.load(fonts.map(toFace)),
    pdfOf: (result) => result.pdf,
    pageCountOf: (result) => result.pageCount,
    diagnosticsOf: (result) => result.diagnostics,
  };
}

/**
 * The bindings under test, each as a uniform adapter (or `null` when not built).
 * Returns `[{ id, binding|null, reason }]`.
 */
export function loadBindings() {
  const napi = napiBinding();
  const wasm = wasmBinding();
  return [
    {
      id: "napi",
      binding: napi,
      reason: napi
        ? null
        : "napi addon not built — run `cargo build -p turbo-pdf-napi --release` " +
          "then `node crates/turbo-pdf-napi/scripts/copy-addon.mjs`",
    },
    {
      id: "wasm",
      binding: wasm,
      reason: wasm
        ? null
        : "wasm pkg not built — run `wasm-pack build crates/turbo-pdf-wasm " +
          "--target nodejs --out-dir pkg-node`",
    },
  ];
}
