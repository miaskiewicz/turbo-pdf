//! PDF/UA role derivation from the semantic HTML tag (`pdf-ua` feature,
//! AC-11.1). Only compiled under the `pdf-ua` feature, so this lives in its own
//! module (kept out of the default coverage surface, like the other gated-only
//! modules).

use crate::layout::fragment::UaRole;
use crate::node::Tag;
use crate::style::StyledElement;

/// The structure role for a styled element's HTML tag, or `None` for tags
/// that are transparent to tagging (their content attaches to an ancestor).
pub(super) fn role_of(el: &StyledElement) -> Option<UaRole> {
    let Tag::Html(name) = &el.tag else {
        return None;
    };
    role_for_tag(name)
}

/// Map a lowercase HTML tag name to its structure role.
fn role_for_tag(name: &str) -> Option<UaRole> {
    if let Some(level) = heading_level(name) {
        return Some(UaRole::Heading(level));
    }
    Some(match name {
        "p" => UaRole::Paragraph,
        "ul" | "ol" => UaRole::List,
        "li" => UaRole::ListItem,
        "table" => UaRole::Table,
        "tr" => UaRole::TableRow,
        "th" => UaRole::TableHeader,
        "td" => UaRole::TableData,
        "img" => UaRole::Figure,
        "span" | "a" | "em" | "strong" | "b" | "i" => UaRole::Span,
        "div" | "section" | "article" | "header" | "footer" | "main" | "nav" | "aside" => {
            UaRole::Group
        }
        _ => return None,
    })
}

/// The 1-based heading level of `h1`..`h6`, or `None`.
fn heading_level(name: &str) -> Option<u8> {
    let level = name.strip_prefix('h')?.parse::<u8>().ok()?;
    (1..=6).contains(&level).then_some(level)
}

/// The `alt` text of an `<img>` (empty allowed: an empty `/Alt` is valid and
/// marks the figure as decorative-with-no-description for readers).
pub(super) fn alt_of(el: &StyledElement) -> Option<String> {
    match &el.tag {
        Tag::Html(name) if name == "img" => Some(alt_attr(el).unwrap_or("").to_string()),
        _ => None,
    }
}

/// The `alt` attribute value of a styled element, if present.
fn alt_attr(el: &StyledElement) -> Option<&str> {
    el.attrs
        .iter()
        .find(|a| a.name == "alt")
        .map(|a| a.value.as_str())
}
