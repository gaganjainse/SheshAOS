//! NexusAOS Snapshot Serializer (Zig)
//! JSON serialization/deserialization for projection snapshots.

const std = @import("std");
const Allocator = std.mem.Allocator;

pub const Snapshot = extern struct {
    snapshot_id: [:0]const u8,
    created_at: i64, // unix millis
    last_sequence: u64,
    data: [:0]const u8, // JSON string
};

pub const SnapshotStore = struct {
    const Self = @This();

    path: [:0]const u8,
    allocator: Allocator,

    pub fn init(allocator: Allocator, path: [:0]const u8) !Self {
        return Self{
            .path = path,
            .allocator = allocator,
        };
    }

    /// Save a snapshot to the filesystem.
    pub fn save(self: *Self, snapshot: *const Snapshot) !void {
        const dir = std.fs.path.dirname(self.path) orelse ".";
        try std.fs.cwd().makePath(dir);

        const filename = try std.fmt.allocPrint(self.allocator, "snapshot_{d}.json", .{snapshot.created_at});
        defer self.allocator.free(filename);

        const file_path = try std.fs.path.join(self.allocator, &[_][:0]const u8{ dir, filename });
        defer self.allocator.free(file_path);

        const json = try self.serialize(snapshot);
        defer self.allocator.free(json);

        const file = try std.fs.cwd().createFile(file_path, .{});
        defer file.close();
        try file.writeAll(json);
    }

    /// Load the most recent snapshot by timestamp in the filename.
    pub fn loadLatest(self: *Self) !?Snapshot {
        const dir = std.fs.path.dirname(self.path) orelse ".";
        var dir_iter = try std.fs.cwd().openDir(dir, .{});
        defer dir_iter.close();

        var latest_path: ?[:0]const u8 = null;
        var latest_ts: i64 = 0;

        var iter = dir_iter.iterate();
        while (try iter.next()) |entry| {
            const name = entry.name;
            if (!std.mem.startsWith(u8, name, "snapshot_") or !std.mem.endsWith(u8, name, ".json")) continue;

            const ts_part = name["snapshot_".len .. name.len - ".json".len];
            const ts = std.fmt.parseInt(i64, ts_part, 10) catch continue;
            if (ts >= latest_ts) {
                latest_ts = ts;
                latest_path = try std.fs.path.join(self.allocator, &[_][:0]const u8{ dir, name });
            }
        }

        if (latest_path) |path| {
            defer self.allocator.free(path);
            const content = try std.fs.cwd().readFileAlloc(self.allocator, path, 10 * 1024 * 1024);
            defer self.allocator.free(content);
            return try self.deserialize(content);
        }

        return null;
    }

    fn serialize(self: *Self, snapshot: *const Snapshot) ![:0]const u8 {
        // Build JSON manually for speed
        var buf = ArrayList(u8).init(self.allocator);
        const writer = buf.writer();

        try writer.print("{{\"snapshot_id\":\"{s}\",\"created_at\":{d},\"last_sequence\":{d},\"data\":{s}}}", .{
            snapshot.snapshot_id,
            snapshot.created_at,
            snapshot.last_sequence,
            snapshot.data,
        });

        return try buf.toOwnedSliceSentinel(0);
    }

    fn deserialize(self: *Self, json: [:0]const u8) !Snapshot {
        // Simple JSON field extraction
        var result = Snapshot{
            .snapshot_id = "",
            .created_at = 0,
            .last_sequence = 0,
            .data = "",
        };

        // Extract snapshot_id
        if (extractString(json, "snapshot_id")) |id| result.snapshot_id = id;
        // Extract created_at
        if (extractInt(json, "created_at")) |ts| result.created_at = ts;
        // Extract last_sequence
        if (extractInt(json, "last_sequence")) |seq| result.last_sequence = seq;
        // Extract data
        if (extractString(json, "data")) |data| result.data = data;

        return result;
    }

    fn extractString(json: [:0]const u8, field: [:0]const u8) ?[:0]const u8 {
        const key = try std.fmt.allocPrint(std.heap.page_allocator, "\"{s}\":\"", .{field});
        defer std.heap.page_allocator.free(key);

        const start = std.mem.indexOf(u8, json, key) orelse return null;
        const content_start = start + key.len;
        const end = std.mem.indexOf(u8, json[content_start..], "\"") orelse return null;
        return json[content_start .. content_start + end];
    }

    fn extractInt(json: [:0]const u8, field: [:0]const u8) ?u64 {
        const key = try std.fmt.allocPrint(std.heap.page_allocator, "\"{s}\":", .{field});
        defer std.heap.page_allocator.free(key);

        const start = std.mem.indexOf(u8, json, key) orelse return null;
        const num_start = start + key.len;
        const num_end = std.mem.indexOfAny(u8, json[num_start..], " \t\n\r,}") orelse return null;
        const num_str = json[num_start .. num_start + num_end];
        return std.fmt.parseInt(u64, num_str, 10);
    }
};

// ── C ABI ──────────────────────────────────────────────────

export fn snapshot_store_save(store: *SnapshotStore, snapshot: *const Snapshot) bool {
    store.save(snapshot) catch return false;
    return true;
}

export fn snapshot_store_load_latest(store: *SnapshotStore) ?Snapshot {
    return store.loadLatest() catch null;
}
