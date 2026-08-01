#ifndef NEXUSAOS_SNAPSHOT_H
#define NEXUSAOS_SNAPSHOT_H

#include <stddef.h>
#include <stdint.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct Snapshot Snapshot;
typedef struct SnapshotStore SnapshotStore;

// Create a snapshot store at the given directory path.
SnapshotStore* snapshot_store_init(const char* path);

// Save a snapshot to the filesystem.
// Returns true on success, false on failure.
bool snapshot_store_save(SnapshotStore* store, const Snapshot* snapshot);

// Load the most recent snapshot by timestamp in the filename.
// Returns NULL if no snapshots exist.
const Snapshot* snapshot_store_load_latest(SnapshotStore* store);

// Destroy and free the snapshot store.
void snapshot_store_deinit(SnapshotStore* store);

#ifdef __cplusplus
}
#endif

#endif // NEXUSAOS_SNAPSHOT_H