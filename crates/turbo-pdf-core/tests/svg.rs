//! Phase 15b `svg` feature tests (§7.4). Only compiled with `--features svg`.
//!
//! Covers the gated decode/probe/sniff layer (`image::{probe, decode}` routing an
//! `image/svg+xml` document through `svg::{is_svg, probe_svg, decode_svg}`), the
//! rasterizer's RGB + alpha-plane output, byte-determinism, malformed/degenerate
//! handling, and the full layout → emit path (rasterized SVG embeds as an image
//! XObject with an `SMask`, deterministic, `qpdf --check` clean when available).
//!
//! All SVG inputs are tiny committed byte strings (shapes only — no external
//! files, no fonts, so rendering is host-independent and deterministic).

#![cfg(feature = "svg")]

mod common;

use std::collections::HashMap;

use turbo_pdf_core::image::{decode, probe, ColorSpace, Payload};
use turbo_pdf_core::paginate::{paginate, Page};
use turbo_pdf_core::svg::{decode_svg, is_svg, probe_svg};
use turbo_pdf_core::{
    build_cascade, emit_pdf_with_images, layout_with_images, style_tree, Attr, Diagnostics,
    Element, EmitOptions, ImageCtx, ImageResolver, Node, Tag,
};
use turbo_pdf_core::{style::TokenSet, StyledNode};

// --------------------------------------------------------------------------
// committed SVG inputs (shapes only — deterministic without fonts)
// --------------------------------------------------------------------------

/// A 4x4 SVG: a fully-opaque red square covering a half-opaque blue background,
/// so the raster carries both RGB colour and a varying alpha plane (→ SMask).
const SVG_RED_4X4: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="4" height="4" viewBox="0 0 4 4">
  <rect x="0" y="0" width="4" height="4" fill="#0000ff" fill-opacity="0.5"/>
  <rect x="1" y="1" width="2" height="2" fill="#ff0000"/>
</svg>"##;

/// A minimal SVG behind a UTF-8 BOM, leading whitespace and an `<?xml ?>` prolog,
/// to exercise that branch of the sniffer (XML is case-sensitive, so the element
/// itself stays lowercase `svg`).
const SVG_WITH_PROLOG: &str = "\u{feff}<?xml version=\"1.0\"?>\n  <svg xmlns=\"http://www.w3.org/2000/svg\" width=\"2\" height=\"2\"><rect width=\"2\" height=\"2\" fill=\"green\"/></svg>";

fn svg_bytes(s: &str) -> Vec<u8> {
    s.as_bytes().to_vec()
}

/// A by-name resolver backed by a map (mirrors the raster image tests).
struct MapResolver {
    map: HashMap<String, Vec<u8>>,
}

impl MapResolver {
    fn new(pairs: Vec<(&str, Vec<u8>)>) -> MapResolver {
        MapResolver {
            map: pairs.into_iter().map(|(k, v)| (k.to_string(), v)).collect(),
        }
    }
}

impl ImageResolver for MapResolver {
    fn resolve(&self, name: &str) -> Option<&[u8]> {
        self.map.get(name).map(Vec::as_slice)
    }
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack.windows(needle.len()).any(|w| w == needle)
}

// --------------------------------------------------------------------------
// sniff
// --------------------------------------------------------------------------

#[test]
fn is_svg_recognizes_svg_and_rejects_raster() {
    assert!(is_svg(SVG_RED_4X4.as_bytes()), "plain <svg> recognized");
    assert!(
        is_svg(SVG_WITH_PROLOG.as_bytes()),
        "BOM + leading whitespace + <?xml?> prolog recognized"
    );
    // The tag sniff is case-insensitive (an uppercase root still sniffs as SVG).
    assert!(is_svg(b"<SVG xmlns=\"http://www.w3.org/2000/svg\"></SVG>"));
    assert!(!is_svg(b"not an svg"));
    assert!(!is_svg(&[]));
    // PNG magic must not be mistaken for SVG.
    assert!(!is_svg(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]));
    // An XML prolog with no <svg> element is not an SVG.
    assert!(!is_svg(b"<?xml version=\"1.0\"?><html></html>"));
}

// --------------------------------------------------------------------------
// probe / decode
// --------------------------------------------------------------------------

