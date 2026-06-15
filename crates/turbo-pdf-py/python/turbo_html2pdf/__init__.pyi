"""Type stubs for the turbo_html2pdf PyO3 extension."""

from typing import Any

class TurboPdfError(Exception):
    code: str
    span: dict[str, int]
    message: str

class Fonts:
    @staticmethod
    def load(fonts: list[bytes]) -> "Fonts": ...

class Program:
    def render(
        self,
        data: Any = ...,
        css: str = ...,
        fonts: "Fonts | None" = ...,
        images: list[bytes] | None = ...,
        meta: dict[str, Any] | None = ...,
        now: int | None = ...,
    ) -> bytes: ...
    def render_full(
        self,
        data: Any = ...,
        css: str = ...,
        fonts: "Fonts | None" = ...,
        images: list[bytes] | None = ...,
        meta: dict[str, Any] | None = ...,
        now: int | None = ...,
    ) -> tuple[bytes, list[dict[str, Any]], int]: ...
    def has_header(self) -> bool: ...
    def has_footer(self) -> bool: ...

def compile(template_html: str, opts: Any = ...) -> Program: ...
def render(
    template_html: str,
    data: Any = ...,
    css: str = ...,
    fonts: "Fonts | None" = ...,
    images: list[bytes] | None = ...,
    meta: dict[str, Any] | None = ...,
    now: int | None = ...,
) -> bytes: ...
