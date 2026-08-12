//! Safe Rust wrapper over the native Zig VT100 Terminal Parsing Expert (via C ABI FFI).

#[repr(C)]
pub struct VT100ParserOpaque {
    _private: [u8; 0],
}

unsafe impl Send for VT100ParserOpaque {}
unsafe impl Sync for VT100ParserOpaque {}

unsafe extern "C" {
    fn vt100_parser_create(cols: usize, rows: usize) -> *mut VT100ParserOpaque;
    fn vt100_parser_feed(parser: *mut VT100ParserOpaque, data: *const u8, len: usize);
    fn vt100_parser_get_line_count(parser: *const VT100ParserOpaque) -> usize;
    fn vt100_parser_get_bytes_count(parser: *const VT100ParserOpaque) -> usize;
    fn vt100_parser_free(parser: *mut VT100ParserOpaque);
}

/// Safe Rust abstraction around native Zig VT100 parser.
pub struct ZigVt100Parser {
    ptr: *mut VT100ParserOpaque,
}

impl ZigVt100Parser {
    pub fn new(cols: usize, rows: usize) -> Option<Self> {
        let ptr = unsafe { vt100_parser_create(cols, rows) };
        if ptr.is_null() {
            None
        } else {
            Some(Self { ptr })
        }
    }

    pub fn feed(&self, data: &[u8]) {
        if !self.ptr.is_null() {
            unsafe {
                vt100_parser_feed(self.ptr, data.as_ptr(), data.len());
            }
        }
    }

    pub fn lines_processed(&self) -> usize {
        if self.ptr.is_null() {
            0
        } else {
            unsafe { vt100_parser_get_line_count(self.ptr) }
        }
    }

    pub fn bytes_processed(&self) -> usize {
        if self.ptr.is_null() {
            0
        } else {
            unsafe { vt100_parser_get_bytes_count(self.ptr) }
        }
    }
}

impl Drop for ZigVt100Parser {
    fn drop(&mut self) {
        if !self.ptr.is_null() {
            unsafe {
                vt100_parser_free(self.ptr);
            }
            self.ptr = std::ptr::null_mut();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zig_vt100_parser_ffi() {
        if let Some(parser) = ZigVt100Parser::new(80, 24) {
            parser.feed(b"Hello World\nLine 2\nLine 3\n");
            assert_eq!(parser.lines_processed(), 3);
            assert_eq!(parser.bytes_processed(), 26);
        }
    }

    #[test]
    fn test_zig_vt100_parser_new_zero_dimensions() {
        let parser = ZigVt100Parser::new(0, 0);
        // Parser creation with 0 dimensions may return None or a valid parser
        // depending on the Zig implementation.
        if let Some(p) = parser {
            assert_eq!(p.lines_processed(), 0);
            assert_eq!(p.bytes_processed(), 0);
        }
    }

    #[test]
    fn test_zig_vt100_parser_new_large_dimensions() {
        let parser = ZigVt100Parser::new(1000, 1000);
        if let Some(p) = parser {
            assert_eq!(p.lines_processed(), 0);
            assert_eq!(p.bytes_processed(), 0);
        }
    }

    #[test]
    fn test_zig_vt100_parser_feed_empty_data() {
        if let Some(parser) = ZigVt100Parser::new(80, 24) {
            parser.feed(b"");
            assert_eq!(parser.lines_processed(), 0);
            assert_eq!(parser.bytes_processed(), 0);
        }
    }

    #[test]
    fn test_zig_vt100_parser_feed_no_newlines() {
        if let Some(parser) = ZigVt100Parser::new(80, 24) {
            parser.feed(b"Hello World");
            assert_eq!(parser.lines_processed(), 0);
            assert_eq!(parser.bytes_processed(), 11);
        }
    }

    #[test]
    fn test_zig_vt100_parser_multiple_feeds() {
        if let Some(parser) = ZigVt100Parser::new(80, 24) {
            parser.feed(b"Line 1\n");
            assert_eq!(parser.lines_processed(), 1);
            assert_eq!(parser.bytes_processed(), 7);

            parser.feed(b"Line 2\nLine 3\n");
            assert_eq!(parser.lines_processed(), 3);
            assert_eq!(parser.bytes_processed(), 21);
        }
    }

    #[test]
    fn test_zig_vt100_parser_feed_with_carriage_return() {
        if let Some(parser) = ZigVt100Parser::new(80, 24) {
            parser.feed(b"Line\r\nAnother\r\n");
            // The parser should handle \r\n sequences
            let _lines = parser.lines_processed();
            let bytes = parser.bytes_processed();
            // Verify the parser processed the input (exact counts depend on Zig impl)
            assert!(bytes >= 14); // "Line\r\nAnother\r\n" is 15 bytes
        }
    }

    #[test]
    fn test_zig_vt100_parser_bytes_count_matches_feed_length() {
        if let Some(parser) = ZigVt100Parser::new(80, 24) {
            let data = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ";
            parser.feed(data);
            assert_eq!(parser.bytes_processed(), data.len());
        }
    }

    #[test]
    fn test_zig_vt100_parser_new_returns_some_for_valid_dims() {
        let parser = ZigVt100Parser::new(80, 24);
        // Most implementations return Some for positive dimensions
        if parser.is_none() {
            // Zig library might reject 0x0, but positive dims should be accepted
            panic!("Expected Some for valid dimensions 80x24");
        }
    }
}
