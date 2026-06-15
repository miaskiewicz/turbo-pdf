//! Hot-path instrumentation tests (Phase 13). Compiled only with the `perf`
//! feature; run with `cargo test -p turbo-pdf-core --features perf`. They assert
//! the `FontFace` caches actually collapse the hot paths (one parse per distinct
//! char, one shape per distinct run) rather than per occurrence.
#![cfg(feature = "perf")]

mod common;

use turbo_pdf_core::perf;

#[test]
fn a_face_parses_the_font_once() {
    perf::reset();
    let f = common::evolventa();
    // The font is parsed once at construction; lookups reuse the cached face.
    f.glyph_index('A');
    f.glyph_index('B');
    assert_eq!(perf::count("font.face.build"), 1);
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
fn report_hot_path_breakdown() {
    use turbo_pdf_core::style::TokenSet;
    use turbo_pdf_core::{build_cascade, compile, layout, style_tree, CompileOptions, Diagnostics};
    let tmpl = "<table>{% for r in rows %}<tr><td>{{ r.a }}</td><td>{{ r.b }}</td>\
                <td>{{ r.c }}</td></tr>{% endfor %}</table>";
    let rows: Vec<_> = (0..1000)
        .map(|i| serde_json::json!({"a": format!("Item {i}"), "b": i, "c": format!("{}.50", i)}))
        .collect();
    let data = serde_json::json!({ "rows": rows });
    let (program, _) = compile(tmpl, &CompileOptions::default()).unwrap();
    let (nodes, _) = program.render_nodes(&data, Some(0)).unwrap();
    let cascade = build_cascade("", "", TokenSet::default());
    let styled = style_tree(&nodes, &cascade);
    let reg = common::registry();
    perf::reset();
    let mut diags = Diagnostics::default();
    let _ = layout(&styled, 600.0, &reg, &mut diags);
    eprintln!("HOTPATH report-1k layout: {:?}", perf::snapshot());
    // Regression guard: shaping must stay deduped (far below the ~6k text cells).
    assert!(perf::count("font.shape") < 5000);
}

#[test]
fn invoice_stage_timing() {
    use std::time::Instant;
    use turbo_pdf_core::style::TokenSet;
    use turbo_pdf_core::{build_cascade, compile, layout, style_tree, CompileOptions, Diagnostics};
    let tmpl = "<style>.tot{font-weight:bold}</style><h1>Invoice {{ n }}</h1>\
        <table>{% for r in rows %}<tr><td>{{ r.d }}</td><td>{{ r.q }}</td>\
        <td>{{ r.p }}</td></tr>{% endfor %}</table><p class=tot>Total {{ t }}</p>";
    let rows: Vec<_> = (0..8)
        .map(|i| serde_json::json!({"d": format!("Service {i}"), "q": i, "p": format!("{i}.00")}))
        .collect();
    let data = serde_json::json!({ "n": 4012, "rows": rows, "t": "1234.00" });
    let time = |iters: u32, f: &mut dyn FnMut()| {
        let t = Instant::now();
        for _ in 0..iters {
            f();
        }
        t.elapsed().as_micros() as f64 / f64::from(iters)
    };
    let t_reg = time(200, &mut || {
        std::hint::black_box(common::registry());
    });
    let (program, _) = compile(tmpl, &CompileOptions::default()).unwrap();
    let t_render = time(2000, &mut || {
        std::hint::black_box(program.render_nodes(&data, Some(0)).unwrap());
    });
    let t_cascade = time(2000, &mut || {
        std::hint::black_box(build_cascade("", "", TokenSet::default()));
    });
    let (nodes, _) = program.render_nodes(&data, Some(0)).unwrap();
    let cascade = build_cascade("", "", TokenSet::default());
    let t_style = time(2000, &mut || {
        std::hint::black_box(style_tree(&nodes, &cascade));
    });
    let styled = style_tree(&nodes, &cascade);
    let reg = common::registry();
    let t_layout = time(2000, &mut || {
        let mut d = Diagnostics::default();
        std::hint::black_box(layout(&styled, 600.0, &reg, &mut d));
    });
    eprintln!(
        "STAGES(us): registry={t_reg:.1} render_nodes={t_render:.1} cascade={t_cascade:.1} style={t_style:.1} layout={t_layout:.1}"
    );
}

#[test]
fn snapshot_exposes_named_counters() {
    let f = common::evolventa();
    perf::reset();
    f.shape("x");
    let snap = perf::snapshot();
    assert_eq!(snap.get("font.shape").copied(), Some(1));
}
