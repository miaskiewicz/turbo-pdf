# turbo-html2pdf-template

Author [**turbo-html2pdf**](https://www.npmjs.com/package/turbo-html2pdf) document
templates with **plain functions** — no React, no JSX, no build step. The helpers
produce a template *source string* (HTML + Jinja + `t:` directives) you hand to the
engine.

```ts
import { each, ifBlock, expr } from 'turbo-html2pdf-template'
import { compile } from 'turbo-html2pdf'   // the engine (Node) — or turbo-html2pdf-wasm in the browser

const source = [
  `<h1>Invoice ${expr('data.number')}</h1>`,
  each('data.rows', 'row',
    `<p>${expr('row.description')} — ${expr('row.amount | currency')}</p>`),
  ifBlock('data.paid', `<div class="stamp">PAID</div>`),
].join('')

const program = compile(source)
const { pdf } = program.render({ data: { number: 42, rows: [], paid: true } })
```

Helpers mirror the DSL: `ifBlock`/`elseIf`/`elseBlock`, `switchBlock`/`caseBlock`/
`defaultBlock`, `each`, `include`, `expr`, `runningHeader`/`runningFooter`, and the
paged-media directives.

This package only produces the template string; rendering is done by
**turbo-html2pdf** (Node) or **turbo-html2pdf-wasm** (browser). See the
[main README](https://github.com/miaskiewicz/turbo-html2pdf) and the
[DSL / API docs](https://github.com/miaskiewicz/turbo-html2pdf/tree/main/docs).

MIT.
