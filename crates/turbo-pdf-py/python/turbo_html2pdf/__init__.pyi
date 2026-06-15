"""Type stubs for the turbo_html2pdf PyO3 extension."""

from typing import Any, TypedDict

class TurboPdfError(Exception):
    code: str
    span: dict[str, int]
    message: str

class Encryption(TypedDict, total=False):
    """AES-256 password-encryption settings (the ``encryption=`` kwarg).

    ``user_password`` is required; ``owner_password`` defaults to it. The
    permission flags default to all-granted; set one ``False`` to clear it for a
    user-password open. Encrypted output is intentionally non-deterministic.
    """

    user_password: str
    owner_password: str
    print: bool
    modify: bool
    copy: bool
    annotate: bool
    fill_forms: bool
    accessibility: bool
    assemble: bool
    high_quality_print: bool

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
        pdf_a: bool = ...,
        pdf_ua: bool = ...,
        lang: str | None = ...,
        cmyk: bool = ...,
        encryption: Encryption | None = ...,
        append_pdfs: list[bytes] | None = ...,
    ) -> bytes: ...
    def render_full(
        self,
        data: Any = ...,
        css: str = ...,
        fonts: "Fonts | None" = ...,
        images: list[bytes] | None = ...,
        meta: dict[str, Any] | None = ...,
        now: int | None = ...,
        pdf_a: bool = ...,
        pdf_ua: bool = ...,
        lang: str | None = ...,
        cmyk: bool = ...,
        encryption: Encryption | None = ...,
        append_pdfs: list[bytes] | None = ...,
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
    pdf_a: bool = ...,
    pdf_ua: bool = ...,
    lang: str | None = ...,
    cmyk: bool = ...,
    encryption: Encryption | None = ...,
    append_pdfs: list[bytes] | None = ...,
) -> bytes: ...
def append_pdf(base: bytes, extras: list[bytes]) -> bytes: ...
