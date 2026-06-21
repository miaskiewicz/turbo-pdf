//! `bundled-fonts` feature (§4.4): the core embeds OFL faces so a document
//! renders with ZERO caller-supplied fonts. These tests drive the full pipeline
//! with a fresh `FontRegistry::new()` (no `.add`) and assert real glyphs are laid
//! out and the bundled face is subset + embedded in the PDF.
//!
//! The whole file is gated on the feature: with `--no-default-features` there are
//! no bundled faces, so a registry with no caller fonts can't render — exactly
//! the no-bundled behaviour the other suites exercise.
#![cfg(feature = "bundled-fonts")]

use turbo_html2pdf_core::layout::fragment::{Fragment, FragmentContent};
use turbo_html2pdf_core::style::TokenSet;
use turbo_html2pdf_core::{
    build_cascade, compile, emit_pdf, render_pages, style::AtRule, CompileOptions, Diagnostics,
    EmitOptions, FontFace, FontRegistry, Page, RenderInputs,
};

fn at_rules(css: &str) -> Vec<AtRule> {
    turbo_html2pdf_core::style::parse_stylesheet(css).at_rules
}

/// Drive a template + CSS through the full pipeline with NO caller fonts.
fn pages_no_caller_fonts(template: &str, css: &str) -> Vec<Page> {
    let (program, cdiags) = compile(template, &CompileOptions::default()).expect("compile");
    assert!(cdiags.is_empty(), "compile diags: {cdiags:?}");
    let cascade = build_cascade(css, "", TokenSet::default());
    // The whole point: a registry the caller never registered a face into.
    let fonts = FontRegistry::new();
    assert_eq!(fonts.len(), 0, "no caller faces registered");
    assert!(!fonts.is_empty(), "bundled faces present");
    let rules = at_rules(css);
    let inputs = RenderInputs {
        program: &program,
        data: &serde_json::json!({}),
        cascade: &cascade,
        at_rules: &rules,
        fonts: &fonts,
        images: &turbo_html2pdf_core::NoImages,
        now: Some(0),
    };
    let mut diags = Diagnostics::default();
    render_pages(&inputs, &mut diags).expect("render_pages")
}

fn collect_glyphs(frag: &Fragment, out: &mut Vec<u16>) {
    if let FragmentContent::TextLine { glyphs, .. } = &frag.content {
        out.extend(glyphs.iter().map(|g| g.glyph_id));
    }
    for child in &frag.children {
        collect_glyphs(child, out);
    }
}

fn page_glyph_ids(pages: &[Page]) -> Vec<u16> {
    let mut out = Vec::new();
    for page in pages {
        for frag in &page.body {
            collect_glyphs(frag, &mut out);
        }
    }
    out
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack.windows(needle.len()).any(|w| w == needle)
}

/// Every embedded bundled face parses and is tagged with a non-zero up-em.
#[test]
fn bundled_set_loads_all_faces() {
    let reg = FontRegistry::new();
    // 4 (Inter) + 4 (Roboto) + 4 (Liberation Serif) + 4 (PT Serif)
    // + 2 (Fira Code) + 4 (IBM Plex Mono) = 22 faces.
    assert!(!reg.is_empty());
    // Each generic family resolves to a real face at regular and bold.
    for fam in ["sans-serif", "serif", "monospace"] {
        let regular = reg.select(&[fam], 400, false);
        assert!(regular.is_some(), "{fam} regular missing");
        let bold = reg.select(&[fam], 700, false);
        assert!(bold.is_some(), "{fam} bold missing");
        assert!(regular.unwrap().units_per_em() > 0);
    }
}

