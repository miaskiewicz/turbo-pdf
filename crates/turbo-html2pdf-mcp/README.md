# turbo-html2pdf-mcp

A native **MCP** (Model Context Protocol) server for HTML/CSS + Jinja → PDF —
hand-rolled JSON-RPC 2.0 over stdio, no SDK — exposing the
[turbo-html2pdf](https://github.com/miaskiewicz/turbo-html2pdf) engine as tools
an agent can call. It drives the same `turbo-html2pdf-core` pipeline as the npm
(N-API) and PyPI (PyO3) bindings, with the full capability surface (PDF/A,
PDF/UA, CMYK, AES-256 encryption, watermarks, named images, foreign-PDF append,
bundled fonts). Only `svg` is omitted, matching the standard `turbo-html2pdf`
npm package.

## Build & run

```sh
cargo build -p turbo-html2pdf-mcp --release   # binary: target/release/turbo-html2pdf-mcp
```

Register it like any stdio MCP server, e.g. with Claude Code:

```sh
claude mcp add turbo-html2pdf -- /absolute/path/to/target/release/turbo-html2pdf-mcp
```

Claude Desktop — add to `claude_desktop_config.json` (macOS:
`~/Library/Application Support/Claude/`, Windows: `%APPDATA%\Claude\`), then
restart Claude:

```jsonc
{
  "mcpServers": {
    "turbo-html2pdf": { "command": "/absolute/path/to/target/release/turbo-html2pdf-mcp" }
  }
}
```

## Tools

| tool | does |
|---|---|
| `render` | a template + `data`/`css`/`fonts`/`images`/`meta`/`watermark`/conformance/`encryption`/`appendPdfs` → `.pdf`. Returns `{ base64 \| path, bytes, pageCount, diagnostics }`. |
| `append_pdf` | glue foreign PDF documents after a `base` PDF, page by page |
| `check_template` | compile a template without rendering; report `{ ok, hasHeader, hasFooter }` (a syntax fault is a tool error) |

Binary I/O is **path-or-base64**: every binary input (`fonts`, `images`,
`appendPdfs`, the append `base`) takes `path` **or** `dataBase64`; `render` /
`append_pdf` take an optional `out` path (→ `{ path, bytes }`) and otherwise
return `{ base64, bytes }`. Bundled fonts mean a document renders with **no**
caller-supplied faces.

```jsonc
// stdin (newline-delimited JSON-RPC)
{"jsonrpc":"2.0","id":1,"method":"initialize"}
{"jsonrpc":"2.0","id":2,"method":"tools/call",
 "params":{"name":"render","arguments":{
   "templateHtml":"<h1>{{ title }}</h1>",
   "data":{"title":"Hello"},
   "css":"@page { size: A4; margin: 1in; }",
   "out":"hello.pdf"}}}
```

The server is a thin layer over `turbo-html2pdf-core`; all the real work — and the
coverage gate — lives in the core. See the
[repo](https://github.com/miaskiewicz/turbo-html2pdf) for the full option surface.

## License

MIT
