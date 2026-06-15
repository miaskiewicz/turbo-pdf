# Binding conformance suite

This suite proves that the **shipped** turbo-html2pdf bindings actually expose
the engine's capabilities — not merely that the core Rust crate has them.

It does that by loading each binding's **real built entry point**:

- napi: `crates/turbo-pdf-napi/index.js` (requires the native `.node` addon)
- wasm: `crates/turbo-pdf-wasm/pkg-node/turbo_pdf_wasm.js` (a `wasm-pack --target nodejs` build)

and running one shared **capability matrix** against both. If a binding silently
drops a capability (forgets to wire a render option, regresses a field name,
returns the wrong shape), the matching row throws and CI goes red — even though
the core crate still passes its own tests.

## What it guarantees

For every binding that is built, every **active** matrix row is exercised:

- `compile(html)` → `program.render(...)` returns a real `%PDF-…%%EOF` document
- the render result shape (`{ pdf, pageCount, diagnostics }`)
- Jinja control flow renders interpolated data
- `@page` geometry + overflow paginates to multiple pages
- per-call fonts and the warm `Fonts.load(...)` handle both work (and the warm
  path is byte-deterministic)
- `meta.title` lands in the PDF Info dictionary
- `program.hasHeader()` / `hasFooter()` report declared running regions
- identical inputs (pinned `now`) produce byte-identical PDFs
- a fatal template fault throws a structured error (code + span)
- non-fatal lints come back in `diagnostics` (and don't throw)
- napi-only: the one-shot `render(html, opts)`
- wasm-only: `compile` opts `{ missingPolicy }` are honored

When `qpdf` is on `PATH`, every emitted PDF is additionally `qpdf --check`ed;
otherwise outputs are validated structurally (`%PDF-` header, `%%EOF` trailer,
object-count regexes), so the suite runs with or without qpdf.

A binding that is **not built** is reported as a skip (with a "how to build"
message), not silently passed. CI builds both packages first, so on CI every
active row must pass.

## The capability matrix

`matrix.mjs` is the single source of truth. Each row is one capability with one
assertion, run against **every** built binding. Rows marked `status: "skip"`
document the **intended-but-not-yet-exposed** surface so the target API is
visible and trivially un-skippable later:

| row id        | status | note                                                |
| ------------- | ------ | --------------------------------------------------- |
| named-images  | skip   | image bytes cross the boundary but are dropped (Phase 9b); no name-keyed resolver yet |
| watermark     | skip   | no watermark render option exposed                  |
| append-pages  | skip   | no PDF append/merge exposed                          |
| encrypt       | skip   | no encryption exposed                                |
| pdf-a         | skip   | no PDF/A output exposed                              |
| pdf-ua        | skip   | no PDF/UA output exposed                             |
| cmyk          | skip   | no CMYK output exposed                               |

## How to add a capability row

1. Add one object to the `MATRIX` array in `matrix.mjs`:

   ```js
   {
     id: "my-capability",                 // unique slug
     capability: "human-readable name",   // shown in test output
     status: "active",
     // only: ["napi"],                    // optional: binding-specific capability
     assert({ binding, assert }) {
       const res = binding.render("<p>{{ x }}</p>", {
         css: h.CSS, now: 0, data: { x: "hi" }, fonts: [{ data: h.FONT_BYTES, family: "Go" }],
       });
       h.assertValidPdf(assert, binding.pdfOf(res), "my-capability");
     },
   }
   ```

   Use the `binding` adapter (`render`, `oneShot`, `compile`, `loadFonts`,
   `pdfOf`, `pageCountOf`, `diagnosticsOf`) so the same row drives both bindings.
   Fonts are always passed as `{ data, family?, weight?, italic? }`; each adapter
   lowers that to its own wire shape (napi `Buffer[]`, wasm `{ data, … }`).

2. That's it — the runner (`conformance.test.mjs`) iterates the matrix against
   every built binding automatically. To un-skip a placeholder, flip its
   `status` to `"active"` and fill in the assertion.

## Running locally

```sh
# build both packages, then run (from the repo root)
pnpm --filter @turbo-pdf/conformance conformance

# or step by step
cargo build -p turbo-pdf-napi --release
node crates/turbo-pdf-napi/scripts/copy-addon.mjs
wasm-pack build crates/turbo-pdf-wasm --target nodejs --out-dir pkg-node
node --test conformance/conformance.test.mjs
```

## Files

- `matrix.mjs` — the capability matrix (add rows here)
- `harness.mjs` — loads the built bindings as uniform adapters; PDF/qpdf checks
- `png.mjs` — synthesizes a real PNG in-process (no fixture files)
- `conformance.test.mjs` — the `node:test` runner that wires the matrix in
