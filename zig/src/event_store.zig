//! NexusAOS Event Store — JSONL parser and writer (Zig)
//! Exposes C ABI for Rust FFI interop.
//!
//! The event store is an append-only JSONL file with an in-memory index
//! mapping EventId -> byte offset for fast lookup, and a monotonically
//! increasing sequence number.

const std = @import("std");
const Allocator = std.mem.Allocator;
const ArrayList = std.ArrayList;
const HashMap = std.HashMap;

pub const EventId = [16]u8; // UUIDv7 as bytes

pub const SequenceNumber = u64;

pub const EventKind = enum(u8) {
    TaskCreated,
    TaskClassified,
    TaskStateChanged,
    ModelRequested,
    ModelResponded,
    ModelFailed,
    ToolRequested,
    ToolCompleted,
    ToolFailed,
    PolicyChecked,
    PolicyDenied,
    ConfirmationRequested,
    ConfirmationGranted,
    ConfirmationDenied,
    CheckpointCreated,
    SnapshotCreated,
    SystemStarted,
    SystemShutdown,
    Error,
};

pub const EventPayload = extern struct {
    task_id: ?EventId = null,
    kind: EventKind = .TaskCreated,
    sequence: SequenceNumber = 0,
    timestamp: i64 = 0, // unix millis
    source: [:0]const u8 = "",
    content: [:0]const u8 = "",
};