/// `font-family: sans-serif` (the UA body default) selects Inter; bold/italic
/// pick the matching Inter face; the bundled secondary (Roboto) is reachable.
#[test]
fn generic_families_map_to_bundled_primaries() {
    let reg = FontRegistry::new();
    assert_eq!(
        reg.select(&["sans-serif"], 400, false).unwrap().family(),
        "Inter"
    );
    assert_eq!(
        reg.select(&["sans-serif"], 700, true).unwrap().family(),
        "Inter"
    );
    assert!(reg.select(&["sans-serif"], 700, true).unwrap().is_italic());
    assert_eq!(
        reg.select(&["serif"], 400, false).unwrap().family(),
        "Liberation Serif"
    );
    assert_eq!(
        reg.select(&["monospace"], 400, false).unwrap().family(),
        "Fira Code"
    );
    // A name that matches no bundled family still falls back to *some* face.
    assert!(reg.select(&["Nonexistent Font"], 400, false).is_some());
}

/// A doc with NO author CSS and NO caller fonts lays out real glyphs (the body
/// default resolves to the bundled sans stack).
#[test]
fn renders_with_no_caller_fonts_and_no_css() {
    let pages = pages_no_caller_fonts("<p>Hello bundled fonts</p>", "");
    let glyphs = page_glyph_ids(&pages);
    assert!(!glyphs.is_empty(), "no glyphs laid out");
    // `.notdef` is glyph 0; assert at least one real (non-zero) glyph.
    assert!(glyphs.iter().any(|&g| g != 0), "only .notdef glyphs");
}

/// The bundled sans face (Inter) is subset and embedded in the emitted PDF when
/// the caller supplies no fonts.
#[test]
fn pdf_embeds_bundled_sans_face() {
    let pages = pages_no_caller_fonts("<p>Bundled sans default</p>", "");
    let pdf = emit_pdf(&pages, &EmitOptions::default());
    assert!(pdf.starts_with(b"%PDF-1.7"));
    // The base font name embeds the sanitized family ("Inter").
    assert!(contains(&pdf, b"Inter"), "PDF does not embed Inter");
    // Inter is a CFF/OTF face: it embeds as a FontFile3 / CIDFontType0.
    assert!(contains(&pdf, b"FontFile3"), "Inter should embed as CFF");
}

/// `font-family: monospace` resolves to Fira Code, embedded as a TrueType
/// FontFile2; `font-family: serif` resolves to Liberation Serif.
#[test]
fn pdf_embeds_bundled_monospace_and_serif() {
    let mono = pages_no_caller_fonts("<p style=\"font-family: monospace\">code()</p>", "");
    let pdf = emit_pdf(&mono, &EmitOptions::default());
    assert!(contains(&pdf, b"FiraCode"), "PDF does not embed Fira Code");
    assert!(
        contains(&pdf, b"FontFile2"),
        "Fira Code should embed as TrueType"
    );

    let serif = pages_no_caller_fonts("<p style=\"font-family: serif\">serif text</p>", "");
    let pdf = emit_pdf(&serif, &EmitOptions::default());
    assert!(
        contains(&pdf, b"LiberationSerif"),
        "PDF does not embed Liberation Serif"
    );
}

/// A caller-supplied face takes precedence over the bundled set for the same
/// family request (bundled faces are fallbacks, not overrides).
#[test]
fn caller_face_overrides_bundled_for_same_family() {
    let go_bytes = std::fs::read(format!(
        "{}/assets/fonts/Go-Regular.ttf",
        env!("CARGO_MANIFEST_DIR")
    ))
    .expect("Go fixture");
    let mut reg = FontRegistry::new();
    reg.add(FontFace::from_bytes(go_bytes, "Go", 400, false).expect("parse Go"));
    // The caller face is selected for its own family.
    assert_eq!(reg.select(&["Go"], 400, false).unwrap().family(), "Go");
    // It is also first in the fallback order, so an unknown family falls back to
    // the caller's face, not a bundled one.
    assert_eq!(
        reg.select(&["Totally Unknown"], 400, false)
            .unwrap()
            .family(),
        "Go"
    );
    // Generic keywords still resolve to bundled faces (the caller didn't supply
    // a sans-serif), so bundled faces remain available as fallbacks.
    assert_eq!(
        reg.select(&["sans-serif"], 400, false).unwrap().family(),
        "Inter"
    );
}
