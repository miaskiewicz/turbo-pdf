//! Append/merge external PDF pages (`append` feature).
//!
//! turbo's emitter ([`crate::emit_pdf`]) renders the document, but it is
//! write-only — it cannot read a foreign PDF. Some workflows need to glue a
//! certified official PDF that arrives as separate bytes from a provider (e.g. a
//! Mexican CFDI or Portuguese DSNE) onto the end of the rendered document. The
//! ecosystem answer is `pdf-lib`'s `copyPages`; [`append_pdfs`] does it natively.
//!
//! This is a self-contained post-emit step: it takes the bytes turbo already
//! produced plus one or more foreign PDF blobs and returns a single PDF whose
//! pages are all of the base, then all pages of each extra in order. It does
//! **not** touch the layout/emit pipeline — it runs on finished bytes.
//!
//! The merge uses [`lopdf`] (read + write) because `pdf-writer` cannot parse:
//! load each document, renumber the extras' objects so their ids never collide
//! with the base, flatten each page's inheritable attributes (so reparenting
//! into a fresh flat tree loses nothing), then rebuild a single `/Pages` tree and
//! `/Catalog`, bumping the document version to the max of the inputs. lopdf's
//! writer is deterministic and uses no clock/random, so the same inputs always
//! yield byte-identical output.

use lopdf::{Dictionary, Document, Object, ObjectId};
use thiserror::Error;