pub const EventStore = struct {
    const Self = @This();

    path: [:0]const u8,
    index: HashMap[EventId]u64,
    next_sequence: SequenceNumber = 1,
    writer: ?std.fs.File = null,
    allocator: Allocator,

    pub fn init(allocator: Allocator, path: [:0]const u8) !Self {
        var index = HashMap[EventId]u64.init(allocator);
        errdefer index.deinit();

        const file = try std.fs.cwd().openFile(path, .{ .mode = .read_only });
        defer file.close();

        var buf_reader = std.io.bufferedReader(file.reader());
        const reader = buf_reader.reader();

        var line_buf = ArrayList(u8).init(allocator);
        defer line_buf.deinit();

        var offset: u64 = 0;
        var line = std.ArrayList(u8).init(allocator);
        defer line.deinit();

        while (try reader.readUntilDelimiterOrEofAlloc(allocator, '\n', 1024 * 1024)) |line_bytes| {
            const trimmed = std.mem.trimRight(u8, line_bytes, "\r\n");
            if (trimmed.len == 0) continue;

            // Parse the JSON line to extract event_id and sequence
            // For now, we just track the offset — full parsing happens on read
            const event_id = try extractEventId(allocator, trimmed);
            const seq = try extractSequence(allocator, trimmed);

            try index.put(event_id, offset);
            if (seq >= Self.next_sequence) {
                Self.next_sequence = seq + 1;
            }

            offset += trimmed.len + 1; // +1 for newline
            line.clearRetainingCapacity();
        }

        // Reopen for appending
        const writer_file = try std.fs.cwd().openFile(path, .{
            .mode = .write_only,
            .options = .{ .append = true },
        });

        return Self{
            .path = path,
            .index = index,
            .next_sequence = Self.next_sequence,
            .writer = writer_file,
            .allocator = allocator,
        };
    }

    pub fn deinit(self: *Self) void {
        if (self.writer) |f| f.close();
        self.index.deinit();
    }

    /// Append a JSON event line to the store.
    pub fn append(self: *Self, event_json: [:0]const u8) !void {
        const file = self.writer orelse return error.StoreNotOpen;
        try file.writeAll(event_json);
        try file.writeAll("\n");
        try file.sync();

        const offset = try file.getPos();
        const event_id = try extractEventId(self.allocator, event_json);
        try self.index.put(event_id, offset);
        self.next_sequence += 1;
    }

    /// Read all events from the store.
    pub fn readAll(self: *Self, allocator: Allocator) !ArrayList([]const u8) {
        const file = try std.fs.cwd().openFile(self.path, .{ .mode = .read_only });
        defer file.close();

        var events = ArrayList([]const u8).init(allocator);
        var buf = ArrayList(u8).init(allocator);
        defer buf.deinit();

        var reader = file.reader();
        while (true) {
            const byte = reader.readByte() catch |err| {
                if (err == error.EndOfStream) break;
                return err;
            };
            if (byte == '\n') {
                try events.append(try buf.toOwnedSlice());
                buf.clearRetainingCapacity();
            } else {
                try buf.append(byte);
            }
        }
        // Don't forget the last line if it doesn't end with newline
        if (buf.items.len > 0) {
            try events.append(try buf.toOwnedSlice());
        }

        return events;
    }

    /// Read events for a specific task by event_id prefix.
    pub fn readForTask(self: *Self, allocator: Allocator, task_id: *const EventId) !ArrayList([]const u8) {
        const all = try self.readAll(allocator);
        var result = ArrayList([]const u8).init(allocator);
        errdefer {
            for (all.items) |item| allocator.free(item);
            all.deinit();
        }

        for (all.items) |line| {
            if (containsEventId(line, task_id)) {
                try result.append(line);
            }
        }

        for (all.items) |item| allocator.free(item);
        all.deinit();

        return result;
    }

    /// Get total event count.
    pub fn count(self: *Self) u64 {
        return self.index.count();
    }

    /// Get the next sequence number and increment.
    pub fn nextSeq(self: *Self) SequenceNumber {
        const seq = self.next_sequence;
        self.next_sequence += 1;
        return seq;
    }

    // ── Internal helpers ──────────────────────────────────

    fn extractEventId(allocator: Allocator, json_line: [:0]const u8) !EventId {
        // Parse "id":"<uuid>" from the JSON line
        const id_key = "\"id\":\"";
        const start = std.mem.indexOf(u8, json_line, id_key) orelse return error.MissingEventId;
        const content_start = start + id_key.len;
        const end = std.mem.indexOf(u8, json_line[content_start..], "\"") orelse return error.MissingEventId;
        const uuid_str = json_line[content_start .. content_start + end];

        var event_id: EventId = undefined;
        _ = try std.fmt.hexToBytes(&event_id, uuid_str);
        return event_id;
    }

    fn extractSequence(allocator: Allocator, json_line: [:0]const u8) !SequenceNumber {
        const seq_key = "\"sequence\":";
        const start = std.mem.indexOf(u8, json_line, seq_key) orelse return 0;
        const num_start = start + seq_key.len;
        const num_end = std.mem.indexOfAny(u8, json_line[num_start..], " \t\n\r,}") orelse return 0;
        const num_str = json_line[num_start .. num_start + num_end];
        return try std.fmt.parseInt(SequenceNumber, num_str, 10);
    }

    fn containsEventId(json_line: [:0]const u8, task_id: *const EventId) bool {
        const hex = std.fmt.bytesToHex(allocator, task_id[0..]);
        defer allocator.free(hex);
        return std.mem.indexOf(u8, json_line, hex) != null;
    }
};

// ── C ABI ──────────────────────────────────────────────────

export fn event_store_open(path: [*:0]const u8) ?*EventStore {
    const allocator = std.heap.c_allocator;
    const store = EventStore.init(allocator, path) catch return null;
    return allocator.create(EventStore) catch null;
}

export fn event_store_append(store: *EventStore, json_line: [*:0]const u8) bool {
    store.append(json_line) catch return false;
    return true;
}

export fn event_store_count(store: *EventStore) u64 {
    return store.count();
}

export fn event_store_next_seq(store: *EventStore) SequenceNumber {
    return store.nextSeq();
}

export fn event_store_deinit(store: *EventStore) void {
    store.deinit();
    std.heap.c_allocator.destroy(store);
}
