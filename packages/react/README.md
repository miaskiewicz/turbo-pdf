# turbo-html2pdf-react

Author [**turbo-html2pdf**](https://www.npmjs.com/package/turbo-html2pdf) document
templates as **React components**. The components render **once, at authoring time**
(via `renderToStaticMarkup`) into a template *source string* — they are never on the
PDF render hot path. Attribute values are **expression strings** resolved later in
the Rust engine, not evaluated JavaScript.

```tsx
import { compileTemplate, If, Each } from 'turbo-html2pdf-react'
import { compile } from 'turbo-html2pdf'   // the engine (Node) — or turbo-html2pdf-wasm in the browser

function Invoice() {
  return (
    <>
      <h1>Invoice {'{{ data.number }}'}</h1>
      <Each of="data.rows" as="row">
        <p>{'{{ row.description }}'} — {'{{ row.amount | currency }}'}</p>
      </Each>
      <If cond="data.paid"><div class="stamp">PAID</div></If>
    </>
  )
}

const source = compileTemplate(<Invoice />)   // -> HTML + Jinja + t: directives (a string)
const program = compile(source)               // hand it to the engine
const { pdf } = program.render({ data: { number: 42, rows: [], paid: true } })
```

Components map to the paged-media DSL: `<If>`/`<ElseIf>`/`<Else>`, `<Switch>`/`<Case>`,
`<Each>`, `<Include>`, `<RunningHeader>`/`<RunningFooter>`, `<Footnote>`, `<Page/>` /
`<Pages/>`, plus `<Raw>` for literal markup.

This package only produces the template string; rendering is done by
**turbo-html2pdf** (Node) or **turbo-html2pdf-wasm** (browser). See the
[main README](https://github.com/miaskiewicz/turbo-html2pdf) and the
[DSL / API docs](https://github.com/miaskiewicz/turbo-html2pdf/tree/main/docs).

MIT.
