//! turbo-html2pdf MCP server — the stdio pump.
//!
//! Reads newline-delimited JSON-RPC requests from stdin, dispatches each through
//! [`turbo_pdf_mcp::handle`], and writes one-line responses to stdout. Blank
//! lines and unparseable lines are skipped; notifications (no `id`) get no reply.
//! Synchronous and single-threaded by design — the tools are CPU/file work, not
//! async I/O, so there is no runtime to pull in.

#![forbid(unsafe_code)]

use std::io::{BufRead, Write};

use turbo_html2pdf_mcp::{handle, Session};

fn main() {
    let mut session = Session::new();
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();
    for line in stdin.lock().lines() {
        if pump(&mut session, line, &mut stdout).is_none() {
            break;
        }
    }
}

/// Handle one input line. Returns `None` only when stdin is closed/errored (to
/// stop the loop); `Some(())` to keep going, including for skipped lines.
fn pump(session: &mut Session, line: std::io::Result<String>, out: &mut impl Write) -> Option<()> {
    let line = line.ok()?;
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return Some(());
    }
    if let Ok(req) = serde_json::from_str::<serde_json::Value>(trimmed) {
        respond(session, &req, out);
    }
    Some(())
}

/// Dispatch a parsed request and write its response line (if any).
fn respond(session: &mut Session, req: &serde_json::Value, out: &mut impl Write) {
    if let Some(resp) = handle(session, req) {
        let _ = writeln!(out, "{resp}");
        let _ = out.flush();
    }
}
