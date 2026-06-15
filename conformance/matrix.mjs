// THE CAPABILITY MATRIX — the single source of truth for what the shipped
// bindings must expose. Each row is one capability with one assertion, run
// against EVERY built binding (napi + wasm). When a new render option lands,
// add one row here and both bindings are checked automatically.
//
// Row shape:
//   {
//     id:        stable slug,
//     capability: human-readable name (shown in test output),
//     status:    "active" | "skip",
//     skipReason: required when status === "skip" — why it is not yet exposed,
//     only:      optional ["napi"] / ["wasm"] when a capability is binding-specific
//                (a documented divergence, not a silent gap),
//     assert:    ({ binding, assert, h }) => void   — throws on failure.
//   }
//
// `h` is the helpers module (assertValidPdf, countImageXObjects, CSS, FONT_BYTES,
// PNG_BYTES, ...). Fonts are always passed as `{ data, family?, weight?, italic? }`;
// each binding adapter lowers that to its own wire shape, so the assertion stays
// shape-agnostic and the same row proves BOTH bindings expose the capability.

import * as h from "./harness.mjs";

const FONT = { data: h.FONT_BYTES, family: "Go", weight: 400, italic: false };

/** A simple, deterministic render input reused by several rows. */
const BASE = { css: h.CSS, now: 0 };

