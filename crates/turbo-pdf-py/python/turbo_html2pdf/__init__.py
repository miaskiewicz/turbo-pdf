"""turbo-html2pdf — native HTML/CSS-to-PDF engine (PyO3 binding).

Compile a template once into a :class:`Program`, then render it against data as
many times as needed; or use the one-shot :func:`render` to do both at once.

    import turbo_html2pdf as t

    prog = t.compile("<p>Hello {{ data.name }}</p>")
    pdf = prog.render(data={"name": "world"})   # -> bytes, starts with b"%PDF"

Warm the fonts once and reuse the handle across renders:

    fonts = t.Fonts.load([open(p, "rb").read() for p in font_paths])
    pdf = prog.render(data={...}, css="body{font-family:font0}", fonts=fonts)

Fatal compile/render faults raise :class:`TurboPdfError` (with ``.code`` and
``.span``). Non-fatal lints are returned by :meth:`Program.render_full`.
"""

from ._turbo_html2pdf import (  # noqa: F401
    Fonts,
    Program,
    TurboPdfError,
    compile,
    render,
)

__all__ = ["Fonts", "Program", "TurboPdfError", "compile", "render"]
