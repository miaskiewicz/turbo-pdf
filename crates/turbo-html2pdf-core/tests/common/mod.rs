//! Shared helpers for the layout integration tests: font fixtures loaded from
//! the in-repo `assets/fonts/` (never a system path) and a `ComputedStyle`
//! builder. `dead_code` is allowed because each test binary uses a subset.
#![allow(dead_code)]

use turbo_html2pdf_core::text::FontRegistry;
use turbo_html2pdf_core::{ComputedStyle, FontFace};

pub fn font_bytes(name: &str) -> Vec<u8> {
    let path = format!("{}/assets/fonts/{name}", env!("CARGO_MANIFEST_DIR"));
    std::fs::read(&path).unwrap_or_else(|_| panic!("fixture {path}"))
}

pub fn evolventa() -> FontFace {
    FontFace::from_bytes(font_bytes("Evolventa-zLXL.ttf"), "Evolventa", 400, false).unwrap()
}

pub fn evolventa_bold() -> FontFace {
    FontFace::from_bytes(
        font_bytes("EvolventaBold-55Xv.ttf"),
        "Evolventa",
        700,
        false,
    )
    .unwrap()
}

pub fn go() -> FontFace {
    FontFace::from_bytes(font_bytes("Go-Regular.ttf"), "Go", 400, false).unwrap()
}

/// A registry with Evolventa (regular + bold) and Go registered.
pub fn registry() -> FontRegistry {
    let mut reg = FontRegistry::new();
    reg.add(evolventa());
    reg.add(evolventa_bold());
    reg.add(go());
    reg
}

/// Build a `ComputedStyle` from `(property, value)` pairs.
pub fn style(pairs: &[(&str, &str)]) -> ComputedStyle {
    ComputedStyle::from_pairs(pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())))
}
