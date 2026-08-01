//! Real-time SSE token stream renderer.

use std::io::{self, Write};

pub struct TokenStreamer;

impl TokenStreamer {
    /// Print incoming streaming token chunk in real time.
    pub fn push_token(token: &str) {
        print!("{}", token);
        let _ = io::stdout().flush();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_push_token() {
        TokenStreamer::push_token("");
        assert!(true);
    }
}
