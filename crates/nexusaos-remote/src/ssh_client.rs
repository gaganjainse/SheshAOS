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
}
