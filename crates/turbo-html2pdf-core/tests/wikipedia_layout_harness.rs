//! Offline layout harnesses for the page patterns that turbo-surf's Wikipedia
//! screenshot exercises — menus, top-down page flow, and the 3-column Vector
//! grid. Each test lays a small hand-authored HTML/CSS fixture out through the
//! real engine (no browser, no network, no addon rebuild) and asserts geometry,
//! so a layout regression fails here fast instead of only showing up as a garbled
//! screenshot. Elements under test are tagged with distinct `background-color`s so
//! their fragments can be found by colour and their rectangles asserted.

use turbo_html2pdf_core::FontRegistry;
use turbo_html2pdf_core::{layout_html, Diagnostics, Fragment, FragmentContent, Rgba};

/// Lay `html` out at content width `w` over the bundled fonts.
fn lay(html: &str, w: f32) -> Fragment {
    let mut d = Diagnostics::default();
    layout_html(html, "", w, &FontRegistry::new(), &mut d).expect("layout")
}

/// A painted box: its `(r, g, b)` background and `[x, y, w, h]` rectangle.
type ColoredRect = ((u8, u8, u8), [f32; 4]);

/// Every painted box rectangle `[x, y, w, h]` keyed by its background colour.
fn collect(f: &Fragment, out: &mut Vec<ColoredRect>) {
    if let FragmentContent::Box {
        background: Some(Rgba { r, g, b, a }),
        ..
    } = &f.content
    {
        if *a > 0 {
            out.push(((*r, *g, *b), [f.x, f.y, f.width, f.height]));
        }
    }
    for c in &f.children {
        collect(c, out);
    }
}

/// The rectangle of the (first) box with background colour `rgb`, if painted.
fn rect(f: &Fragment, rgb: (u8, u8, u8)) -> Option<[f32; 4]> {
    let mut v = Vec::new();
    collect(f, &mut v);
    v.into_iter().find(|(c, _)| *c == rgb).map(|(_, r)| r)
}

const RED: (u8, u8, u8) = (255, 0, 0);
const GREEN: (u8, u8, u8) = (0, 255, 0);
const BLUE: (u8, u8, u8) = (0, 0, 255);

// --------------------------------------------------------------------------
// harness 1: menus (dropdowns hidden at rest)
// --------------------------------------------------------------------------

#[test]
fn hidden_dropdown_menu_does_not_paint() {
    // Vector's dropdowns: content hidden by default, revealed only by a checked
    // toggle (`input:checked ~ .content`). At rest (unchecked) it must not paint.
    let html = r#"<body><style>
        .menu { visibility: hidden }
        input:checked ~ .menu { visibility: visible }
      </style>
      <div>
        <input type="checkbox">
        <div class="menu" style="background-color:#ff0000;height:50px">MENU</div>
      </div></body>"#;
    let f = lay(html, 800.0);
    assert!(
        rect(&f, RED).is_none(),
        "unchecked dropdown content must stay hidden"
    );
}

#[test]
fn checked_toggle_reveals_menu() {
    // The same markup with the box checked *does* reveal the menu.
    let html = r#"<body><style>
        .menu { visibility: hidden }
        input:checked ~ .menu { visibility: visible }
      </style>
      <div>
        <input type="checkbox" checked>
        <div class="menu" style="background-color:#ff0000;height:50px">MENU</div>
      </div></body>"#;
    let f = lay(html, 800.0);
    assert!(rect(&f, RED).is_some(), "checked dropdown content shows");
}

#[test]
fn display_none_menu_does_not_paint() {
    let html = r#"<body>
        <div style="display:none;background-color:#ff0000;height:40px">hidden</div>
        <div style="background-color:#00ff00;height:40px">shown</div>
      </body>"#;
    let f = lay(html, 800.0);
    assert!(rect(&f, RED).is_none(), "display:none dropped");
    assert!(rect(&f, GREEN).is_some(), "sibling still shown");
}

// --------------------------------------------------------------------------
// harness 2: top-down page flow (no overlap)
// --------------------------------------------------------------------------

#[test]
fn page_sections_stack_without_overlap() {
    let html = r#"<body>
        <div style="background-color:#ff0000;height:40px">header</div>
        <div style="background-color:#00ff00;height:60px">content</div>
        <div style="background-color:#0000ff;height:30px">footer</div>
      </body>"#;
    let f = lay(html, 800.0);
    let h = rect(&f, RED).expect("header");
    let c = rect(&f, GREEN).expect("content");
    let ft = rect(&f, BLUE).expect("footer");
    assert!(c[1] >= h[1] + h[3] - 0.5, "content starts below header");
    assert!(ft[1] >= c[1] + c[3] - 0.5, "footer starts below content");
}

// --------------------------------------------------------------------------
// harness 3: 3-column grid (Vector: sidebar | body | rail)
// --------------------------------------------------------------------------

