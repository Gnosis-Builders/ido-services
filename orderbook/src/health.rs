use std::sync::atomic::{AtomicBool, Ordering};

/// Trait for asynchronously notifying health information
pub trait HealthReporting: Send + Sync {
    /// Notify that the service is ready. Can be called multiple times.
    /// We use this to signal readiness only at the start of a batch in order to not interrupt the
    /// still running kubernetes pod while it is handling a batch.
    fn notify_ready(&self);
}

/// Implementation sharing health information over an HTTP endpoint.
#[derive(Debug, Default)]
pub struct HttpHealthEndpoint {
    ready: AtomicBool,
}

impl HttpHealthEndpoint {
    /// Creates a new HTTP health enpoint.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns true if the service is ready, false otherwise.
    pub fn is_ready(&self) -> bool {
        self.ready.load(Ordering::SeqCst)
    }
}

impl HealthReporting for HttpHealthEndpoint {
    fn notify_ready(&self) {
        self.ready.store(true, Ordering::SeqCst);
    }
}