#[test]
fn probe_svg_reports_oversampled_size_and_alpha() {
    let p = probe_svg(SVG_RED_4X4.as_bytes()).expect("probe svg");
    // SVG_SCALE = 2 => 4px intrinsic rasterizes to 8px.
    assert_eq!((p.width, p.height), (8, 8));
    assert!(p.has_alpha, "rasterized svg always carries an alpha plane");
}

#[test]
fn probe_routes_svg_through_svg_path() {
    // The public `probe` entry point must detect SVG (gated) before raster sniff.
    let p = probe(SVG_RED_4X4.as_bytes()).expect("probe routes svg");
    assert_eq!((p.width, p.height, p.has_alpha), (8, 8, true));
}

#[test]
fn decode_svg_yields_rgb_samples_and_alpha_plane() {
    let img = decode_svg(SVG_RED_4X4.as_bytes()).expect("decode svg");
    assert_eq!((img.width, img.height), (8, 8));
    assert_eq!(img.color(), ColorSpace::Rgb);
    let alpha = img.alpha.expect("svg must yield an alpha plane");
    assert_eq!(alpha.len(), 8 * 8, "one alpha byte per pixel");
    // The inner red square is opaque; the surrounding blue is half-opaque; so the
    // plane has both fully-opaque and partial values (not a flat 255).
    assert!(alpha.contains(&255), "opaque pixels present");
    assert!(
        alpha.iter().any(|&a| a > 0 && a < 255),
        "partial-alpha pixels present"
    );
    match img.payload {
        Payload::Raw { samples, color } => {
            assert_eq!(color, ColorSpace::Rgb);
            assert_eq!(samples.len(), 8 * 8 * 3, "3 rgb samples per pixel");
            // Centre pixel (the opaque red square) is red.
            let centre = (4 * 8 + 4) * 3;
            assert_eq!(&samples[centre..centre + 3], &[255, 0, 0], "centre is red");
        }
        Payload::Jpeg { .. } => panic!("svg must rasterize to raw samples"),
    }
}

#[test]
fn decode_routes_svg_through_svg_path() {
    let img = decode(SVG_RED_4X4.as_bytes()).expect("decode routes svg");
    assert_eq!((img.width, img.height), (8, 8));
    assert!(img.alpha.is_some());
}

#[test]
fn decode_svg_is_byte_deterministic() {
    let a = decode_svg(SVG_RED_4X4.as_bytes()).expect("a");
    let b = decode_svg(SVG_RED_4X4.as_bytes()).expect("b");
    assert_eq!(a.width, b.width);
    assert_eq!(a.height, b.height);
    assert_eq!(a.alpha, b.alpha, "alpha plane must be identical run-to-run");
    let (Payload::Raw { samples: sa, .. }, Payload::Raw { samples: sb, .. }) =
        (&a.payload, &b.payload)
    else {
        panic!("raw payload expected");
    };
    assert_eq!(sa, sb, "rgb samples must be byte-identical run-to-run");
}

#[test]
fn malformed_svg_is_handled_without_panic() {
    // Looks like an SVG (sniff hits) but the body is not parseable XML.
    let broken = b"<svg xmlns=\"http://www.w3.org/2000/svg\"><rect";
    assert!(is_svg(broken), "sniff still matches the opening tag");
    assert!(
        decode_svg(broken).is_none(),
        "malformed svg must not decode"
    );
    assert!(probe_svg(broken).is_none(), "malformed svg must not probe");
    // And via the public routers.
    assert!(decode(broken).is_none());
    assert!(probe(broken).is_none());
}

#[test]
fn zero_sized_svg_decodes_to_nothing() {
    // A syntactically valid SVG whose canvas collapses to zero has no pixels to
    // rasterize: it must yield `None`, not an empty/garbage buffer.
    let zero = br#"<svg xmlns="http://www.w3.org/2000/svg" width="0" height="0"></svg>"#;
    assert!(decode_svg(zero).is_none(), "zero-size svg decodes to none");
    assert!(probe_svg(zero).is_none(), "zero-size svg probes to none");
}

#[test]
fn prolog_svg_decodes() {
    // The BOM/prolog/uppercase variant must parse and rasterize too.
    let img = decode(SVG_WITH_PROLOG.as_bytes()).expect("decode prolog svg");
    assert_eq!((img.width, img.height), (4, 4)); // 2px * SVG_SCALE
    assert!(img.alpha.is_some());
}

