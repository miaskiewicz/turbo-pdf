//! Multi-run inline layout (§5.2, AC-5.4). Generalizes `text::layout_text` (one
//! run) to a paragraph of styled runs: it itemizes each run into shaping segments
//! by per-glyph fallback face (`.notdef` + [`LintCode::NotdefGlyph`] when no face
//! covers a char), wraps words greedily across run boundaries, baseline-aligns
//! mixed font sizes, and applies `vertical-align`.
//!
//! Deferred (per §2): justified spacing (`Align::Justify` lays out left), Knuth-
//! Liang hyphenation, bidi/RTL, and intra-word break opportunities beyond
//! whitespace (CJK). Words break at whitespace only in v1.

use crate::error::{Diagnostics, LintCode, Span};
use crate::text::{Align, FontFace, FontRegistry};

use super::fragment::{NodeId, PositionedGlyph};
use super::value::{Rgba, VAlign};

/// One styled run of text entering inline layout. `face` is the run's primary
/// face (already selected); `families`/`weight`/`italic` drive per-glyph fallback.
#[derive(Debug, Clone)]
pub struct InlineRun {
    pub node_id: NodeId,
    pub text: String,
    pub face: FontFace,
    pub families: Vec<String>,
    pub weight: u16,
    pub italic: bool,
    pub font_size: f32,
    pub line_height: Option<f32>,
    pub letter_spacing: f32,
    pub color: Rgba,
    pub valign: VAlign,
}

/// A contiguous run of glyphs sharing a face/size/color, positioned relative to
/// the line's top-left.
#[derive(Debug, Clone)]
pub struct GlyphRun {
    pub node_id: NodeId,
    pub glyphs: Vec<PositionedGlyph>,
    pub face: FontFace,
    pub font_size: f32,
    pub color: Rgba,
}

/// One laid-out line: its glyph runs, used width, top offset, and box height.
#[derive(Debug, Clone)]
pub struct InlineLine {
    pub runs: Vec<GlyphRun>,
    pub width: f32,
    pub top: f32,
    pub height: f32,
}

/// The result of laying out a paragraph of runs into a column of `max_width`.
#[derive(Debug, Clone)]
pub struct ParagraphLayout {
    pub lines: Vec<InlineLine>,
    pub width: f32,
    pub height: f32,
}

// --------------------------------------------------------------------------
// itemization: chars -> per-glyph fallback faces
// --------------------------------------------------------------------------

struct CharInfo {
    ch: char,
    run: usize,
    face: FontFace,
}

fn resolve_face(ch: char, run: &InlineRun, reg: &FontRegistry) -> (FontFace, bool) {
    if run.face.has_glyph(ch) {
        return (run.face.clone(), false);
    }
    let families: Vec<&str> = run.families.iter().map(String::as_str).collect();
    match reg.resolve_glyph(&families, run.weight, run.italic, ch) {
        Some(face) => (face.clone(), false),
        None => (run.face.clone(), true),
    }
}

fn flatten_chars(runs: &[InlineRun], reg: &FontRegistry, diags: &mut Diagnostics) -> Vec<CharInfo> {
    let mut out = Vec::new();
    for (i, run) in runs.iter().enumerate() {
        let mut missing = false;
        for ch in run.text.chars() {
            let (face, notdef) = resolve_face(ch, run, reg);
            missing |= notdef;
            out.push(CharInfo { ch, run: i, face });
        }
        if missing {
            diags.push(
                LintCode::NotdefGlyph,
                "glyph missing from all fonts",
                Span::default(),
            );
        }
    }
    out
}

// --------------------------------------------------------------------------
// words & segments
// --------------------------------------------------------------------------

struct Seg {
    face: FontFace,
    text: String,
    font_size: f32,
    color: Rgba,
    valign: VAlign,
    letter_spacing: f32,
    line_height: Option<f32>,
    node_id: NodeId,
    width: f32,
}

struct Word {
    segs: Vec<Seg>,
    width: f32,
    space_after: f32,
}

fn seg_key(c: &CharInfo) -> (usize, String, u16, bool) {
    (
        c.run,
        c.face.family().to_string(),
        c.face.weight(),
        c.face.is_italic(),
    )
}

fn make_seg(chars: &[CharInfo], runs: &[InlineRun]) -> Seg {
    let text: String = chars.iter().map(|c| c.ch).collect();
    let run = &runs[chars[0].run];
    let face = chars[0].face.clone();
    let width = face.measure(&text, run.font_size, run.letter_spacing);
    Seg {
        face,
        text,
        font_size: run.font_size,
        color: run.color,
        valign: run.valign,
        letter_spacing: run.letter_spacing,
        line_height: run.line_height,
        node_id: run.node_id,
        width,
    }
}

fn make_word(chars: &[CharInfo], runs: &[InlineRun], space_after: f32) -> Word {
    let mut segs = Vec::new();
    let mut idx = 0;
    while idx < chars.len() {
        let start = idx;
        let key = seg_key(&chars[idx]);
        while idx < chars.len() && seg_key(&chars[idx]) == key {
            idx += 1;
        }
        segs.push(make_seg(&chars[start..idx], runs));
    }
    let width = segs.iter().map(|s| s.width).sum();
    Word {
        segs,
        width,
        space_after,
    }
}

fn space_width(c: &CharInfo, runs: &[InlineRun]) -> f32 {
    let run = &runs[c.run];
    run.face.measure(" ", run.font_size, run.letter_spacing)
}

