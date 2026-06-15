// BINDING CONFORMANCE SUITE.
//
// Proves the SHIPPED turbo-html2pdf bindings (napi + wasm) actually expose the
// engine's capabilities — by loading each binding's real built entry point and
// running the shared capability MATRIX against it. A binding that silently drops
// a capability fails here, even when the core crate still has it.
//
// HOW TO RUN
//   1. Build napi:  cargo build -p turbo-pdf-napi --release
//                   node crates/turbo-pdf-napi/scripts/copy-addon.mjs
//   2. Build wasm:  wasm-pack build crates/turbo-pdf-wasm --target nodejs \
//                     --out-dir pkg-node
//   3. node --test conformance/conformance.test.mjs
//
// A binding that is not built is SKIPPED (with a clear "how to build" message),
// never silently passed — so a missing toolchain lane is visible but not a red
// herring. In CI both packages are built first, so every active row must pass.

import assert from "node:assert/strict";
import { describe, test } from "node:test";

import { loadBindings, qpdfAvailable } from "./harness.mjs";
import { MATRIX, rowsFor } from "./matrix.mjs";

const bindings = loadBindings();

describe("turbo-html2pdf binding conformance", () => {
  test("at least one binding is built (otherwise the gate is meaningless)", () => {
    const built = bindings.filter((b) => b.binding).map((b) => b.id);
    // Local dev may have only one binding built; CI builds both. We only fail
    // if NOTHING is built, which would make a green run a false negative.
    assert.ok(
      built.length > 0,
      "no binding package is built — see HOW TO RUN at the top of this file.\n" +
        bindings.map((b) => `  - ${b.id}: ${b.reason}`).join("\n"),
    );
  });

  test("the matrix has no duplicate capability ids", () => {
    const ids = MATRIX.map((r) => r.id);
    assert.equal(new Set(ids).size, ids.length, "capability ids are unique");
  });

  for (const { id, binding, reason } of bindings) {
    describe(`binding: ${id}`, () => {
      const rows = rowsFor(id);
      for (const row of rows) {
        const title = `${row.id} — ${row.capability}`;

        if (row.status === "skip") {
          // Documented-but-not-yet-exposed capability: recorded as a skip so the
          // intended surface is visible in the report and trivially un-skippable.
          test.skip(`${title}  [not yet exposed: ${row.skipReason}]`, () => {});
          continue;
        }

        // Active row: skip (not fail) when THIS binding is not built, so an
        // un-built lane is visible without red-barring an unrelated capability.
        test(title, { skip: binding ? false : reason }, () => {
          row.assert({ binding, assert });
        });
      }
    });
  }
});

test("conformance environment", () => {
  // Informational: surfaces whether qpdf deep-checks ran, for the CI log.
  const note = qpdfAvailable()
    ? "qpdf present — outputs are additionally `qpdf --check`ed"
    : "qpdf absent — outputs are checked structurally (%PDF-, %%EOF, object regex)";
  assert.ok(note.length > 0);
});
