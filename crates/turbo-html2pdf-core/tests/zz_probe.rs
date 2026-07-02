use std::collections::BTreeMap;
use turbo_html2pdf_core::{layout_html, Diagnostics, FontRegistry, Fragment, FragmentContent};
fn texts(f: &Fragment, out: &mut Vec<(f32, f32)>) {
    if let FragmentContent::TextLine { .. } = &f.content {
        out.push((f.x, f.y));
    }
    for c in &f.children {
        texts(c, out);
    }
}
fn pile(html: &str, css: &str, label: &str) {
    let mut d = Diagnostics::default();
    let f = layout_html(html, css, 1280.0, &FontRegistry::new(), &mut d).unwrap();
    let mut t = vec![];
    texts(&f, &mut t);
    let mut m: BTreeMap<(i32, i32), usize> = BTreeMap::new();
    for (x, y) in t.iter().filter(|(_, y)| *y < 500.0) {
        *m.entry((*x as i32, *y as i32)).or_default() += 1;
    }
    println!(
        "{label}: max pile={}",
        m.values().max().copied().unwrap_or(0)
    );
}
#[test]
#[ignore]
fn probe() {
    let dir="/private/tmp/claude-501/-Users-grzegorzmiaskiewicz-github-flux-turbo-surf/e9e90c9c-503d-4d81-86a8-2bbbed67bf8c/scratchpad";
    let html = std::fs::read_to_string(format!("{dir}/wiki-hydrated.html")).unwrap();
    let css = std::fs::read_to_string(format!("{dir}/wiki-full.css")).unwrap();
    pile(&html, &css, "baseline");
    // hide common visually-hidden classes
    pile(&html,&format!("{css}\n.sr-only,.visually-hidden,.mw-jump-link,[class*=visually-hidden]{{display:none!important}}"),"no-sr-only");
    // neuter the clip/1px pattern by finding elements with those
    pile(
        &html,
        &format!("{css}\n.vector-toc-landmark,.mw-editsection,.noprint{{display:none!important}}"),
        "no-misc",
    );
}