/// Why an [`append_pdfs`] call failed.
#[derive(Debug, Error)]
pub enum AppendError {
    /// A document (the base or one of the extras) could not be parsed as a PDF.
    #[error("malformed PDF input: {0}")]
    Malformed(#[from] lopdf::Error),
    /// The merged result would have no pages (every input was page-less).
    #[error("no pages to append: inputs contain no pages")]
    NoPages,
}

/// Keys a page may inherit from an ancestor `/Pages` node (PDF 1.7 §7.7.3.4).
/// They are flattened onto each page before reparenting so the new flat tree
/// carries everything the page relied on.
const INHERITABLE: [&[u8]; 4] = [b"Resources", b"MediaBox", b"CropBox", b"Rotate"];

/// Merge `extras`' pages after `base`'s pages into one PDF.
///
/// `base` is the bytes [`crate::emit_pdf`] produced; each entry in `extras` is a
/// foreign PDF blob. The result's pages are all of `base`, then all pages of each
/// extra in input order, with object numbers renumbered to avoid collisions and a
/// freshly rebuilt catalog/page tree. Output is deterministic.
///
/// Returns [`AppendError::Malformed`] if any input fails to parse and
/// [`AppendError::NoPages`] if the inputs together contain no pages.
pub fn append_pdfs(base: &[u8], extras: &[&[u8]]) -> Result<Vec<u8>, AppendError> {
    let mut doc = Document::load_mem(base)?;
    let mut version = doc.version.clone();
    let mut page_ids = ordered_page_ids(&doc);

    let mut next_id = doc.max_id;
    for extra in extras {
        next_id = absorb(&mut doc, extra, next_id, &mut version, &mut page_ids)?;
    }
    doc.max_id = next_id;
    doc.version = version;

    if page_ids.is_empty() {
        return Err(AppendError::NoPages);
    }
    flatten_inheritable(&mut doc, &page_ids);
    Ok(rebuild(doc, page_ids))
}

/// Load one extra, renumber its objects above `next_id` so they never collide,
/// fold its objects into `doc`, append its page ids, and raise `version` to its
/// own if newer. Returns the new high-water id for the next extra.
fn absorb(
    doc: &mut Document,
    extra: &[u8],
    next_id: u32,
    version: &mut String,
    page_ids: &mut Vec<ObjectId>,
) -> Result<u32, AppendError> {
    let mut other = Document::load_mem(extra)?;
    other.renumber_objects_with(next_id + 1);
    if other.version > *version {
        *version = other.version.clone();
    }
    page_ids.extend(ordered_page_ids(&other));
    let high = other.max_id;
    for (id, obj) in other.objects {
        doc.objects.insert(id, obj);
    }
    Ok(high)
}

/// The document's page object ids in page order.
fn ordered_page_ids(doc: &Document) -> Vec<ObjectId> {
    doc.get_pages().into_values().collect()
}

/// Copy each inheritable attribute (`/Resources`, `/MediaBox`, `/CropBox`,
/// `/Rotate`) an ancestor provided down onto every page that lacks its own, so a
/// flat page tree loses no inherited state.
fn flatten_inheritable(doc: &mut Document, page_ids: &[ObjectId]) {
    for id in page_ids {
        let inherited = resolve_inherited(doc, *id);
        apply_inherited(doc, *id, inherited);
    }
}

/// Walk a page's `/Parent` chain collecting any inheritable key the page itself
/// does not set; the nearest ancestor wins.
fn resolve_inherited(doc: &Document, page_id: ObjectId) -> Vec<(&'static [u8], Object)> {
    let mut found: Vec<(&'static [u8], Object)> = Vec::new();
    let mut cursor = doc.get_dictionary(page_id).ok();
    let mut guard = 0usize;
    while let Some(dict) = cursor {
        collect_missing(dict, &mut found);
        guard += 1;
        cursor = parent_dict(doc, dict, guard);
    }
    found
}

/// For each inheritable key present on `dict` but not yet captured, capture it.
fn collect_missing(dict: &Dictionary, found: &mut Vec<(&'static [u8], Object)>) {
    for key in INHERITABLE {
        let already = found.iter().any(|(k, _)| *k == key);
        if let (false, Ok(value)) = (already, dict.get(key)) {
            found.push((key, value.clone()));
        }
    }
}

/// The parent dictionary of `dict`, or `None` at the root or past the cycle guard.
fn parent_dict<'a>(doc: &'a Document, dict: &Dictionary, guard: usize) -> Option<&'a Dictionary> {
    if guard > 64 {
        return None;
    }
    let parent_id = dict.get(b"Parent").and_then(Object::as_reference).ok()?;
    doc.get_dictionary(parent_id).ok()
}

/// Write the resolved inheritable attributes onto a page dictionary (only the
/// ones it did not already carry, which `resolve_inherited` guarantees).
fn apply_inherited(doc: &mut Document, page_id: ObjectId, inherited: Vec<(&[u8], Object)>) {
    if let Ok(page) = doc.get_object_mut(page_id).and_then(Object::as_dict_mut) {
        for (key, value) in inherited {
            if !page.has(key) {
                page.set(key.to_vec(), value);
            }
        }
    }
}

/// Build the final document: a single `/Pages` node parenting every collected
/// page, a fresh `/Catalog` pointing at it, dead objects pruned. Consumes `doc`.
fn rebuild(mut doc: Document, page_ids: Vec<ObjectId>) -> Vec<u8> {
    let pages_id = doc.new_object_id();
    reparent(&mut doc, &page_ids, pages_id);
    doc.objects
        .insert(pages_id, Object::Dictionary(pages_node(&page_ids)));

    let catalog_id = doc.new_object_id();
    doc.objects
        .insert(catalog_id, Object::Dictionary(catalog_node(pages_id)));
    doc.trailer.set("Root", Object::Reference(catalog_id));

    doc.prune_objects();
    let mut out = Vec::new();
    doc.save_to(&mut out).expect("lopdf save to Vec");
    out
}

/// Point every page's `/Parent` at the new `/Pages` node.
fn reparent(doc: &mut Document, page_ids: &[ObjectId], pages_id: ObjectId) {
    for id in page_ids {
        if let Ok(page) = doc.get_object_mut(*id).and_then(Object::as_dict_mut) {
            page.set("Parent", Object::Reference(pages_id));
        }
    }
}

/// A `/Pages` dictionary listing the pages as `/Kids` with the right `/Count`.
fn pages_node(page_ids: &[ObjectId]) -> Dictionary {
    let kids: Vec<Object> = page_ids.iter().map(|id| Object::Reference(*id)).collect();
    let mut pages = Dictionary::new();
    pages.set("Type", "Pages");
    pages.set("Count", page_ids.len() as i64);
    pages.set("Kids", Object::Array(kids));
    pages
}

/// A `/Catalog` dictionary referencing the page tree root.
fn catalog_node(pages_id: ObjectId) -> Dictionary {
    let mut cat = Dictionary::new();
    cat.set("Type", "Catalog");
    cat.set("Pages", Object::Reference(pages_id));
    cat
}