// --------------------------------------------------------------------------
// layout + emit pipeline
// --------------------------------------------------------------------------

fn img_node(src: &str) -> Vec<Node> {
    vec![Node::Element(Element {
        tag: Tag::Html("img".into()),
        attrs: vec![Attr {
            name: "src".into(),
            value: src.into(),
        }],
        children: Vec::new(),
    })]
}

fn styled_img(src: &str) -> Vec<StyledNode> {
    let cascade = build_cascade("", "", TokenSet::default());
    style_tree(&img_node(src), &cascade)
}

fn paginate_img(src: &str, resolver: &dyn ImageResolver) -> Vec<Page> {
    let styled = styled_img(src);
    let ctx = ImageCtx {
        resolver,
        body_height: Some(700.0),
    };
    let mut diags = Diagnostics::default();
    let galley = layout_with_images(&styled, 540.0, &common::registry(), &ctx, &mut diags);
    paginate(&galley, &[], &mut diags).expect("paginate")
}

#[test]
fn svg_emits_image_xobject_with_smask() {
    let resolver = MapResolver::new(vec![("logo.svg", svg_bytes(SVG_RED_4X4))]);
    let pages = paginate_img("logo.svg", &resolver);
    let pdf = emit_pdf_with_images(&pages, &EmitOptions::default(), &resolver);
    assert!(pdf.starts_with(b"%PDF-1.7"));
    assert!(contains(&pdf, b"/Subtype /Image"), "missing image XObject");
    assert!(contains(&pdf, b"/Im0"), "missing image resource name");
    assert!(contains(&pdf, b"/DeviceRGB"), "rasterized svg is rgb");
    assert!(
        contains(&pdf, b"/SMask"),
        "transparent svg must emit an SMask"
    );
    assert!(contains(&pdf, b"/DeviceGray"), "smask is a gray image");
    assert!(contains(&pdf, b"%%EOF"));
}

#[test]
fn svg_emit_is_deterministic() {
    let resolver = MapResolver::new(vec![("logo.svg", svg_bytes(SVG_RED_4X4))]);
    let pages = paginate_img("logo.svg", &resolver);
    let a = emit_pdf_with_images(&pages, &EmitOptions::default(), &resolver);
    let b = emit_pdf_with_images(&pages, &EmitOptions::default(), &resolver);
    assert_eq!(a, b, "svg embedding must be byte-deterministic");
}

#[test]
fn malformed_svg_emits_nothing() {
    let resolver = MapResolver::new(vec![("bad.svg", b"<svg><rect".to_vec())]);
    let pages = paginate_img("bad.svg", &resolver);
    let pdf = emit_pdf_with_images(&pages, &EmitOptions::default(), &resolver);
    assert!(pdf.starts_with(b"%PDF-1.7"));
    assert!(
        !contains(&pdf, b"/Subtype /Image"),
        "malformed svg embeds nothing"
    );
}

// --------------------------------------------------------------------------
// qpdf structural check (gated on availability)
// --------------------------------------------------------------------------

#[test]
fn qpdf_check_svg_pdf_when_available() {
    if !qpdf_available() {
        return;
    }
    let resolver = MapResolver::new(vec![("logo.svg", svg_bytes(SVG_RED_4X4))]);
    let pages = paginate_img("logo.svg", &resolver);
    let pdf = emit_pdf_with_images(&pages, &EmitOptions::default(), &resolver);
    assert_qpdf_clean("logo.svg", &pdf);
}

fn qpdf_available() -> bool {
    std::process::Command::new("which")
        .arg("qpdf")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn assert_qpdf_clean(name: &str, pdf: &[u8]) {
    let path = std::env::temp_dir().join(format!("turbo-pdf-svg-{name}.pdf"));
    std::fs::write(&path, pdf).expect("write temp pdf");
    let out = std::process::Command::new("qpdf")
        .arg("--check")
        .arg(&path)
        .output()
        .expect("run qpdf");
    assert!(
        out.status.success(),
        "{name}: qpdf --check failed: {}",
        String::from_utf8_lossy(&out.stdout)
    );
}
