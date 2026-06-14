//! Hot-path instrumentation tests (Phase 13). Compiled only with the `perf`
//! feature; run with `cargo test -p turbo-pdf-core --features perf`. They assert
//! the `FontFace` caches actually collapse the hot paths (one parse per distinct
//! char, one shape per distinct run) rather than per occurrence.
#![cfg(feature = "perf")]

mod common;

use turbo_pdf_core::perf;

#[test]
fn glyph_lookup_parses_once_per_distinct_char() {
    let f = common::evolventa();
    perf::reset();
    f.glyph_index('A');
    f.glyph_index('A'); // cached — no re-parse
    f.glyph_index('B');
    // Two distinct chars => two cmap parses, not three lookups' worth.
    assert_eq!(perf::count("font.ttf.parse"), 2);
}

#[test]
fn shaping_runs_once_per_distinct_run() {
    let f = common::evolventa();
    perf::reset();
    f.shape("Hello");
    f.measure("Hello", 16.0, 0.0); // re-uses the cached shaping
    f.shape("World");
    assert_eq!(perf::count("font.shape"), 2); // not 3
}

#[test]
fn snapshot_exposes_named_counters() {
    let f = common::evolventa();
    perf::reset();
    f.shape("x");
    let snap = perf::snapshot();
    assert_eq!(snap.get("font.shape").copied(), Some(1));
}
