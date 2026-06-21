//! `xref`-feature box-generation helpers (AC-3.25): deriving named destinations
//! from `<t:anchor>` directives and internal link targets from `<a href="#…">`.
//! Only compiled under the `xref` feature, so this lives in its own module (kept
//! out of the default coverage surface, like the other gated-only modules).

use crate::node::{Attr, TKind, Tag};
use crate::style::StyledElement;

/// The destination name of a `<t:anchor name="x">`, or `None`.
pub(super) fn anchor_name(kind: TKind, el: &StyledElement) -> Option<String> {
    if !matches!(kind, TKind::Anchor) {
        return None;
    }
    attr_value(&el.attrs, "name")
        .filter(|n| !n.is_empty())
        .map(str::to_string)
}

/// The bare `#name` destination of an `<a href="#name">`, or `None` for any other
/// element or a non-internal `href`.
pub(super) fn internal_link_href(el: &StyledElement) -> Option<String> {
    let Tag::Html(name) = &el.tag else {
        return None;
    };
    if name != "a" {
        return None;
    }
    let target = attr_value(&el.attrs, "href")?.strip_prefix('#')?;
    (!target.is_empty()).then(|| target.to_string())
}

/// The value of the named attribute in `attrs`, if present.
fn attr_value<'a>(attrs: &'a [Attr], name: &str) -> Option<&'a str> {
    attrs
        .iter()
        .find(|a| a.name == name)
        .map(|a| a.value.as_str())
}
