//! Optional hot-path instrumentation (Phase 13, §13). Behind the `perf` feature
//! so production builds pay nothing: with the feature off the [`hot!`] macro
//! expands to nothing and this module's body is not compiled.
//!
//! When enabled, [`hot!`] bumps a named counter; a bench or test reads
//! [`snapshot`] to assert how often a path ran — e.g. that `font.shape.parse`
//! fires once per *distinct* run rather than once per occurrence (proving the
//! `FontFace` shape cache holds). Use [`reset`] between measured sections.
//!
//! Run the perf tests with `cargo test -p turbo-pdf-core --features perf`.

/// Increment the named hot-path counter. A no-op (zero machine code) unless the
/// `perf` feature is enabled.
#[macro_export]
macro_rules! hot {
    ($label:expr) => {{
        #[cfg(feature = "perf")]
        $crate::perf::bump($label);
    }};
}

#[cfg(feature = "perf")]
mod inner {
    use std::cell::RefCell;
    use std::collections::BTreeMap;

    // Per-thread so counters are isolated across parallel tests and so a single
    // render (which runs on one thread) reads exactly its own hot-path counts.
    thread_local! {
        static COUNTS: RefCell<BTreeMap<&'static str, u64>> = const { RefCell::new(BTreeMap::new()) };
    }

    fn with_counts<R>(f: impl FnOnce(&mut BTreeMap<&'static str, u64>) -> R) -> R {
        COUNTS.with(|c| f(&mut c.borrow_mut()))
    }

    /// Increment the counter for `label`.
    pub fn bump(label: &'static str) {
        with_counts(|c| *c.entry(label).or_insert(0) += 1);
    }

    /// How many times `label` has fired since the last [`reset`].
    pub fn count(label: &'static str) -> u64 {
        with_counts(|c| c.get(label).copied().unwrap_or(0))
    }

    /// A copy of all counters.
    pub fn snapshot() -> BTreeMap<&'static str, u64> {
        with_counts(|c| c.clone())
    }

    /// Clear all counters.
    pub fn reset() {
        with_counts(|c| c.clear());
    }
}

#[cfg(feature = "perf")]
pub use inner::{bump, count, reset, snapshot};