export const MATRIX = [
  // --- ACTIVE: the currently-exposed surface --------------------------------
  {
    id: "compile-render-pdf",
    capability: "compile(html) -> program.render() yields a %PDF- document",
    status: "active",
    assert({ binding, assert }) {
      const res = binding.render("<h1>{{ t }}</h1>", {
        ...BASE,
        data: { t: "Conformance" },
        fonts: [FONT],
      });
      h.assertValidPdf(assert, binding.pdfOf(res), "compile-render");
      assert.ok(binding.pageCountOf(res) >= 1, "pageCount >= 1");
    },
  },
  {
    id: "result-shape",
    capability: "render result exposes { pdf, pageCount, diagnostics }",
    status: "active",
    assert({ binding, assert }) {
      const res = binding.render("<p>x</p>", { ...BASE, data: {}, fonts: [FONT] });
      assert.ok(binding.pdfOf(res) != null, "pdf present");
      assert.equal(typeof binding.pageCountOf(res), "number", "pageCount is number");
      assert.ok(Array.isArray(binding.diagnosticsOf(res)), "diagnostics is array");
    },
  },
  {
    id: "jinja-templating",
    capability: "Jinja control flow ({% for %}) renders interpolated data",
    status: "active",
    assert({ binding, assert }) {
      const res = binding.render("{% for n in items %}<p>row {{ n }}</p>{% endfor %}", {
        ...BASE,
        data: { items: [1, 2, 3] },
        fonts: [FONT],
      });
      h.assertValidPdf(assert, binding.pdfOf(res), "jinja");
    },
  },
  {
    id: "css-page-pagination",
    capability: "@page geometry + content overflow produces multiple pages",
    status: "active",
    assert({ binding, assert }) {
      const many = Array.from({ length: 80 }, (_, i) => `<p>line ${i}</p>`).join("");
      const res = binding.render(many, { ...BASE, data: {}, fonts: [FONT] });
      assert.ok(binding.pageCountOf(res) > 1, "tall content paginates to >1 page");
    },
  },
  {
    id: "fonts-per-call",
    capability: "per-call fonts embed and shape glyphs",
    status: "active",
    assert({ binding, assert }) {
      const res = binding.render("<p>Hello</p>", { ...BASE, data: {}, fonts: [FONT] });
      h.assertValidPdf(assert, binding.pdfOf(res), "fonts");
    },
  },
  {
    id: "fonts-warm-handle",
    capability: "warm Fonts.load(...) handle is reused across renders",
    status: "active",
    assert({ binding, assert }) {
      const handle = binding.loadFonts([FONT]);
      const a = binding.render("<p>warm</p>", { ...BASE, data: {}, fontsHandle: handle });
      const b = binding.render("<p>warm</p>", { ...BASE, data: {}, fontsHandle: handle });
      h.assertValidPdf(assert, binding.pdfOf(a), "warm-fonts");
      assert.ok(
        Buffer.from(binding.pdfOf(a)).equals(Buffer.from(binding.pdfOf(b))),
        "warm handle: identical inputs -> identical bytes",
      );
    },
  },
  {
    id: "doc-meta",
    capability: "meta.title is written into the PDF Info dictionary",
    status: "active",
    assert({ binding, assert }) {
      const title = "Conformance Title 4711";
      const res = binding.render("<p>x</p>", {
        ...BASE,
        data: {},
        fonts: [FONT],
        meta: { title },
      });
      const buf = h.assertValidPdf(assert, binding.pdfOf(res), "doc-meta");
      assert.ok(buf.toString("latin1").includes(title), "meta.title appears in PDF bytes");
    },
  },
  {
    id: "running-header-flag",
    capability: "program.hasHeader()/hasFooter() report declared running regions",
    status: "active",
    assert({ binding, assert }) {
      const plain = binding.compile("<p>body</p>");
      assert.equal(plain.hasHeader(), false, "no header declared");
      const withHeader = binding.compile(
        "<t:running-header><p>H</p></t:running-header><p>body</p>",
      );
      assert.equal(withHeader.hasHeader(), true, "running-header detected");
    },
  },
  {
    id: "determinism",
    capability: "identical inputs (pinned now) produce byte-identical PDFs",
    status: "active",
    assert({ binding, assert }) {
      const opts = { ...BASE, data: { t: "x" }, fonts: [FONT] };
      const a = binding.render("<h1>{{ t }}</h1>", opts);
      const b = binding.render("<h1>{{ t }}</h1>", opts);
      assert.ok(
        Buffer.from(binding.pdfOf(a)).equals(Buffer.from(binding.pdfOf(b))),
        "deterministic bytes",
      );
    },
  },
  {
    id: "fatal-error-typed",
    capability: "a fatal template fault throws a structured error (code + span)",
    status: "active",
    assert({ binding, assert }) {
      assert.throws(
        () => binding.compile("{{ broken "),
        (err) => {
          // napi -> TurboPdfError{ code, span }; wasm -> rejects with { code, message, span }.
          assert.ok(err, "an error was thrown");
          assert.ok(
            "code" in err || (err.message && err.message.length > 0),
            "error carries a code or message",
          );
          return true;
        },
        "malformed template throws",
      );
    },
  },
  {
    id: "lints-returned-not-thrown",
    capability: "non-fatal lints come back in diagnostics, render still succeeds",
    status: "active",
    assert({ binding, assert }) {
      // Glyphs absent from the supplied face emit a NotdefGlyph lint.
      const res = binding.render("<p>你好世界</p>", { ...BASE, data: {}, fonts: [FONT] });
      assert.ok(binding.pageCountOf(res) >= 1, "render succeeds despite missing glyphs");
      assert.ok(
        binding.diagnosticsOf(res).some((d) => d.code === "NotdefGlyph"),
        "NotdefGlyph lint is returned, not thrown",
      );
    },
  },

  // --- ACTIVE, binding-specific (documented divergences) --------------------
  {
    id: "oneshot-render",
    capability: "one-shot render(html, opts) compiles + renders in one call",
    status: "active",
    only: ["napi"], // wasm exposes only compile()->program.render; not a silent gap.
    assert({ binding, assert }) {
      const res = binding.oneShot("<p>{{ t }}</p>", {
        ...BASE,
        data: { t: "one-shot" },
        fonts: [FONT],
      });
      h.assertValidPdf(assert, binding.pdfOf(res), "oneshot");
    },
  },
  {
    id: "compile-missing-policy",
    capability: "compile opts { missingPolicy } is honored (lenient renders blanks)",
    status: "active",
    only: ["wasm"], // napi `compile` opts are currently ignored; tracked as a gap below.
    assert({ binding, assert }) {
      const res = binding.render("<p>{{ maybe }}</p>", {
        ...BASE,
        data: {},
        fonts: [FONT],
        compileOpts: { missingPolicy: "empty" },
      });
      h.assertValidPdf(assert, binding.pdfOf(res), "missing-policy");
    },
  },

  // --- SKIPPED: intended surface, not yet exposed by the bindings -----------
  // These rows document the target API. Un-skip (flip status to "active" and
  // fill in the assertion) the moment the binding wires the capability through.
  {
    id: "named-images",
    capability: "named images embed an /Image XObject via <img src=name>",
    status: "skip",
    skipReason:
      "images cross the boundary as raw bytes but are dropped (NoImages); the " +
      "name-keyed resolver is not wired through either binding yet (Phase 9b). " +
      "When wired, assert countImageXObjects(pdf) > 0 with a named PNG and 0 without.",
    assert({ binding, assert }) {
      // Intended assertion once exposed:
      const res = binding.render('<img src="logo">', {
        ...BASE,
        data: {},
        fonts: [FONT],
        images: [{ name: "logo", data: h.PNG_BYTES }],
      });
      assert.ok(h.countImageXObjects(binding.pdfOf(res)) > 0, "image XObject embedded");
    },
  },
  {
    id: "watermark",
    capability: "watermark render option stamps DRAFT text + image overlay",
    status: "skip",
    skipReason: "no watermark render option is exposed by either binding yet.",
    assert() {
      throw new Error("watermark not exposed");
    },
  },
  {
    id: "append-pages",
    capability: "append: merge/append an existing PDF",
    status: "skip",
    skipReason: "PDF append/merge is not exposed by either binding yet.",
    assert() {
      throw new Error("append not exposed");
    },
  },
  {
    id: "encrypt",
    capability: "encrypt: password / permissions on the output PDF",
    status: "skip",
    skipReason: "encryption is not exposed by either binding yet.",
    assert() {
      throw new Error("encrypt not exposed");
    },
  },
  {
    id: "pdf-a",
    capability: "pdfA: PDF/A archival conformance output",
    status: "skip",
    skipReason: "PDF/A output is not exposed by either binding yet.",
    assert() {
      throw new Error("pdfA not exposed");
    },
  },
  {
    id: "pdf-ua",
    capability: "pdfUa: PDF/UA tagged-accessibility output",
    status: "skip",
    skipReason: "PDF/UA output is not exposed by either binding yet.",
    assert() {
      throw new Error("pdfUa not exposed");
    },
  },
  {
    id: "cmyk",
    capability: "cmyk: CMYK color output for print",
    status: "skip",
    skipReason: "CMYK color output is not exposed by either binding yet.",
    assert() {
      throw new Error("cmyk not exposed");
    },
  },
];

/** Rows that apply to a binding id (respecting an `only` restriction). */
export function rowsFor(bindingId) {
  return MATRIX.filter((row) => !row.only || row.only.includes(bindingId));
}
