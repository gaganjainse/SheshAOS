//! Real-time SSE token stream renderer.

use std::io::{self, Write};

pub struct TokenStreamer;

impl TokenStreamer {
    /// Print incoming streaming token chunk in real time.
    pub fn push_token(token: &str) {
        let mut out = io::stdout().lock();
        let _ = Self::write_token(&mut out, token);
    }

    /// Write one chunk to any sink + flush. The testable core of the streamer.
    pub fn write_token<W: Write>(writer: &mut W, token: &str) -> io::Result<()> {
        writer.write_all(token.as_bytes())?;
        writer.flush()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_write_token_emits_exact_bytes() {
        let mut buf = Vec::new();
        TokenStreamer::write_token(&mut buf, "hello world").unwrap();
        assert_eq!(buf, b"hello world");
    }

    #[test]
    fn test_write_token_empty_is_noop_but_flushes() {
        let mut buf = Vec::new();
        TokenStreamer::write_token(&mut buf, "").unwrap();
        assert!(buf.is_empty());
    }

    #[test]
    fn test_write_token_unicode_roundtrip() {
        let mut buf = Vec::new();
        TokenStreamer::write_token(&mut buf, "ñø∂ — शेष").unwrap();
        assert_eq!(String::from_utf8(buf).unwrap(), "ñø∂ — शेष");
    }

    #[test]
    fn test_write_token_chunks_concatenate_in_order() {
        let mut buf = Vec::new();
        for chunk in ["the ", "quick ", "brown", " fox"] {
            TokenStreamer::write_token(&mut buf, chunk).unwrap();
        }
        assert_eq!(String::from_utf8(buf).unwrap(), "the quick brown fox");
    }
}
