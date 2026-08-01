use tokio::signal::unix::{SignalKind, signal};
use tokio_util::sync::CancellationToken;

/// Handles SIGINT/SIGTERM for graceful shutdown.
pub struct ShutdownHandler {
    token: CancellationToken,
}

impl Default for ShutdownHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl ShutdownHandler {
    pub fn new() -> Self {
        Self { token: CancellationToken::new() }
    }

    /// Get a clone of the cancellation token.
    pub fn token(&self) -> CancellationToken {
        self.token.clone()
    }

    /// Wait for a shutdown signal.
    pub async fn wait_for_signal(&self) {
        let sigterm_res = signal(SignalKind::terminate());

        match sigterm_res {
            Ok(mut sigterm) => {
                tokio::select! {
                    _ = tokio::signal::ctrl_c() => {
                        self.token.cancel();
                    }
                    _ = sigterm.recv() => {
                        self.token.cancel();
                    }
                    _ = self.token.cancelled() => {}
                }
            }
            Err(_) => {
                tokio::select! {
                    _ = tokio::signal::ctrl_c() => {
                        self.token.cancel();
                    }
                    _ = self.token.cancelled() => {}
                }
            }
        }
    }

    /// Check if shutdown was requested.
    pub fn is_shutdown(&self) -> bool {
        self.token.is_cancelled()
    }
}
