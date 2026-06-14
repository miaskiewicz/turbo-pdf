//! The layout engine (§5, Stage 3): turns the styled tree into a galley — a
//! single continuous coordinate space of positioned [`Fragment`]s that the
//! fragmenter (Stage 4) paginates. Block and table flow and inline/text layout
//! are owned here; flex delegates to `taffy` (§5.3 decision).

pub mod block;
pub mod boxgen;
pub mod flex;
pub mod fragment;
pub mod inline;
pub mod table;
pub mod value;

use crate::error::Diagnostics;
use crate::style::StyledNode;
use crate::text::FontRegistry;

pub use fragment::Fragment;

/// Lay out a styled tree into the galley (Stage 3): box generation followed by
/// block/inline/flex/table layout into one continuous top-down fragment tree of
/// content width `cb_width` px. Missing-glyph and similar issues are collected
/// into `diags`.
pub fn layout(
    styled: &[StyledNode],
    cb_width: f32,
    fonts: &FontRegistry,
    diags: &mut Diagnostics,
) -> Fragment {
    let tree = boxgen::build_box_tree(styled);
    block::layout_tree(&tree, cb_width, fonts, diags)
}
