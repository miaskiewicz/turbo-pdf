//! Phase 2 spec harness: CSS positioning (`position: relative/absolute/fixed`)
//! and out-of-flow behavior, asserted on the laid-out `Fragment` galley.
//!
//! Each probe box carries a unique opaque `background-color`; helpers find its
//! `Box` fragment by colour and read its border-box top-left. Boxes are styled
//! **inline** (the layout engine applies `style=` attributes; `<head>` is dropped
//! by the HTML parse, so class rules in `<style>` would need `<body>`-level
//! sheets — inline keeps the fixtures unambiguous).
//!
//! These encode the *expected* behavior and FAIL until Phase 2 (out-of-flow
//! layout in block.rs) is implemented. Do not weaken them to match current
//! (position-ignoring) output.

use turbo_html2pdf_core::layout::fragment::{Fragment, FragmentContent};
use turbo_html2pdf_core::layout::value::Rgba;
use turbo_html2pdf_core::text::FontRegistry;
use turbo_html2pdf_core::{layout_html, Diagnostics};

const WIDTH: f32 = 1000.0;

fn galley(body_html: &str) -> Fragment {
    let html = format!("<html><body style=\"margin:0\">{body_html}</body></html>");
    let mut diags = Diagnostics::default();
    layout_html(&html, "", WIDTH, &FontRegistry::new(), &mut diags).expect("layout")
}

fn walk<'a>(f: &'a Fragment, out: &mut Vec<&'a Fragment>) {
    out.push(f);
    for c in &f.children {
        walk(c, out);
    }
}

fn all(g: &Fragment) -> Vec<&Fragment> {
    let mut v = Vec::new();
    walk(g, &mut v);
    v
}

fn hex(h: &str) -> Rgba {
    let n = u32::from_str_radix(h, 16).unwrap();
    Rgba {
        r: (n >> 16) as u8,
        g: (n >> 8) as u8,
        b: n as u8,
        a: 255,
    }
}

/// Border-box top-left of the box whose background is `color`.
fn box_xy(g: &Fragment, color: &str) -> (f32, f32) {
    let want = hex(color);
    let f = all(g)
        .into_iter()
        .find(|f| matches!(&f.content, FragmentContent::Box { background: Some(bg), .. } if *bg == want))
        .unwrap_or_else(|| panic!("no box with background #{color}"));
    (f.x, f.y)
}

fn approx(a: f32, b: f32, what: &str) {
    assert!((a - b).abs() < 1.0, "{what}: {a} != {b}");
}

// A probe box: fixed 40×40, positioned per `style`, distinct colour.
fn probe(color: &str, style: &str) -> String {
    format!("<div style=\"width:40px;height:40px;background-color:#{color};{style}\"></div>")
}

#[test]
fn absolute_lands_at_its_insets() {
    // An absolutely-positioned box sits at (left, top) of its containing block
    // (here the body/root at origin), regardless of its position in the source.
    let g = galley(&format!(
        "{}{}",
        probe("aaaaaa", ""), // a normal-flow box first (40px tall)
        probe("ff0000", "position:absolute;top:120px;left:80px"),
    ));
    approx(box_xy(&g, "ff0000").0, 80.0, "abs left");
    approx(box_xy(&g, "ff0000").1, 120.0, "abs top");
}

#[test]
fn absolute_is_removed_from_flow() {
    // The absolute box must NOT push the following in-flow sibling down: the
    // green box lands right after the first grey box (y = 40), as if the red
    // absolute box were not in flow.
    let g = galley(&format!(
        "{}{}{}",
        probe("aaaaaa", ""),                                   // y = 0..40
        probe("ff0000", "position:absolute;top:500px;left:0"), // out of flow
        probe("00aa00", ""),                                   // y should be 40, not 80
    ));
    approx(box_xy(&g, "00aa00").1, 40.0, "sibling after absolute");
}

#[test]
fn relative_shifts_box_but_keeps_its_space() {
    // A relatively-positioned box is offset by (left, top) from its normal-flow
    // position, but still occupies its original space — so the following sibling
    // is unaffected (as if the box were static).
    let g = galley(&format!(
        "{}{}{}",
        probe("aaaaaa", ""),                                     // y = 0..40
        probe("0000ff", "position:relative;top:15px;left:25px"), // normal y=40 -> shifted to (25,55)
        probe("00aa00", ""),                                     // y = 80 (space still reserved)
    ));
    approx(box_xy(&g, "0000ff").0, 25.0, "relative left shift");
    approx(box_xy(&g, "0000ff").1, 55.0, "relative top shift");
    approx(
        box_xy(&g, "00aa00").1,
        80.0,
        "sibling after relative keeps flow",
    );
}

#[test]
fn absolute_is_relative_to_positioned_ancestor() {
    // An absolute box is positioned against its nearest positioned ancestor's
    // padding box, not the page. Parent is `position:relative` offset 200px down
    // with 10px padding; child `absolute; top:0; left:0` lands at the parent's
    // content origin (x = 10, y = 200 + 10).
    let g = galley(&format!(
        "<div style=\"height:60px\"></div>\
         <div style=\"position:relative;padding:10px;background-color:#cccccc\">{}</div>",
        probe("ff0000", "position:absolute;top:0;left:0"),
    ));
    approx(box_xy(&g, "ff0000").0, 10.0, "abs left in relative parent");
    approx(box_xy(&g, "ff0000").1, 70.0, "abs top in relative parent");
}

#[test]
fn fixed_is_relative_to_the_page_origin() {
    // `position:fixed` is positioned against the initial containing block (the
    // page), even inside offset ancestors: top:0;left:0 -> (0, 0).
    let g = galley(&format!(
        "<div style=\"height:100px\"></div>\
         <div style=\"padding:20px\">{}</div>",
        probe("ff0000", "position:fixed;top:0;left:0"),
    ));
    approx(box_xy(&g, "ff0000").0, 0.0, "fixed left");
    approx(box_xy(&g, "ff0000").1, 0.0, "fixed top");
}
