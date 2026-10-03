//! Model checking with loom (`--cfg loom` and the `loom` feature).
//!
//! A model runs the same atomics production code uses, under every interleaving loom explores.

use loom::model::Builder;
pub use loom::sync::Arc;
pub use loom::thread;
use tracing::subscriber::with_default;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::fmt::Subscriber;

/// Runs `f` under every interleaving loom explores within a preemption bound.
///
/// The bound is 3 unless `LOOM_MAX_PREEMPTIONS` sets another; no setting removes it. `LOOM_LOG`,
/// a `tracing` filter such as `trace`, prints each execution's trace with the test's output.
pub fn check<F: Fn() + Sync + Send + 'static>(f: F) {
    let mut builder = Builder::new();
    if builder.preemption_bound.is_none() {
        builder.preemption_bound = Some(3);
    }
    let subscriber = Subscriber::builder()
        .with_env_filter(EnvFilter::from_env("LOOM_LOG"))
        .with_test_writer()
        .without_time()
        .finish();
    with_default(subscriber, || builder.check(f));
}
