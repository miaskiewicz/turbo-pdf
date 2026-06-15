//! Vector (SVG) image rasterization (Phase 15b, §7.4). Compiled only under the
//! `svg` feature. The single job here is: take `image/svg+xml` bytes, rasterize
//! them deterministically into a straight-alpha RGBA buffer, and hand back a
//! [`RasterImage`] that the *existing* Phase 9b image XObject pipeline embeds
//! (RGB body + alpha → `SMask`). No new XObject code, no I/O.
//!
//! **Determinism (§0.2).** `usvg::Options::default()` is used verbatim: the font
//! database is left **empty** — no system-font lookup, no filesystem probe, no
//! clock, no threads (`resvg`/`tiny-skia` rasterize single-threaded). Identical
//! SVG bytes therefore rasterize to byte-identical pixels on every host. Text in
//! the SVG needs no glyphs to stay deterministic: with an empty `fontdb` glyph
//! runs simply contribute nothing, the same way an unresolved `<img>` paints
//! nothing. Shapes, paths, gradients, clips and opacity all render fully.
//!
//! **Scale.** SVG is resolution-independent; we rasterize at [`SVG_SCALE`]× the
//! document's intrinsic CSS-pixel size so logos/icons stay crisp when the PDF is
//! zoomed, then the layout size caps in `imgsize` scale the result to the box.

use resvg::tiny_skia;
use resvg::usvg;

use crate::image::{ColorSpace, Intrinsic, Payload, RasterImage};

/// Oversampling factor: SVGs rasterize at this multiple of their intrinsic
/// CSS-pixel size so the embedded raster stays sharp under PDF zoom. A fixed
/// constant keeps output byte-deterministic.
const SVG_SCALE: f32 = 2.0;

/// Whether `bytes` look like an SVG document (XML with an `<svg` element).
///
/// SVG has no single magic-number signature, so this sniffs the leading,
/// possibly-BOM/whitespace-prefixed bytes for the XML/SVG markers the way
/// browsers do. Kept conservative: a false negative just means the bytes fall
/// through to the raster sniffer (and then paint nothing), never a panic.
pub fn is_svg(bytes: &[u8]) -> bool {
    // Skip a UTF-8 BOM and leading ASCII whitespace.
    let mut rest = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(bytes);
    while let [first, tail @ ..] = rest {
        if first.is_ascii_whitespace() {
            rest = tail;
        } else {
            break;
        }
    }
    // Scan a bounded prefix for the `<svg` tag or an XML declaration that an SVG
    // document opens with. Bounded so a huge non-SVG blob is rejected cheaply.
    let head = &rest[..rest.len().min(512)];
    contains_ci(head, b"<svg") || (head.starts_with(b"<?xml") && contains_ci(rest, b"<svg"))
}

/// Case-insensitive substring search over a byte slice (ASCII), used for the SVG
/// tag sniff where the element name may be `<svg` or `<SVG`.
fn contains_ci(haystack: &[u8], needle: &[u8]) -> bool {
    haystack
        .windows(needle.len())
        .any(|w| w.eq_ignore_ascii_case(needle))
}

/// Parse SVG bytes into a usvg tree with the deterministic default options
/// (empty font database: no system-font lookup, no I/O). `None` on malformed
/// input.
fn parse(bytes: &[u8]) -> Option<usvg::Tree> {
    // `Options::default()` ships an empty `fontdb`; we deliberately never call
    // `load_system_fonts`, so the parse depends only on the input bytes.
    usvg::Tree::from_data(bytes, &usvg::Options::default()).ok()
}

/// The intrinsic pixel size of an SVG, read by parsing its root `width`/`height`
/// (or `viewBox`). Reports `has_alpha = true`: a rasterized SVG always carries an
/// alpha plane (its background is transparent), so the emitter writes an `SMask`.
pub fn probe_svg(bytes: &[u8]) -> Option<Intrinsic> {
    let tree = parse(bytes)?;
    let size = tree.size();
    let (width, height) = raster_size(size.width(), size.height())?;
    Some(Intrinsic {
        width,
        height,
        has_alpha: true,
    })
}

/// The oversampled raster dimensions for an SVG of CSS-pixel size `(w, h)`, or
/// `None` if either axis is degenerate (non-finite, ≤ 0, or rounds to 0). Sized
/// at [`SVG_SCALE`]× and clamped so an absurd document cannot request a buffer
/// that overflows `u32` or exhausts memory.
fn raster_size(w: f32, h: f32) -> Option<(u32, u32)> {
    let width = scaled_axis(w)?;
    let height = scaled_axis(h)?;
    Some((width, height))
}

/// One axis of [`raster_size`]: a finite, positive CSS-pixel length scaled by
/// [`SVG_SCALE`], ceiled, and clamped to a sane maximum. `None` for a degenerate
/// length (non-finite, ≤ 0, or one that rounds to a zero-pixel axis).
fn scaled_axis(len: f32) -> Option<u32> {
    /// Hard cap on either raster axis (px); a normal logo/icon is far below this.
    const MAX_DIM: f32 = 8192.0;
    if !len.is_finite() || len <= 0.0 {
        return None;
    }
    let px = (len * SVG_SCALE).ceil().min(MAX_DIM) as u32;
    (px > 0).then_some(px)
}

/// Rasterize SVG bytes into an embeddable [`RasterImage`]: RGB samples plus a
/// straight-alpha plane (→ `SMask`). `None` if the bytes are not a parseable SVG
/// or its size is degenerate.
pub fn decode_svg(bytes: &[u8]) -> Option<RasterImage> {
    let tree = parse(bytes)?;
    let size = tree.size();
    let (width, height) = raster_size(size.width(), size.height())?;
    let mut pixmap = tiny_skia::Pixmap::new(width, height)?;
    // Scale the unit SVG up to the oversampled buffer; identity otherwise, so the
    // transform (and thus the output) depends only on `SVG_SCALE` and the input.
    let transform = tiny_skia::Transform::from_scale(SVG_SCALE, SVG_SCALE);
    resvg::render(&tree, transform, &mut pixmap.as_mut());
    Some(pixmap_to_image(width, height, &pixmap))
}

/// Split a rendered tiny-skia pixmap (premultiplied RGBA) into a straight-alpha
/// RGB payload plus an 8-bit alpha plane, matching what the PNG path produces for
/// an RGBA source so the XObject/SMask emitter handles both identically.
fn pixmap_to_image(width: u32, height: u32, pixmap: &tiny_skia::Pixmap) -> RasterImage {
    let pixel_count = (width as usize) * (height as usize);
    let mut samples = Vec::with_capacity(pixel_count * 3);
    let mut alpha = Vec::with_capacity(pixel_count);
    for px in pixmap.pixels() {
        // `demultiply` recovers straight (non-premultiplied) colour, which is what
        // a PDF `/SMask` expects alongside the separate alpha plane.
        let c = px.demultiply();
        samples.push(c.red());
        samples.push(c.green());
        samples.push(c.blue());
        alpha.push(c.alpha());
    }
    RasterImage {
        width,
        height,
        payload: Payload::Raw {
            samples,
            color: ColorSpace::Rgb,
        },
        alpha: Some(alpha),
    }
}
