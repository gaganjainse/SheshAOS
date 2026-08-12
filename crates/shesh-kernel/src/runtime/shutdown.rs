use tokio::signal::unix::{signal, SignalKind};
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

    /// Cancel the shutdown token (for testing).
    pub fn cancel(&self) {
        self.token.cancel();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_creates_handler() {
        let handler = ShutdownHandler::new();
        assert!(!handler.is_shutdown());
    }

    #[test]
    fn test_default_creates_handler() {
        let handler = ShutdownHandler::default();
        assert!(!handler.is_shutdown());
    }

    #[test]
    fn test_token_is_cloneable() {
        let handler = ShutdownHandler::new();
        let token1 = handler.token();
        let token2 = handler.token();
        assert!(!token1.is_cancelled());
        assert!(!token2.is_cancelled());
    }

    #[test]
    fn test_is_shutdown_initially_false() {
        let handler = ShutdownHandler::new();
        assert!(!handler.is_shutdown());
    }

    #[test]
    fn test_is_shutdown_after_cancel() {
        let handler = ShutdownHandler::new();
        handler.cancel();
        assert!(handler.is_shutdown());
    }

    #[test]
    fn test_token_reflects_cancel() {
        let handler = ShutdownHandler::new();
        let token = handler.token();
        assert!(!token.is_cancelled());
        handler.cancel();
        assert!(token.is_cancelled());
    }

    #[test]
    fn test_multiple_handlers_independent() {
        let h1 = ShutdownHandler::new();
        let h2 = ShutdownHandler::new();
        h1.cancel();
        assert!(h1.is_shutdown());
        assert!(!h2.is_shutdown());
    }
}
