//! Error marshaling across the PyO3 boundary.
//!
//! Core fatal errors ([`CompileError`]/[`RenderError`]) carry a machine-readable
//! [`ErrorCode`] and a source [`Span`] (line/col/byte-offset). They surface to
//! Python as a typed `TurboPdfError` exception whose `.code` (the stable variant
//! string) and `.span` (a `dict` of `line`/`col`/`byte_offset`) mirror the core
//! payload. Non-fatal lints are *returned* in the result, never raised.

use pyo3::create_exception;
use pyo3::exceptions::PyException;
use pyo3::prelude::*;
use pyo3::types::PyDict;

use turbo_pdf_core::{CompileError, ErrorCode, RenderError, Span};

create_exception!(
    turbo_html2pdf,
    TurboPdfError,
    PyException,
    "Fatal compile/render fault. Carries `.code` (stable string) and `.span`."
);

/// The stable string form of an [`ErrorCode`] (mirrors the variant name).
fn code_str(code: ErrorCode) -> &'static str {
    match code {
        ErrorCode::TemplateSyntax => "TemplateSyntax",
        ErrorCode::UnknownFilter => "UnknownFilter",
        ErrorCode::UndefinedValue => "UndefinedValue",
        ErrorCode::IncludeDepthExceeded => "IncludeDepthExceeded",
        ErrorCode::UnknownElement => "UnknownElement",
        ErrorCode::Render => "Render",
    }
}

/// Build the `{line, col, byte_offset}` span dict attached to the exception.
fn span_dict<'py>(py: Python<'py>, span: Span) -> PyResult<Bound<'py, PyDict>> {
    let d = PyDict::new(py);
    d.set_item("line", span.line)?;
    d.set_item("col", span.col)?;
    d.set_item("byte_offset", span.byte_offset)?;
    Ok(d)
}

/// Construct a `TurboPdfError` instance with `.code`/`.span`/`.message` set.
fn build(py: Python<'_>, code: ErrorCode, message: &str, span: Span) -> PyErr {
    match build_inner(py, code, message, span) {
        Ok(err) => err,
        // If attribute population ever fails (e.g. interpreter shutdown), still
        // raise a typed error so callers always see `TurboPdfError`.
        Err(e) => e,
    }
}

/// The fallible body of [`build`]: instantiate the exception and attach fields.
fn build_inner(py: Python<'_>, code: ErrorCode, message: &str, span: Span) -> PyResult<PyErr> {
    let err = TurboPdfError::new_err(message.to_string());
    let value = err.value(py);
    value.setattr("code", code_str(code))?;
    value.setattr("span", span_dict(py, span)?)?;
    value.setattr("message", message)?;
    Ok(err)
}

/// Map a fatal compile error to a typed `TurboPdfError`.
pub fn from_compile(py: Python<'_>, e: CompileError) -> PyErr {
    build(py, e.code, &e.message, e.span)
}

/// Map a fatal render error to a typed `TurboPdfError`.
pub fn from_render(py: Python<'_>, e: RenderError) -> PyErr {
    build(py, e.code, &e.message, e.span)
}