fn build_words(chars: &[CharInfo], runs: &[InlineRun]) -> Vec<Word> {
    let mut words = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        if chars[i].ch.is_whitespace() {
            i += 1;
            continue;
        }
        let start = i;
        while i < chars.len() && !chars[i].ch.is_whitespace() {
            i += 1;
        }
        let space_after = chars.get(i).map_or(0.0, |c| space_width(c, runs));
        words.push(make_word(&chars[start..i], runs, space_after));
    }
    words
}

// --------------------------------------------------------------------------
// line breaking
// --------------------------------------------------------------------------

fn wrap_words(words: Vec<Word>, max_width: f32) -> Vec<Vec<Word>> {
    let mut lines = Vec::new();
    let mut cur: Vec<Word> = Vec::new();
    let mut x = 0.0;
    for word in words {
        if !cur.is_empty() && x + word.width > max_width {
            lines.push(std::mem::take(&mut cur));
            x = 0.0;
        }
        x += word.width + word.space_after;
        cur.push(word);
    }
    if !cur.is_empty() {
        lines.push(cur);
    }
    lines
}

// --------------------------------------------------------------------------
// line placement & metrics
// --------------------------------------------------------------------------

fn valign_shift(v: VAlign, size: f32) -> f32 {
    match v {
        VAlign::Super => size * 0.33,
        VAlign::Sub => -size * 0.2,
        _ => 0.0,
    }
}

fn align_offset(align: Align, width: f32, max_width: f32) -> f32 {
    match align {
        Align::Left | Align::Justify => 0.0,
        Align::Right => (max_width - width).max(0.0),
        Align::Center => ((max_width - width) / 2.0).max(0.0),
    }
}

struct Metrics {
    ascent: f32,
    descent: f32,
    height: f32,
}

fn fold_seg_metrics(seg: &Seg, m: &mut Metrics) {
    let shift = valign_shift(seg.valign, seg.font_size);
    m.ascent = m.ascent.max(seg.face.ascent_px(seg.font_size) + shift);
    m.descent = m.descent.max(seg.face.descent_px(seg.font_size) - shift);
    let lh = seg
        .line_height
        .unwrap_or_else(|| seg.face.line_height_px(seg.font_size));
    m.height = m.height.max(lh);
}

fn line_metrics(words: &[Word]) -> Metrics {
    let mut m = Metrics {
        ascent: 0.0,
        descent: 0.0,
        height: 0.0,
    };
    for word in words {
        for seg in &word.segs {
            fold_seg_metrics(seg, &mut m);
        }
    }
    m.height = m.height.max(m.ascent + m.descent);
    m
}

fn line_used_width(words: &[Word]) -> f32 {
    let mut w = 0.0;
    for (i, word) in words.iter().enumerate() {
        w += word.width;
        if i + 1 < words.len() {
            w += word.space_after;
        }
    }
    w
}

fn shape_seg(seg: &Seg, pen_x: f32, baseline: f32) -> GlyphRun {
    let scale = seg.face.scale(seg.font_size);
    let seg_baseline = baseline - valign_shift(seg.valign, seg.font_size);
    let mut glyphs = Vec::new();
    let mut x = pen_x;
    for g in seg.face.shape(&seg.text) {
        glyphs.push(PositionedGlyph {
            glyph_id: g.glyph_id,
            x: x + scale * g.x_offset as f32,
            y: seg_baseline - scale * g.y_offset as f32,
        });
        x += scale * g.x_advance as f32 + seg.letter_spacing;
    }
    GlyphRun {
        node_id: seg.node_id,
        glyphs,
        face: seg.face.clone(),
        font_size: seg.font_size,
        color: seg.color,
    }
}

fn same_face(a: &FontFace, b: &FontFace) -> bool {
    a.family() == b.family() && a.weight() == b.weight() && a.is_italic() == b.is_italic()
}

fn can_merge(a: &GlyphRun, b: &GlyphRun) -> bool {
    a.node_id == b.node_id
        && a.font_size == b.font_size
        && a.color == b.color
        && same_face(&a.face, &b.face)
}

fn merge_runs(runs: Vec<GlyphRun>) -> Vec<GlyphRun> {
    let mut out: Vec<GlyphRun> = Vec::new();
    for run in runs {
        match out.last_mut() {
            Some(last) if can_merge(last, &run) => last.glyphs.extend(run.glyphs),
            _ => out.push(run),
        }
    }
    out
}

fn place_line(words: Vec<Word>, max_width: f32, align: Align) -> InlineLine {
    let m = line_metrics(&words);
    let baseline = m.ascent + (m.height - m.ascent - m.descent) / 2.0;
    let width = line_used_width(&words);
    let mut pen = align_offset(align, width, max_width);
    let mut runs = Vec::new();
    for word in &words {
        for seg in &word.segs {
            runs.push(shape_seg(seg, pen, baseline));
            pen += seg.width;
        }
        pen += word.space_after;
    }
    InlineLine {
        runs: merge_runs(runs),
        width,
        top: 0.0,
        height: m.height,
    }
}

fn finalize(mut lines: Vec<InlineLine>) -> ParagraphLayout {
    let mut y = 0.0;
    let mut width = 0.0_f32;
    for line in &mut lines {
        line.top = y;
        y += line.height;
        width = width.max(line.width);
    }
    ParagraphLayout {
        lines,
        width,
        height: y,
    }
}

/// Lay out a paragraph of styled runs into lines fitting `max_width` px.
pub fn layout_paragraph(
    runs: &[InlineRun],
    reg: &FontRegistry,
    max_width: f32,
    align: Align,
    diags: &mut Diagnostics,
) -> ParagraphLayout {
    let chars = flatten_chars(runs, reg, diags);
    let words = build_words(&chars, runs);
    let lines = wrap_words(words, max_width);
    let placed = lines
        .into_iter()
        .map(|w| place_line(w, max_width, align))
        .collect();
    finalize(placed)
}
