//! Bundled open-licensed fonts (`bundled-fonts` feature, default-on, §4.4).
//!
//! The core embeds a small set of OFL-licensed faces so a document renders with
//! *zero* caller-supplied fonts: three CSS generic families, each a primary →
//! secondary fallback pair. The font programs are baked in with `include_bytes!`
//! (relative to this file), so nothing is read from disk or the system at render
//! time and output stays deterministic (AC-4.10).
//!
//! These faces are *fallbacks*: the [`FontRegistry`](super::FontRegistry) only
//! consults them after every caller-supplied face, so a caller that registers
//! its own "Inter"/"serif"/etc. always wins. The generic-keyword wiring lives in
//! the registry; this module only knows how to turn the embedded bytes into
//! tagged [`FontFace`]s.
//!
//! When the feature is off this module is not compiled and the registry has no
//! bundled faces, so the build is byte-for-byte the no-bundled build.

use super::font::FontFace;

/// The CSS generic families the bundled set covers. The registry maps a
/// `font-family: <generic>` to `[primary, secondary]` real family names so the
/// embedded faces resolve.
pub(super) const GENERICS: &[(&str, [&str; 2])] = &[
    ("sans-serif", ["Inter", "Roboto"]),
    ("serif", ["Liberation Serif", "PT Serif"]),
    ("monospace", ["Fira Code", "IBM Plex Mono"]),
];

/// One embedded face: its program bytes plus the tags the registry selects on.
struct Bundled {
    bytes: &'static [u8],
    family: &'static str,
    weight: u16,
    italic: bool,
}

macro_rules! face {
    ($family:literal, $weight:expr, $italic:expr, $path:literal) => {
        Bundled {
            bytes: include_bytes!(concat!("../../assets/fonts/", $path)),
            family: $family,
            weight: $weight,
            italic: $italic,
        }
    };
}

/// The full embedded face table: Regular/Bold/Italic/BoldItalic per family where
/// the family ships them (Fira Code, a programming monospace, has no italics, so
/// italic monospace falls through to IBM Plex Mono).
const FACES: &[Bundled] = &[
    // sans-serif primary: Inter (static OTF/CFF build).
    face!("Inter", 400, false, "inter/Inter-Regular.otf"),
    face!("Inter", 700, false, "inter/Inter-Bold.otf"),
    face!("Inter", 400, true, "inter/Inter-Italic.otf"),
    face!("Inter", 700, true, "inter/Inter-BoldItalic.otf"),
    // sans-serif secondary: Roboto.
    face!("Roboto", 400, false, "roboto/Roboto-Regular.ttf"),
    face!("Roboto", 700, false, "roboto/Roboto-Bold.ttf"),
    face!("Roboto", 400, true, "roboto/Roboto-Italic.ttf"),
    face!("Roboto", 700, true, "roboto/Roboto-BoldItalic.ttf"),
    // serif primary: Liberation Serif.
    face!(
        "Liberation Serif",
        400,
        false,
        "liberation-serif/LiberationSerif-Regular.ttf"
    ),
    face!(
        "Liberation Serif",
        700,
        false,
        "liberation-serif/LiberationSerif-Bold.ttf"
    ),
    face!(
        "Liberation Serif",
        400,
        true,
        "liberation-serif/LiberationSerif-Italic.ttf"
    ),
    face!(
        "Liberation Serif",
        700,
        true,
        "liberation-serif/LiberationSerif-BoldItalic.ttf"
    ),
    // serif secondary: PT Serif.
    face!("PT Serif", 400, false, "pt-serif/PTSerif-Regular.ttf"),
    face!("PT Serif", 700, false, "pt-serif/PTSerif-Bold.ttf"),
    face!("PT Serif", 400, true, "pt-serif/PTSerif-Italic.ttf"),
    face!("PT Serif", 700, true, "pt-serif/PTSerif-BoldItalic.ttf"),
    // monospace primary: Fira Code (no italic faces upstream).
    face!("Fira Code", 400, false, "fira-code/FiraCode-Regular.ttf"),
    face!("Fira Code", 700, false, "fira-code/FiraCode-Bold.ttf"),
    // monospace secondary: IBM Plex Mono.
    face!(
        "IBM Plex Mono",
        400,
        false,
        "ibm-plex-mono/IBMPlexMono-Regular.ttf"
    ),
    face!(
        "IBM Plex Mono",
        700,
        false,
        "ibm-plex-mono/IBMPlexMono-Bold.ttf"
    ),
    face!(
        "IBM Plex Mono",
        400,
        true,
        "ibm-plex-mono/IBMPlexMono-Italic.ttf"
    ),
    face!(
        "IBM Plex Mono",
        700,
        true,
        "ibm-plex-mono/IBMPlexMono-BoldItalic.ttf"
    ),
];

/// Parse every embedded face into a tagged [`FontFace`]. The bytes are validated
/// at build time only by `include_bytes!`, so a corrupt asset would surface here;
/// the bundled assets are real fonts, so every entry parses (asserted in tests).
pub(super) fn bundled_faces() -> Vec<FontFace> {
    FACES
        .iter()
        .filter_map(|b| FontFace::from_bytes(b.bytes.to_vec(), b.family, b.weight, b.italic))
        .collect()
}
