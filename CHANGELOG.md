# Changelog

All notable changes to turbo-html2pdf are documented here. Format follows
[Keep a Changelog](https://keepachangelog.com/); versions follow SemVer. The npm,
PyPI, and crates.io packages release in lockstep from a `v*` tag (PyPI on `pyv*`).

## [0.2.4]

### Added
- **Public Jinja-free HTML→Fragment drive** in `turbo-html2pdf-core`: `parse_html`,
  `collect_style_css`, and `layout_html(html, extra_css, width, fonts, diags)` —
  lay a raw/final HTML string out into a positioned `Fragment` galley **without**
  the minijinja templating pass. For callers that already hold final HTML (e.g. a
  hydrated DOM snapshot) where `{{ }}`/`{% %}` are page content, not template
  syntax. Lets external consumers (e.g. turbo-surf's synthetic screenshots) reuse
  the native layout + font engine and paint the `Fragment` display list
  themselves. No PDF/emit/pagination/template-render code is touched; the default
  build and its byte output are unchanged.

### CI
- aarch64-linux release builds run on native `ubuntu-24.04-arm` runners (napi,
  wasm/svg, and the mcp binary) instead of cross-compiling.

## [0.2.3]

### Changed
- Renamed the core crate `turbo-pdf-core` → **`turbo-html2pdf-core`**.

## [0.2.2]

### Added
- Publish `turbo-html2pdf-core` + `turbo-html2pdf-mcp` to crates.io.

## [0.2.1]

### Added
- Publish the `turbo-html2pdf-mcp` server binary as per-platform archives on each
  GitHub Release.

## [0.2.0]

### Added
- **`turbo-html2pdf-mcp`** — a native MCP server (stdio JSON-RPC 2.0) exposing
  `render` / `append_pdf` / `check_template` to agents.
