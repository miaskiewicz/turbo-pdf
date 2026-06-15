//! Font registry + fallback chain (§4.4). Selects a face for a `font-family`
//! list by weight/style closeness, and resolves per-character fallback so a
//! glyph missing from the primary face is found in a later one. A glyph absent
//! from every face yields `None`, which the caller turns into `.notdef` + a lint.
//!
//! ## Bundled fallbacks (`bundled-fonts`, default-on)
//!
//! When the `bundled-fonts` feature is on, the registry is born with a set of
//! embedded OFL faces (see [`super::bundled`]) so a document renders with zero
//! caller-supplied fonts. They are kept in a *separate* list consulted only after
//! every caller face, so a caller that registers its own faces always wins
//! (requirement: bundled faces are fallbacks, never overrides). The CSS generic
//! keywords (`sans-serif`/`serif`/`monospace`) are expanded to the bundled real
//! family names here, so `font-family: sans-serif` selects Inter then Roboto.
//!
//! When the feature is off, the bundled list is always empty and every code path
//! below behaves exactly as the no-bundled build.

use super::font::FontFace;

/// A set of font faces: caller-supplied faces plus, when `bundled-fonts` is on,
/// the embedded fallback faces.
#[derive(Debug, Clone, Default)]
pub struct FontRegistry {
    /// Caller-registered faces, in registration order. Always preferred.
    faces: Vec<FontFace>,
    /// Embedded fallback faces (empty unless `bundled-fonts` is on). Consulted
    /// only after `faces`, so they never override a caller's font.
    bundled: Vec<FontFace>,
}

fn family_matches(face: &FontFace, name: &str) -> bool {
    face.family().eq_ignore_ascii_case(name.trim())
}

fn score(face: &FontFace, weight: u16, italic: bool) -> u32 {
    let weight_diff = (i32::from(face.weight()) - i32::from(weight)).unsigned_abs();
    let style_penalty = if face.is_italic() == italic { 0 } else { 1000 };
    weight_diff + style_penalty
}

/// Expand a CSS family name to the concrete family names to try. A generic
/// keyword (`sans-serif`/`serif`/`monospace`) expands to the bundled primary +
/// secondary real family names; anything else is itself. With the feature off
/// the bundled table is empty, so a generic keyword expands to nothing extra and
/// behaviour matches the no-bundled build.
fn expand_family(name: &str) -> Vec<&str> {
    #[cfg(feature = "bundled-fonts")]
    {
        let key = name.trim();
        for (generic, reals) in super::bundled::GENERICS {
            if key.eq_ignore_ascii_case(generic) {
                return reals.to_vec();
            }
        }
    }
    vec![name]
}

impl FontRegistry {
    /// A registry with no caller faces. Carries the bundled fallback faces when
    /// the `bundled-fonts` feature is on, so it can render without any caller
    /// font; identical to [`FontRegistry::default`] otherwise.
    pub fn new() -> Self {
        Self {
            faces: Vec::new(),
            bundled: bundled_faces(),
        }
    }

    pub fn add(&mut self, face: FontFace) {
        self.faces.push(face);
    }

    /// True when the registry has no usable face at all (neither caller nor
    /// bundled). With `bundled-fonts` on this is only true if the bundled set
    /// failed to load, which never happens for the shipped assets.
    pub fn is_empty(&self) -> bool {
        self.faces.is_empty() && self.bundled.is_empty()
    }

    /// The number of caller-supplied faces. Bundled fallbacks are not counted,
    /// so a caller can tell whether *it* registered anything.
    pub fn len(&self) -> usize {
        self.faces.len()
    }

    /// All faces in lookup order: caller faces first, then bundled fallbacks.
    fn all(&self) -> impl Iterator<Item = &FontFace> {
        self.faces.iter().chain(self.bundled.iter())
    }

    fn best_in_family(&self, name: &str, weight: u16, italic: bool) -> Option<&FontFace> {
        self.all()
            .filter(|f| family_matches(f, name))
            .min_by_key(|f| score(f, weight, italic))
    }

    /// Select the best face for a family list + weight/style, falling back to the
    /// first available face (caller, else bundled) if no family matches.
    pub fn select(&self, families: &[&str], weight: u16, italic: bool) -> Option<&FontFace> {
        families
            .iter()
            .flat_map(|fam| expand_family(fam))
            .find_map(|fam| self.best_in_family(fam, weight, italic))
            .or_else(|| self.all().next())
    }

    fn glyph_in_family(
        &self,
        name: &str,
        weight: u16,
        italic: bool,
        ch: char,
    ) -> Option<&FontFace> {
        self.all()
            .filter(|f| family_matches(f, name) && f.has_glyph(ch))
            .min_by_key(|f| score(f, weight, italic))
    }

    /// Resolve the face that should render `ch`, walking the (expanded) family
    /// list then any available face. Returns `None` if no face covers it.
    pub fn resolve_glyph(
        &self,
        families: &[&str],
        weight: u16,
        italic: bool,
        ch: char,
    ) -> Option<&FontFace> {
        for fam in families.iter().flat_map(|fam| expand_family(fam)) {
            if let Some(face) = self.glyph_in_family(fam, weight, italic, ch) {
                return Some(face);
            }
        }
        self.all().find(|f| f.has_glyph(ch))
    }
}

/// The bundled fallback faces for a fresh registry: the embedded set when
/// `bundled-fonts` is on, empty otherwise.
#[cfg(feature = "bundled-fonts")]
fn bundled_faces() -> Vec<FontFace> {
    super::bundled::bundled_faces()
}

#[cfg(not(feature = "bundled-fonts"))]
fn bundled_faces() -> Vec<FontFace> {
    Vec::new()
}
