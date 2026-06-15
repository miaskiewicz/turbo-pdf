//! Build script for the PyO3 binding.
//!
//! The crate is an abi3 `extension-module` cdylib: it must NOT link a
//! `libpython` — the host interpreter resolves the `Py*` symbols at import time.
//! maturin already passes the right linker flags when it builds the wheel, but a
//! plain host `cargo build` / `clippy` / `cc-check` (run from the repo root, so a
//! crate-local `.cargo/config.toml` would be ignored) also links the cdylib and
//! would otherwise fail with "undefined symbol _Py…". Emitting the flag here,
//! scoped to this crate via `cargo:rustc-cdylib-link-arg`, fixes that everywhere.
//!
//! Only macOS needs it: its linker rejects undefined symbols in a `.dylib` by
//! default, whereas ELF shared objects on Linux/BSD allow them, and Windows
//! resolves against the import lib maturin supplies.

fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") {
        // Applies only to the cdylib link step, leaving the rlib untouched.
        println!("cargo:rustc-cdylib-link-arg=-undefined");
        println!("cargo:rustc-cdylib-link-arg=dynamic_lookup");
    }
}
