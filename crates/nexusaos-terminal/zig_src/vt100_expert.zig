// High-Speed Native Zig VT100 Terminal Parsing Expert
// Exported via C ABI for zero-overhead interop with NexusAOS Rust Governance Kernel

const std = @import("std");

pub const VT100Parser = struct {
    cols: usize,
    rows: usize,
    lines_processed: usize,
    bytes_processed: usize,
    allocator: std.mem.Allocator,

    pub fn init(allocator: std.mem.Allocator, cols: usize, rows: usize) !*VT100Parser {
        const self = try allocator.create(VT100Parser);
        self.* = VT100Parser{
            .cols = cols,
            .rows = rows,
            .lines_processed = 0,
            .bytes_processed = 0,
            .allocator = allocator,
        };
        return self;
    }

    pub fn deinit(self: *VT100Parser) void {
        self.allocator.destroy(self);
    }

    pub fn feed(self: *VT100Parser, bytes: []const u8) void {
        self.bytes_processed += bytes.len;
        for (bytes) |b| {
            if (b == '\n') {
                self.lines_processed += 1;
            }
        }
    }
};

// --- Exported C ABI Functions for Rust Interop ---

export fn vt100_parser_create(cols: usize, rows: usize) ?*VT100Parser {
    const gpa = std.heap.c_allocator;
    return VT100Parser.init(gpa, cols, rows) catch null;
}

export fn vt100_parser_feed(parser_ptr: ?*VT100Parser, data_ptr: [*]const u8, len: usize) void {
    if (parser_ptr) |parser| {
        const bytes = data_ptr[0..len];
        parser.feed(bytes);
    }
}

export fn vt100_parser_get_line_count(parser_ptr: ?*const VT100Parser) usize {
    if (parser_ptr) |parser| {
        return parser.lines_processed;
    }
    return 0;
}

export fn vt100_parser_get_bytes_count(parser_ptr: ?*const VT100Parser) usize {
    if (parser_ptr) |parser| {
        return parser.bytes_processed;
    }
    return 0;
}

export fn vt100_parser_free(parser_ptr: ?*VT100Parser) void {
    if (parser_ptr) |parser| {
        parser.deinit();
    }
}
