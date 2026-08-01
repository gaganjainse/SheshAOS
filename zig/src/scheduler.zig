//! NexusAOS Priority Scheduler (Zig)
//! Binary heap-based priority queue with O(log n) enqueue/dequeue.

const std = @import("std");
const Allocator = std.mem.Allocator;

pub const Priority = enum(u8) {
    Low = 0,
    Normal = 1,
    High = 2,
    Critical = 3,
};

pub const SchedulerEntry = struct {
    task_id: [16]u8, // UUID bytes
    priority: Priority,
    enqueued_at: i64, // unix millis
};

pub const Scheduler = struct {
    const Self = @This();

    entries: std.ArrayList(SchedulerEntry),
    max_depth: usize,
    allocator: Allocator,

    pub fn init(allocator: Allocator, max_depth: usize) Self {
        return Self{
            .entries = std.ArrayList(SchedulerEntry).init(allocator),
            .max_depth = max_depth,
            .allocator = allocator,
        };
    }

    pub fn deinit(self: *Self) void {
        self.entries.deinit();
    }

    /// Enqueue a task. Returns error if queue is full.
    pub fn enqueue(self: *Self, task_id: [16]u8, priority: Priority) !void {
        if (self.entries.items.len >= self.max_depth) return error.QueueFull;

        const now = std.time.milliTimestamp();
        try self.entries.append(.{
            .task_id = task_id,
            .priority = priority,
            .enqueued_at = now,
        });

        // Sift up
        var idx = self.entries.items.len - 1;
        while (idx > 0) {
            const parent = (idx - 1) / 2;
            if (self.compare(idx, parent)) break;
            std.mem.swap(SchedulerEntry, &self.entries.items[idx], &self.entries.items[parent]);
            idx = parent;
        }
    }

    /// Dequeue the highest-priority task.
    pub fn dequeue(self: *Self) ?SchedulerEntry {
        if (self.entries.items.len == 0) return null;

        const result = self.entries.items[0];
        const last = self.entries.pop();

        if (self.entries.items.len > 0) {
            self.entries.items[0] = last;
            self.siftDown(0);
        }

        return result;
    }

    /// Get current queue depth.
    pub fn queueDepth(self: *const Self) usize {
        return self.entries.items.len;
    }

    /// Cancel a task by task_id.
    pub fn cancel(self: *Self, task_id: [16]u8) bool {
        for (self.entries.items, 0..) |entry, i| {
            if (std.mem.eql(u8, &entry.task_id, &task_id)) {
                _ = self.entries.swapRemove(i);
                if (i < self.entries.items.len) {
                    self.siftDown(i);
                    // Also try sifting up
                    var idx = i;
                    while (idx > 0) {
                        const parent = (idx - 1) / 2;
                        if (self.compare(idx, parent)) break;
                        std.mem.swap(SchedulerEntry, &self.entries.items[idx], &self.entries.items[parent]);
                        idx = parent;
                    }
                }
                return true;
            }
        }
        return false;
    }

    fn siftDown(self: *Self, idx: usize) void {
        var i = idx;
        while (true) {
            var largest = i;
            const left = 2 * i + 1;
            const right = 2 * i + 2;

            if (left < self.entries.items.len and self.compare(left, largest)) {
                largest = left;
            }
            if (right < self.entries.items.len and self.compare(right, largest)) {
                largest = right;
            }

            if (largest == i) break;
            std.mem.swap(SchedulerEntry, &self.entries.items[i], &self.entries.items[largest]);
            i = largest;
        }
    }

    /// Returns true if a has higher priority than b.
    fn compare(self: *const Self, a: usize, b: usize) bool {
        const ea = self.entries.items[a];
        const eb = self.entries.items[b];
        if (@intFromEnum(ea.priority) != @intFromEnum(eb.priority)) {
            return @intFromEnum(ea.priority) > @intFromEnum(eb.priority);
        }
        return ea.enqueued_at < eb.enqueued_at; // older first
    }
};

// ── C ABI ──────────────────────────────────────────

export fn scheduler_create(max_depth: usize) ?*Scheduler {
    const allocator = std.heap.c_allocator;
    const s = allocator.create(Scheduler) catch return null;
    s.* = Scheduler.init(allocator, max_depth);
    return s;
}

export fn scheduler_enqueue(s: *Scheduler, task_id: *const [16]u8, priority: u8) bool {
    s.enqueue(task_id.*, @enumFromInt(priority)) catch return false;
    return true;
}

export fn scheduler_dequeue(s: *Scheduler) ?SchedulerEntry {
    return s.dequeue();
}

export fn scheduler_queue_depth(s: *const Scheduler) usize {
    return s.queueDepth();
}

export fn scheduler_cancel(s: *Scheduler, task_id: *const [16]u8) bool {
    return s.cancel(task_id.*);
}

export fn scheduler_deinit(s: *Scheduler) void {
    s.deinit();
    std.heap.c_allocator.destroy(s);
}