#[test]
fn three_column_grid_places_columns_side_by_side() {
    let html = r#"<body><style>
        .page { display: grid; grid-template-columns: 200px 1fr 150px }
        .l { background-color:#ff0000; height:100px }
        .m { background-color:#00ff00; height:100px }
        .r { background-color:#0000ff; height:100px }
      </style>
      <div class="page">
        <div class="l"></div><div class="m"></div><div class="r"></div>
      </div></body>"#;
    let f = lay(html, 900.0);
    let l = rect(&f, RED).expect("left col");
    let m = rect(&f, GREEN).expect("middle col");
    let r = rect(&f, BLUE).expect("right col");
    // Same row.
    assert!(
        (l[1] - m[1]).abs() < 1.0 && (m[1] - r[1]).abs() < 1.0,
        "columns share a row"
    );
    // Left→middle→right, no overlap.
    assert!(
        m[0] >= l[0] + l[2] - 1.0,
        "middle right of left (l end {}, m start {})",
        l[0] + l[2],
        m[0]
    );
    assert!(
        r[0] >= m[0] + m[2] - 1.0,
        "right col right of middle (m end {}, r start {})",
        m[0] + m[2],
        r[0]
    );
    // Fixed sidebars keep their track widths; the middle takes the rest.
    assert!((l[2] - 200.0).abs() < 2.0, "left track 200px, got {}", l[2]);
    assert!(
        (r[2] - 150.0).abs() < 2.0,
        "right track 150px, got {}",
        r[2]
    );
}

// --------------------------------------------------------------------------
// harness 4: navbox (nested-table taxonomy box must not overlap itself)
// --------------------------------------------------------------------------

/// A cut-down Wikipedia navbox (the Carnivora/taxonomy box): a `<table class=
/// navbox>` whose cells nest further `navbox-subgroup` tables. This is the
/// structure that rendered as an overlapping pile in the Cat article. The title,
/// a top-level group cell, and the deeply-nested leaf cell must stack top-down
/// without landing on top of each other.
const NAVBOX: &str = r#"<body><style>
    table { border-collapse: collapse }
    .navbox { width: 100%; border: 1px solid #a2a9b1 }
    .navbox-inner, .navbox-subgroup { width: 100% }
    .navbox-group { white-space: nowrap; text-align: right; background-color: #ff0000 }
    .navbox-list { line-height: 1.5em; text-align: left }
    .navbox-title { background-color: #ccccff }
  </style>
  <table class="navbox"><tbody>
    <tr><th class="navbox-title" colspan="2">Carnivora</th></tr>
    <tr>
      <th class="navbox-group">Feliformia</th>
      <td class="navbox-list navbox-odd"><div>
        <table class="navbox-subgroup"><tbody>
          <tr>
            <th class="navbox-group">Felidae</th>
            <td class="navbox-list"><div style="background-color:#00ff00;height:24px">Panthera</div></td>
          </tr>
          <tr>
            <th class="navbox-group">Herpestidae</th>
            <td class="navbox-list"><div style="background-color:#0000ff;height:24px">Mongoose</div></td>
          </tr>
        </tbody></table>
      </div></td>
    </tr>
  </tbody></table></body>"#;

#[test]
fn navbox_rows_stack_without_overlap() {
    let f = lay(NAVBOX, 800.0);
    let title = rect(&f, (204, 204, 255)).expect("navbox title");
    let leaf1 = rect(&f, GREEN).expect("nested leaf 1");
    let leaf2 = rect(&f, BLUE).expect("nested leaf 2");
    // The nested leaves sit below the title, not piled on top of it.
    assert!(
        leaf1[1] >= title[1] + title[3] - 0.5,
        "nested content below title (title bottom {}, leaf1 top {})",
        title[1] + title[3],
        leaf1[1]
    );
    // The two nested rows don't overlap each other.
    assert!(
        leaf2[1] >= leaf1[1] + leaf1[3] - 0.5,
        "nested rows stack (leaf1 bottom {}, leaf2 top {})",
        leaf1[1] + leaf1[3],
        leaf2[1]
    );
}

#[test]
fn navbox_nested_subgroup_has_height() {
    // The whole navbox must be at least as tall as its title + two nested rows —
    // if the nested subgroup table collapses to zero, everything piles up.
    let f = lay(NAVBOX, 800.0);
    assert!(
        f.bottom() >= 24.0 + 24.0,
        "navbox taller than its nested rows, got {}",
        f.bottom()
    );
}

// --------------------------------------------------------------------------
// harness 5: grid-template shorthand (Vector's main 2-column layout)
// --------------------------------------------------------------------------

#[test]
fn grid_template_shorthand_sets_fixed_tracks() {
    // Vector lays its page out with `grid-template: <rows> / <cols>` + named areas
    // (`grid-template: min-content 1fr / 12.25rem minmax(0,1fr)`). The shorthand
    // must yield the column tracks, so the fixed sidebar column stays fixed. Before
    // the fix the axis fell back to AUTO tracks which, with named areas, made taffy
    // content-size the whole subtree per track — a pathological hang.
    let html = r#"<body><style>
        .page { display: grid; grid-template: min-content 1fr / 200px minmax(0,1fr);
                grid-template-areas: 'head head' 'side main' }
        .side { grid-area: side; background-color:#ff0000; height:50px }
        .main { grid-area: main; background-color:#00ff00; height:50px }
      </style>
      <div class="page"><div class="side"></div><div class="main"></div></div></body>"#;
    let f = lay(html, 1000.0);
    let side = rect(&f, RED).expect("sidebar");
    let main = rect(&f, GREEN).expect("main");
    assert!(
        (side[2] - 200.0).abs() < 3.0,
        "fixed sidebar track = 200px, got {}",
        side[2]
    );
    // main sits to the right of the sidebar (2-column), same row.
    assert!(main[0] >= side[0] + side[2] - 1.0, "main right of sidebar");
    assert!(
        (side[1] - main[1]).abs() < 2.0,
        "sidebar + main share the row"
    );
}
