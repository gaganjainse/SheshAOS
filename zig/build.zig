const std = @import("std");

pub fn build(b: *std.Build) void {
    const target = b.standardTargetOptions(.{});
    const optimize = b.standardOptimizeOption(.{});

    // ── sheshaaos-event-store (JSONL parser) ──────────────────────
    const event_store = b.addStaticLibrary(.{
        .name = "sheshaaos_event_store",
        .root_source_file = b.path("src/event_store.zig"),
        .target = target,
        .optimize = optimize,
    });
    event_store.link_libc();
    b.installArtifact(event_store);

    // ── sheshaaos-snapshot (JSON snapshot serializer) ─────────────
    const snapshot = b.addStaticLibrary(.{
        .name = "sheshaaos_snapshot",
        .root_source_file = b.path("src/snapshot.zig"),
        .target = target,
        .optimize = optimize,
    });
    snapshot.link_libc();
    b.installArtifact(snapshot);

    // ── sheshaaos-scheduler (priority queue) ──────────────────────
    const scheduler = b.addStaticLibrary(.{
        .name = "sheshaaos_scheduler",
        .root_source_file = b.path("src/scheduler.zig"),
        .target = target,
        .optimize = optimize,
    });
    scheduler.link_libc();
    b.installArtifact(scheduler);

    // ── sheshaaos-terminal (VT100 parser — already exists) ────────
    const vt100 = b.addStaticLibrary(.{
        .name = "vt100_expert",
        .root_source_file = b.path("crates/sheshaaos-terminal/zig_src/vt100_expert.zig"),
        .target = target,
        .optimize = optimize,
    });
    vt100.link_libc();
    b.installArtifact(vt100);
}
