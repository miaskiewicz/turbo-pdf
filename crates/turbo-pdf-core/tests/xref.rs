//! Phase 15b `xref` feature tests (§3, AC-3.25). Only compiled with `--features
//! xref`. Drives `<t:anchor name>` + `<a href="#name">` through the whole
//! pipeline (compile → render → cascade → layout → paginate → emit) and asserts
//! the emitted PDF carries a named GoTo destination and a Link annotation
//! targeting it. When `qpdf` is on PATH the output is also structurally checked.

#![cfg(feature = "xref")]

mod common;

use std::process::Command;

use turbo_pdf_core::layout::fragment::Fragment;
use turbo_pdf_core::paginate::{paginate, Page};
use turbo_pdf_core::style::TokenSet;
use turbo_pdf_core::{
    build_cascade, compile, emit_pdf, layout, style_tree, CompileOptions, Diagnostics, EmitOptions,
    StyledNode,
};

/// Run a bare HTML template (no data) through to paginated pages.
fn pages(template: &str) -> Vec<Page> {
    let (program, _) = compile(template, &CompileOptions::default()).expect("compile");
    let (nodes, _) = program
        .render_nodes(&serde_json::json!({}), Some(0))
        .expect("render");
    let cascade = build_cascade("", "", TokenSet::default());
    let styled: Vec<StyledNode> = style_tree(&nodes, &cascade);
    let mut diags = Diagnostics::default();
    let galley = layout(&styled, 540.0, &common::registry(), &mut diags);
    paginate(&galley, &[], &mut diags).expect("paginate")
}

/// Locate `needle` in a byte slice.
fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack
        .windows(needle.len())
        .any(|window| window == needle)
}

/// Concatenate every `link_href` found across a page's fragments.
fn collect_link_hrefs(frag: &Fragment, out: &mut Vec<String>) {
    if let Some(href) = &frag.xref.link_href {
        out.push(href.clone());
    }
    for child in &frag.children {
        collect_link_hrefs(child, out);
    }
}

/// Concatenate every `anchor` name found across a page's fragments.
fn collect_anchors(frag: &Fragment, out: &mut Vec<String>) {
    if let Some(name) = &frag.xref.anchor {
        out.push(name.clone());
    }
    for child in &frag.children {
        collect_anchors(child, out);
    }
}

fn all_hrefs(pages: &[Page]) -> Vec<String> {
    let mut out = Vec::new();
    for page in pages {
        for frag in &page.body {
            collect_link_hrefs(frag, &mut out);
        }
    }
    out
}

fn all_anchors(pages: &[Page]) -> Vec<String> {
    let mut out = Vec::new();
    for page in pages {
        for frag in &page.body {
            collect_anchors(frag, &mut out);
        }
    }
    out
}

// -- fragment threading --------------------------------------------------------

#[test]
fn anchor_name_rides_the_fragment() {
    let pages = pages("<t:anchor name=\"ch2\"/><div>body</div>");
    assert_eq!(all_anchors(&pages), vec!["ch2".to_string()]);
}

#[test]
fn internal_link_href_rides_the_fragment() {
    let pages = pages("<div><a href=\"#ch2\">see chapter 2</a></div>");
    assert_eq!(all_hrefs(&pages), vec!["ch2".to_string()]);
}

#[test]
fn external_and_empty_links_are_not_internal() {
    // An absolute href and a bare `#` carry no internal-link payload.
    let pages = pages("<div><a href=\"https://x\">x</a><a href=\"#\">y</a></div>");
    assert!(all_hrefs(&pages).is_empty());
}

// -- emit ----------------------------------------------------------------------

fn anchor_and_link_pdf() -> Vec<u8> {
    let pages = pages("<t:anchor name=\"ch2\"/><div>intro</div><div><a href=\"#ch2\">go</a></div>");
    emit_pdf(&pages, &EmitOptions::default())
}

#[test]
fn emits_named_destination_and_goto_link() {
    let pdf = anchor_and_link_pdf();
    assert!(contains(&pdf, b"/Dests"), "catalog references a Dests dict");
    assert!(contains(&pdf, b"/ch2"), "destination is named ch2");
    assert!(
        contains(&pdf, b"/Subtype /Link"),
        "a Link annotation is written"
    );
    assert!(contains(&pdf, b"/S /GoTo"), "the link uses a GoTo action");
    assert!(
        contains(&pdf, b"/Annots"),
        "the page references its annotations"
    );
}

#[test]
fn no_xref_markup_writes_no_annotations() {
    let pdf = emit_pdf(&pages("<div>plain body</div>"), &EmitOptions::default());
    assert!(!contains(&pdf, b"/Dests"), "no dests without an anchor");
    assert!(
        !contains(&pdf, b"/Subtype /Link"),
        "no links without an <a>"
    );
    assert!(!contains(&pdf, b"/Annots"), "no Annots array");
}

#[test]
fn output_is_deterministic() {
    let a = anchor_and_link_pdf();
    let b = anchor_and_link_pdf();
    assert_eq!(a, b, "identical inputs produce byte-identical xref output");
}

#[test]
fn qpdf_accepts_the_document() {
    if which_qpdf().is_none() {
        eprintln!("qpdf not on PATH; skipping structural check");
        return;
    }
    let pdf = anchor_and_link_pdf();
    let dir = std::env::temp_dir();
    let path = dir.join("turbo_pdf_xref_check.pdf");
    std::fs::write(&path, &pdf).expect("write pdf");
    let out = Command::new("qpdf")
        .arg("--check")
        .arg(&path)
        .output()
        .expect("run qpdf");
    assert!(
        out.status.success(),
        "qpdf --check failed:\n{}\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );
}

fn which_qpdf() -> Option<()> {
    Command::new("qpdf")
        .arg("--version")
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|_| ())
}
