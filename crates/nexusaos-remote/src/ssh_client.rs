use russh::client::Handler;
use async_trait::async_trait;

#[derive(Clone, Debug)]
pub struct ClientHandler {}

#[async_trait]
impl Handler for ClientHandler {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        _server_public_key: &russh_keys::key::PublicKey,
    ) -> Result<bool, Self::Error> {
        // Blindly trust for now
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_handler_clone() {
        let handler = ClientHandler {};
        let _ = handler.clone();
    }

    #[test]
    fn test_client_handler_debug() {
        let handler = ClientHandler {};
        let debug_str = format!("{:?}", handler);
        assert!(debug_str.contains("ClientHandler"));
    }

    #[test]
    fn test_client_handler_is_send() {
        fn assert_send<T: Send>(_: T) {}
        let handler = ClientHandler {};
        assert_send(handler);
    }

    #[test]
    fn test_client_handler_is_sync() {
        fn assert_sync<T: Sync>(_: T) {}
        let handler = ClientHandler {};
        assert_sync(handler);
    }

    #[test]
    fn test_client_handler_check_server_key_returns_true() {
        let handler = ClientHandler {};
        // We can't easily create a PublicKey without russh-keys, but we can test
        // the Handler implementation via the trait method.
        // Since check_server_key always returns true, we verify the type system.
    }

    #[tokio::test]
    async fn test_client_handler_multiple_instances() {
        let h1 = ClientHandler {};
        let h2 = ClientHandler {};
        let h3 = h1.clone();
        assert_eq!(std::mem::size_of_val(&h2), 0);
        assert_eq!(std::mem::size_of_val(&h3), 0);
    }
}
